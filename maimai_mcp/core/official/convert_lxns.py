"""Convert official raw GetUserMusicApi data into Lxns score upload payload.

Lxns docs: POST /api/v0/user/maimai/player/scores
  { "scores": [ { id, type, level_index, achievements, fc, fs, dx_score, ... } ] }

Song id rules (official musicId → Lxns):
  - id > 100000: utage, keep id
  - id > 10000: dx, id %= 10000
  - else: standard

When Diving-Fish music_data cache is available, drop scores whose
(id, type, level_index) is not in the catalog — Lxns rejects those with 400.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .convert import (
    COMBO_STATUS_TO_FC,
    SYNC_STATUS_TO_FS,
    default_music_data_path,
    iter_user_music_details,
    load_json,
    normalize_int,
    normalize_number,
    write_json,
)

VALID_FC = frozenset({"fc", "fcp", "ap", "app"})
VALID_FS = frozenset({"sync", "fs", "fsp", "fsd", "fsdp"})


def official_music_id_to_lxns(music_id: int) -> tuple[int, str]:
    if music_id >= 100000:
        return music_id, "utage"
    if music_id > 10000:
        return music_id % 10000, "dx"
    return music_id, "standard"


def build_lxns_chart_index(music_data: Path | None = None) -> dict[tuple[int, str], set[int]]:
    """(song_id, type) → allowed level_index set from Diving-Fish music_data."""
    path = music_data
    if path is None:
        try:
            path = default_music_data_path()
        except Exception:
            return {}
    if not path or not Path(path).is_file():
        return {}
    try:
        data = load_json(Path(path))
    except Exception:
        return {}
    if not isinstance(data, list):
        return {}
    charts: dict[tuple[int, str], set[int]] = {}
    for song in data:
        if not isinstance(song, dict):
            continue
        sid = normalize_int(song.get("id"))
        if sid is None:
            continue
        raw_type = song.get("type")
        stype = str(raw_type or "").upper()
        if sid >= 100000:
            key = (sid, "utage")
        elif stype == "DX" or sid > 10000:
            key = (sid % 10000, "dx")
        else:
            key = (sid, "standard")
        n = 0
        ds = song.get("ds")
        levels = song.get("level")
        if isinstance(ds, list):
            n = len(ds)
        elif isinstance(levels, list):
            n = len(levels)
        if n <= 0:
            n = 5
        charts.setdefault(key, set()).update(range(n))
    return charts


def convert_record_lxns(
    detail: dict[str, Any],
    *,
    chart_index: dict[tuple[int, str], set[int]] | None = None,
) -> tuple[dict[str, Any] | None, dict[str, Any] | None]:
    music_id = normalize_int(detail.get("musicId"))
    level_index = normalize_int(detail.get("level"))
    if music_id is None:
        return None, {"reason": "invalid_music_id", "record": detail}
    if level_index == 10:
        level_index = 0
    if level_index is None or level_index < 0 or level_index > 4:
        return None, {
            "reason": "unsupported_level_index",
            "musicId": music_id,
            "level": detail.get("level"),
            "record": detail,
        }

    song_id, song_type = official_music_id_to_lxns(music_id)

    if chart_index is not None:
        allowed = chart_index.get((song_id, song_type))
        if allowed is None:
            return None, {
                "reason": "chart_not_in_catalog",
                "musicId": music_id,
                "id": song_id,
                "type": song_type,
            }
        if level_index not in allowed:
            return None, {
                "reason": "level_not_in_catalog",
                "musicId": music_id,
                "id": song_id,
                "type": song_type,
                "level_index": level_index,
            }

    achievement_raw = normalize_number(detail.get("achievement"))
    if achievement_raw is None:
        return None, {"reason": "invalid_achievement", "musicId": music_id, "record": detail}
    achievements = round(achievement_raw / 10000.0, 4)
    if achievements <= 0:
        return None, {
            "reason": "zero_achievement",
            "musicId": music_id,
            "achievements": achievements,
        }
    if achievements > 101.0:
        achievements = 101.0

    dx_score = normalize_int(detail.get("deluxscoreMax"))
    if dx_score is None or dx_score < 0:
        dx_score = 0

    combo_status = normalize_int(detail.get("comboStatus"))
    sync_status = normalize_int(detail.get("syncStatus"))
    fc = COMBO_STATUS_TO_FC.get(combo_status if combo_status is not None else 0, "")
    fs = SYNC_STATUS_TO_FS.get(sync_status if sync_status is not None else 0, "")
    if fc and fc not in VALID_FC:
        fc = ""
    if fs and fs not in VALID_FS:
        fs = ""

    score: dict[str, Any] = {
        "id": song_id,
        "type": song_type,
        "level_index": level_index,
        "achievements": achievements,
        "dx_score": dx_score,
    }
    if fc:
        score["fc"] = fc
    if fs:
        score["fs"] = fs
    return score, None


def convert_raw_to_lxns(
    raw: Any,
    *,
    music_data: Path | None = None,
    use_catalog: bool = True,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    chart_index: dict[tuple[int, str], set[int]] | None = None
    if use_catalog:
        chart_index = build_lxns_chart_index(music_data)
        if not chart_index:
            chart_index = None  # catalog unavailable → do not over-filter

    payload: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []
    seen: set[tuple[int, str, int]] = set()
    for detail in iter_user_music_details(raw):
        converted, skip = convert_record_lxns(detail, chart_index=chart_index)
        if skip is not None:
            skipped.append(skip)
            continue
        assert converted is not None
        key = (converted["id"], converted["type"], converted["level_index"])
        if key in seen:
            skipped.append(
                {"reason": "duplicate_payload_key", "key": list(key), "record": detail}
            )
            continue
        seen.add(key)
        payload.append(converted)
    return payload, skipped


def convert_file_lxns(
    input_path: Path,
    *,
    output: Path | None = None,
    report: Path | None = None,
    pretty: bool = False,
    music_data: Path | None = None,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    raw = load_json(input_path)
    payload, skipped = convert_raw_to_lxns(raw, music_data=music_data)
    body = {"scores": payload}
    write_json(output, body, pretty=pretty)
    report_body = {
        "input": str(input_path),
        "output": str(output) if output else None,
        "target": "lxns",
        "converted": len(payload),
        "skipped": len(skipped),
        "skippedRecords": skipped,
    }
    if report is not None:
        write_json(report, report_body, pretty=True)
    return payload, report_body
