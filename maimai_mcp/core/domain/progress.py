"""Level / plate progress data assembly (no drawing)."""

from __future__ import annotations

from ...constants import (
    ACHIEVEMENT_LIST,
    COMBO_PLUS,
    COMBO_SP,
    RANK_PLUS,
    SYNC_D_SP,
    SYNC_PLUS,
    SYNC_SP,
)
from ..merge.models import NotPlayedResult, PlayedResult
from ..service import mai

PLAN_MAP: dict[str, tuple[int, int | float]] = {
    **{p: (0, ACHIEVEMENT_LIST[i - 1]) for i, p in enumerate(RANK_PLUS)},
    **{p: (1, i) for i, p in enumerate(COMBO_PLUS)},
    **{p: (2, i) for i, p in enumerate(SYNC_PLUS)},
}


def get_rows(count: int, row_size: int) -> int:
    if count == 0:
        return 0
    return (count + row_size - 1) // row_size


def build_level_progress_data(
    play_result: list[PlayedResult],
    level: str,
    plan: str,
) -> tuple[list[PlayedResult], list[PlayedResult], list[NotPlayedResult], int]:
    """Return completed, unfinished, notplayed, plan_type."""
    played_map: dict[tuple[int, int], PlayedResult] = {
        (r.song_id, r.level_index): r for r in play_result if r.level == level
    }
    plan_type, plan_value = PLAN_MAP[plan]

    def check_status(res: PlayedResult) -> bool:
        if plan_type == 0:
            return res.achievements >= plan_value
        if plan_type == 1:
            return bool(res.fc and COMBO_SP.index(res.fc) >= plan_value)
        if plan_type == 2:
            if not res.fs:
                return False
            if res.fs in SYNC_D_SP:
                return SYNC_D_SP.index(res.fs) >= plan_value
            if res.fs in SYNC_SP:
                return SYNC_SP.index(res.fs) >= plan_value
            return False
        return False

    completed: list[PlayedResult] = []
    unfinished: list[PlayedResult] = []
    notplayed: list[NotPlayedResult] = []

    music_list = mai.total_list.by_plan(level)
    for song_id, difficulties in music_list.items():
        for _d in difficulties:
            res = played_map.get((song_id, _d.level_index))
            if res:
                if check_status(res):
                    completed.append(res)
                else:
                    unfinished.append(res)
            else:
                notplayed.append(
                    NotPlayedResult(
                        level_value=_d.level_value,
                        song_id=song_id,
                        level_index=_d.level_index,
                    )
                )

    sort_key = {0: "achievements", 1: "fc", 2: "fs"}.get(plan_type, "achievements")
    sort_default: int | str = 0 if plan_type == 0 else ""

    def _sort_value(res: PlayedResult):
        value = getattr(res, sort_key)
        return value if value is not None else sort_default

    completed.sort(key=_sort_value, reverse=True)
    unfinished.sort(key=_sort_value, reverse=True)
    notplayed.sort(key=lambda x: x.level_value, reverse=True)
    return completed, unfinished, notplayed, plan_type
