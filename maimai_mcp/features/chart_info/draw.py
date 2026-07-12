"""Chart info drawing — original song_chart_info / banquet (coords untouched)."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.image.chart import song_chart_banquet_info, song_chart_info
from ...core.io_image import default_out_path
from ...core.merge.models import Song
from ...result import FeatureResult, image_result


def draw_chart_info(
    song: Song,
    ctx: dict,
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"chart_{song.song_id}", out)
    t0 = time.perf_counter()
    if ctx.get("banquet"):
        image = song_chart_banquet_info(song)
    else:
        image = song_chart_info(
            song,
            ctx["calc"],
            ctx["is_full"],
            ctx["best_list"],
            ctx["theme"],
        )
    return image_result(
        image,
        path,
        text=f"ID.{song.song_id} {song.song_name}",
        t0=t0,
    )
