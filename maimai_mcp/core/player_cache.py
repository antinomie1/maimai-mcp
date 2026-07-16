"""Local player score snapshots.

Written when:
  - personal b50 / minfo succeeds (side-effect), or
  - group rank queries fetch missing/stale scores on demand.

Fresh entries are reused so repeat group-rank calls do not re-hit the prober.
"""

from __future__ import annotations

import json
import os
import threading
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

# Default 24h. Override with PLAYER_CACHE_TTL_SECONDS (0 = always reuse if present).
_DEFAULT_TTL_SECONDS = 24 * 3600


def cache_dir() -> Path:
    configured = os.environ.get("PLAYER_CACHE_DIR")
    if configured and configured.strip():
        return Path(configured).expanduser().resolve()
    return (Path.cwd() / "player-cache").resolve()


def ttl_seconds() -> int:
    raw = os.environ.get("PLAYER_CACHE_TTL_SECONDS")
    if raw is None or not str(raw).strip():
        return _DEFAULT_TTL_SECONDS
    try:
        return max(0, int(raw))
    except ValueError:
        return _DEFAULT_TTL_SECONDS


def _qq_path(qq: str | int) -> Path:
    return cache_dir() / "by_qq" / f"{qq}.json"


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _norm_qq(qq: Any) -> str | None:
    if isinstance(qq, int):
        if qq <= 0:
            return None
        return str(qq)
    if isinstance(qq, str):
        text = qq.strip()
        return text or None
    return None


def _parse_iso(value: Any) -> datetime | None:
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parsed = datetime.fromisoformat(value)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed


def is_fresh(fetched_at: Any, *, force: bool = False) -> bool:
    """True if cache entry can be reused without a network call."""
    if force:
        return False
    parsed = _parse_iso(fetched_at)
    if not parsed:
        return False
    ttl = ttl_seconds()
    if ttl <= 0:
        return True
    age = (datetime.now(timezone.utc) - parsed).total_seconds()
    return age <= ttl


def _empty_doc(qq: str) -> dict[str, Any]:
    return {
        "qq": qq,
        "updatedAt": now_iso(),
        "rating": None,
        "songs": {},
    }


def read_player(qq: Any) -> dict[str, Any] | None:
    key = _norm_qq(qq)
    if not key:
        return None
    path = _qq_path(key)
    if not path.exists():
        return None
    try:
        parsed = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None
    return parsed if isinstance(parsed, dict) else None


def _write_player(doc: dict[str, Any]) -> None:
    qq = str(doc.get("qq") or "")
    if not qq:
        return
    path = _qq_path(qq)
    path.parent.mkdir(parents=True, exist_ok=True)
    doc["updatedAt"] = now_iso()
    tmp = path.with_name(f".{path.name}.{os.getpid()}.{threading.get_ident()}.tmp")
    tmp.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    os.replace(tmp, path)


def _load_or_empty(qq: str) -> dict[str, Any]:
    existing = read_player(qq)
    if isinstance(existing, dict) and existing.get("qq"):
        existing.setdefault("songs", {})
        return existing
    return _empty_doc(qq)


def write_rating(
    qq: Any,
    *,
    rating: int | float,
    name: str | None = None,
    source: str | None = None,
    extra: dict[str, Any] | None = None,
) -> None:
    """Record rating from a successful personal B50 query."""
    key = _norm_qq(qq)
    if not key:
        return
    if not isinstance(rating, (int, float)) or rating <= 0:
        return
    try:
        doc = _load_or_empty(key)
        payload: dict[str, Any] = {
            "value": int(rating),
            "name": name,
            "source": source,
            "fetchedAt": now_iso(),
        }
        if extra:
            payload["extra"] = extra
        doc["rating"] = payload
        _write_player(doc)
    except Exception:
        return


def write_song_scores(
    qq: Any,
    *,
    song_id: int,
    song_name: str | None,
    charts: list[dict[str, Any]],
    source: str | None = None,
) -> None:
    """Record charts for one song from a successful personal minfo query."""
    key = _norm_qq(qq)
    if not key or not charts:
        return
    try:
        doc = _load_or_empty(key)
        songs = doc.setdefault("songs", {})
        if not isinstance(songs, dict):
            songs = {}
            doc["songs"] = songs
        songs[str(int(song_id))] = {
            "songId": int(song_id),
            "songName": song_name,
            "source": source,
            "fetchedAt": now_iso(),
            "charts": charts,
        }
        _write_player(doc)
    except Exception:
        return


def get_rating(qq: Any) -> dict[str, Any] | None:
    doc = read_player(qq)
    if not doc:
        return None
    rating = doc.get("rating")
    return rating if isinstance(rating, dict) and rating.get("value") else None


def get_rating_if_fresh(qq: Any, *, force: bool = False) -> dict[str, Any] | None:
    rating = get_rating(qq)
    if not rating:
        return None
    if not is_fresh(rating.get("fetchedAt"), force=force):
        return None
    return rating


def get_song_entry(qq: Any, song_id: int) -> dict[str, Any] | None:
    doc = read_player(qq)
    if not doc:
        return None
    songs = doc.get("songs") if isinstance(doc.get("songs"), dict) else {}
    entry = songs.get(str(int(song_id)))
    return entry if isinstance(entry, dict) else None


def get_song_entry_if_fresh(
    qq: Any, song_id: int, *, force: bool = False
) -> dict[str, Any] | None:
    entry = get_song_entry(qq, song_id)
    if not entry:
        return None
    if not is_fresh(entry.get("fetchedAt"), force=force):
        return None
    return entry


def chart_to_cache_dict(chart: Any) -> dict[str, Any]:
    """Normalize a PlayedResult (or similar) into a cache-friendly dict."""
    if hasattr(chart, "model_dump"):
        raw = chart.model_dump()
    elif isinstance(chart, dict):
        raw = chart
    else:
        raw = {
            k: getattr(chart, k)
            for k in (
                "song_id",
                "song_name",
                "level_index",
                "type",
                "level",
                "level_value",
                "achievements",
                "rating",
                "dx_score",
                "fc",
                "fs",
                "rate",
                "level_label",
            )
            if hasattr(chart, k)
        }

    def _enum_val(v: Any) -> Any:
        if v is None:
            return None
        if hasattr(v, "value"):
            return v.value
        return v

    level_index = raw.get("level_index")
    if hasattr(level_index, "value"):
        level_index = level_index.value
    return {
        "songId": raw.get("song_id"),
        "songName": raw.get("song_name"),
        "levelIndex": level_index,
        "type": raw.get("type"),
        "level": raw.get("level"),
        "ds": raw.get("level_value"),
        "achievements": raw.get("achievements"),
        "rating": raw.get("rating"),
        "dxScore": raw.get("dx_score"),
        "fc": _enum_val(raw.get("fc")),
        "fs": _enum_val(raw.get("fs")),
        "rate": _enum_val(raw.get("rate")),
        "levelLabel": raw.get("level_label"),
    }


def cache_status() -> dict[str, Any]:
    base = cache_dir()
    by_qq = base / "by_qq"
    count = 0
    rating_count = 0
    song_entries = 0
    if by_qq.is_dir():
        for path in by_qq.glob("*.json"):
            if path.name.startswith("."):
                continue
            count += 1
            try:
                doc = json.loads(path.read_text(encoding="utf-8"))
            except Exception:
                continue
            if isinstance(doc, dict):
                if isinstance(doc.get("rating"), dict) and doc["rating"].get("value"):
                    rating_count += 1
                songs = doc.get("songs")
                if isinstance(songs, dict):
                    song_entries += len(songs)
    return {
        "cacheDir": str(base),
        "playerCount": count,
        "ratingCount": rating_count,
        "songEntryCount": song_entries,
    }
