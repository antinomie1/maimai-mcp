"""Song search (定数/bpm/曲师/谱师/标题)."""

from __future__ import annotations

from ...core.errors import handle_errors
from ...core.merge.models import Song
from ...core.service import mai


def _is_float(value: str) -> bool:
    try:
        float(value)
        return True
    except ValueError:
        return False


@handle_errors
async def query_search(
    args: str,
    *,
    mode: str | None = None,
    page: int = 1,
) -> tuple[list[Song], int]:
    """
    mode: 定数 | bpm | 曲师 | 谱师 | None(标题模糊)
    """
    a_list = args.split() if args else []
    songs: list[Song] = []

    if mode == "定数" and a_list:
        # same spirit as depend.process_regex
        if len(a_list) == 1 and _is_float(a_list[0]):
            ds = float(a_list[0])
            songs = mai.total_list.filter(level_value=(ds, ds))
        elif len(a_list) >= 2 and _is_float(a_list[0]) and _is_float(a_list[1]):
            songs = mai.total_list.filter(
                level_value=(float(a_list[0]), float(a_list[1]))
            )
    elif mode == "bpm" and a_list:
        if len(a_list) == 1 and a_list[0].isdigit():
            songs = mai.total_list.filter(bpm=int(a_list[0]))
        elif len(a_list) >= 2 and a_list[0].isdigit() and a_list[1].isdigit():
            songs = mai.total_list.filter(bpm=(int(a_list[0]), int(a_list[1])))
    elif mode == "曲师" and a_list:
        songs = mai.total_list.filter(artist=" ".join(a_list))
    elif mode == "谱师" and a_list:
        songs = mai.total_list.filter(charter=" ".join(a_list))
    else:
        songs = list(mai.total_list.filter(title=args))

    return songs, page
