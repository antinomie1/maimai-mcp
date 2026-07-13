"""Read-only QQ identity cache from identity_cache.json.

Does not fetch friends/groups — only reads a local cache file when present.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any


def cache_dir() -> Path:
    configured = os.environ.get("QQ_IDENTITY_CACHE_DIR")
    if configured:
        return Path(configured).expanduser().resolve()
    return (Path.cwd() / "qq-identity-cache").resolve()


def cache_path() -> Path:
    return cache_dir() / "identity_cache.json"


def read_cache() -> dict[str, Any]:
    path = cache_path()
    if not path.exists():
        return {"groups": {}, "users": {}}
    try:
        parsed = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return {"groups": {}, "users": {}}
    if not isinstance(parsed, dict):
        return {"groups": {}, "users": {}}
    parsed.setdefault("groups", {})
    parsed.setdefault("users", {})
    return parsed


def _norm_id(value: Any) -> str | None:
    if isinstance(value, int):
        value = str(value)
    if not isinstance(value, str):
        return None
    value = value.strip()
    return value or None


def looks_like_group_id(qq: int | str | None) -> bool:
    """True if id is listed only under cache.groups (not users)."""
    key = _norm_id(qq)
    if not key:
        return False
    cache = read_cache()
    groups = cache.get("groups") if isinstance(cache.get("groups"), dict) else {}
    users = cache.get("users") if isinstance(cache.get("users"), dict) else {}
    return key in groups and key not in users


def get_identity(qq: Any, group_id: Any = None) -> dict[str, Any] | None:
    key = _norm_id(qq)
    if not key:
        return None
    cache = read_cache()
    user = cache.get("users", {}).get(key)
    if not isinstance(user, dict):
        return None
    groups = user.get("groups") if isinstance(user.get("groups"), dict) else {}
    preferred = None
    gid = _norm_id(group_id)
    if gid and isinstance(groups.get(gid), dict):
        preferred = groups[gid]
    group_entries = [v for v in groups.values() if isinstance(v, dict)]
    return {
        "qq": key,
        "qqNickname": user.get("qqNickname"),
        "friendNickname": user.get("friendNickname"),
        "preferredGroup": preferred,
        "groups": sorted(
            group_entries,
            key=lambda i: (str(i.get("groupName") or ""), str(i.get("groupId") or "")),
        ),
        "waterfishNickname": user.get("waterfishNickname"),
        "waterfishUsername": user.get("waterfishUsername"),
        "isFriend": user.get("isFriend") is True,
        "cachePath": str(cache_path()),
    }


def _score_name(value: Any, q: str) -> int:
    if not isinstance(value, str) or not value.strip():
        return 0
    n = value.strip().casefold()
    if n == q:
        return 100
    if q in n:
        return 50
    return 0


def resolve_identities(
    query: Any, *, group_id: Any = None, max_results: int = 10
) -> dict[str, Any]:
    raw = str(query).strip() if query is not None else ""
    if not raw:
        return {"query": raw, "matches": [], "ambiguous": False}
    q = raw.casefold()
    cache = read_cache()
    users = cache.get("users") if isinstance(cache.get("users"), dict) else {}
    matches: list[dict[str, Any]] = []
    for qq_key, user in users.items():
        if not isinstance(user, dict):
            continue
        snap = get_identity(qq_key, group_id)
        if not snap:
            continue
        score = 0
        fields: list[str] = []
        if raw == str(snap.get("qq") or ""):
            score, fields = 200, ["qq"]
        else:
            for name, val in (
                ("qqNickname", snap.get("qqNickname")),
                ("friendNickname", snap.get("friendNickname")),
                ("waterfishNickname", snap.get("waterfishNickname")),
                ("waterfishUsername", snap.get("waterfishUsername")),
            ):
                s = _score_name(val, q)
                if s:
                    score = max(score, s)
                    fields.append(name)
            pref = snap.get("preferredGroup")
            if isinstance(pref, dict):
                for key in ("groupNickname", "card", "nickname"):
                    s = _score_name(pref.get(key), q)
                    if s:
                        score = max(score, s + 10)
                        fields.append(f"preferredGroup.{key}")
            for g in snap.get("groups") or []:
                if not isinstance(g, dict):
                    continue
                for key in ("groupNickname", "card", "nickname"):
                    s = _score_name(g.get(key), q)
                    if s:
                        score = max(score, s)
                        fields.append(f"group.{key}")
        if score <= 0:
            continue
        snap["matchScore"] = score
        snap["matchedFields"] = sorted(set(fields))
        matches.append(snap)
    matches.sort(key=lambda i: (-int(i.get("matchScore") or 0), str(i.get("qq") or "")))
    limited = matches[: max(1, max_results)]
    amb = False
    if len(limited) > 1:
        amb = int(limited[0].get("matchScore") or 0) == int(
            limited[1].get("matchScore") or 0
        )
    return {
        "query": raw,
        "groupId": _norm_id(group_id),
        "matches": limited,
        "ambiguous": amb,
        "cachePath": str(cache_path()),
        "cacheExists": cache_path().exists(),
    }
