#!/usr/bin/env python3
"""Convert official raw user data JSON into Diving-Fish update_records payload.

Input is a full official API export containing GetUserMusicApi. Output is the
JSON list accepted by Diving-Fish /player/update_records:
achievements, dxScore, fc, fs, level_index, title, type.

Song title/type index MUST come from Diving-Fish /music_data cache
({STATIC_PATH}/data/music_data.json), not a third-party song list snapshot.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

COMBO_STATUS_TO_FC = {
    0: "",
    1: "fc",
    2: "fcp",
    3: "ap",
    4: "app",
}

# Official raw exports use 5 for ordinary Sync. 1-4 correspond to the stronger
# Full Sync markers used by Diving-Fish.
SYNC_STATUS_TO_FS = {
    0: "",
    1: "fs",
    2: "fsp",
    3: "fsd",
    4: "fsdp",
    5: "sync",
}


def default_music_data_path() -> Path:
    """Path to Diving-Fish music_data cache written by get_music_list()."""
    from maimai_mcp.resources import music_file

    return music_file


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def normalize_int(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, float) and value.is_integer():
        return int(value)
    if isinstance(value, str):
        try:
            return int(value.strip())
        except ValueError:
            return None
    return None


def normalize_number(value: Any) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value.strip())
        except ValueError:
            return None
    return None


def build_divingfish_index(path: Path) -> dict[int, dict[str, str]]:
    """Build musicId → {title, type} from Diving-Fish /music_data JSON list."""
    data = load_json(path)
    if not isinstance(data, list):
        raise ValueError(f"{path} must contain a JSON list (Diving-Fish music_data)")
    index: dict[int, dict[str, str]] = {}
    for song in data:
        if not isinstance(song, dict):
            continue
        song_id = normalize_int(song.get("id"))
        title = song.get("title")
        song_type = song.get("type")
        if song_id is None or not isinstance(title, str) or not isinstance(song_type, str):
            continue
        normalized_type = song_type.upper()
        if normalized_type in {"SD", "DX"}:
            index[song_id] = {
                "title": title,
                "type": normalized_type,
                "source": "divingfish",
            }
    if not index:
        raise ValueError(f"{path} produced empty music index")
    return index


def build_music_index(*, music_data: Path) -> dict[int, dict[str, str]]:
    # /player/update_records matches title against Diving-Fish music_data.
    return build_divingfish_index(music_data)


def iter_user_music_details(raw: Any) -> list[dict[str, Any]]:
    if not isinstance(raw, dict):
        raise ValueError("raw JSON root must be an object")
    music_api = raw.get("GetUserMusicApi")
    if not isinstance(music_api, dict):
        raise ValueError("raw JSON missing GetUserMusicApi object")
    groups = music_api.get("userMusicList")
    if not isinstance(groups, list):
        raise ValueError("GetUserMusicApi.userMusicList must be a list")
    details: list[dict[str, Any]] = []
    for group in groups:
        if not isinstance(group, dict):
            continue
        items = group.get("userMusicDetailList")
        if isinstance(items, list):
            details.extend(item for item in items if isinstance(item, dict))
    return details


def convert_record(
    detail: dict[str, Any],
    *,
    music_index: dict[int, dict[str, str]],
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

    song = music_index.get(music_id)
    if song is None:
        return None, {"reason": "music_id_not_found", "musicId": music_id, "record": detail}

    achievement_raw = normalize_number(detail.get("achievement"))
    if achievement_raw is None:
        return None, {"reason": "invalid_achievement", "musicId": music_id, "record": detail}
    achievements = achievement_raw / 10000.0

    dx_score = normalize_int(detail.get("deluxscoreMax"))
    if dx_score is None:
        dx_score = 0

    combo_status = normalize_int(detail.get("comboStatus"))
    sync_status = normalize_int(detail.get("syncStatus"))
    fc = COMBO_STATUS_TO_FC.get(combo_status, "")
    fs = SYNC_STATUS_TO_FS.get(sync_status, "")

    return (
        {
            "achievements": round(achievements, 4),
            "dxScore": dx_score,
            "fc": fc,
            "fs": fs,
            "level_index": level_index,
            "title": song["title"],
            "type": song["type"],
        },
        None,
    )


def convert_raw_records(
    raw: Any,
    *,
    music_index: dict[int, dict[str, str]],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    payload: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []
    seen: set[tuple[str, str, int]] = set()
    for detail in iter_user_music_details(raw):
        converted, skip = convert_record(detail, music_index=music_index)
        if skip is not None:
            skipped.append(skip)
            continue
        assert converted is not None
        key = (converted["title"], converted["type"], converted["level_index"])
        if key in seen:
            skipped.append({"reason": "duplicate_payload_key", "key": list(key), "record": detail})
            continue
        seen.add(key)
        payload.append(converted)
    return payload, skipped


def write_json(path: Path | None, data: Any, *, pretty: bool) -> None:
    text = json.dumps(data, ensure_ascii=False, indent=2 if pretty else None)
    if pretty:
        text += "\n"
    if path is None:
        print(text)
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def convert_file(
    input_path: Path,
    *,
    output: Path | None = None,
    report: Path | None = None,
    pretty: bool = False,
    music_data: Path | None = None,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    song_list = music_data or default_music_data_path()
    if not song_list.is_file():
        raise FileNotFoundError(
            f"水鱼曲库缓存不存在: {song_list}。"
            "请运行 maimai update music，或确保网络可访问 Diving-Fish /music_data。"
        )
    music_index = build_music_index(music_data=song_list)
    raw = load_json(input_path)
    payload, skipped = convert_raw_records(raw, music_index=music_index)
    write_json(output, payload, pretty=pretty)
    report_body = {
        "input": str(input_path),
        "output": str(output) if output else None,
        "musicData": str(song_list),
        "converted": len(payload),
        "skipped": len(skipped),
        "skippedRecords": skipped,
    }
    if report is not None:
        write_json(report, report_body, pretty=True)
    return payload, report_body


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Convert full official maimai raw JSON into Diving-Fish /player/update_records payload."
    )
    parser.add_argument("input", type=Path, help="Full raw JSON export containing GetUserMusicApi.")
    parser.add_argument("-o", "--output", type=Path, help="Output payload JSON path. Defaults to stdout.")
    parser.add_argument("--report", type=Path, help="Optional skipped-record report JSON path.")
    parser.add_argument("--pretty", action="store_true", help="Pretty-print output JSON.")
    parser.add_argument(
        "--music-data",
        type=Path,
        default=None,
        help="Diving-Fish music_data.json cache path (default: STATIC_PATH/data/music_data.json)",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    convert_file(
        args.input,
        output=args.output,
        report=args.report,
        pretty=args.pretty,
        music_data=args.music_data,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
