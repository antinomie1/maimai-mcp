"""Search result drawing: chart info for 1, text for few, song_list for many."""

from __future__ import annotations

from pathlib import Path

from ...core.image.song import song_list
from ...core.io_image import default_out_path, save_image
from ...core.merge.models import Song
from ...result import FeatureResult
from ..chart_info.draw import draw_chart_info
from ..chart_info.query import query_chart_info


async def draw_search_result(
    songs: list[Song],
    page: int,
    *,
    qq: int | None = None,
    username: str | None = None,
    out: Path | str | None = None,
) -> FeatureResult:
    if not songs:
        return FeatureResult(error="没有找到这样的乐曲。")
    if len(songs) == 1:
        song, ctx, _ = await query_chart_info(
            str(songs[0].song_id), qq, username=username
        )
        return draw_chart_info(song, ctx, out=out)
    if len(songs) <= 5:
        lines = [f"{'「' + str(s.song_id) + '」':<7} {s.song_name}" for s in songs]
        return FeatureResult(text="\n".join(lines))
    path = default_out_path(f"search_p{page}", out)
    image = song_list(songs, page)
    saved = save_image(image, path)
    return FeatureResult(text=f"共 {len(songs)} 首", image_path=saved)
