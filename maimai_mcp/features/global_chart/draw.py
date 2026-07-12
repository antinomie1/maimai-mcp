"""ginfo pie chart via original song_global_data."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.image.chart import song_global_data
from ...core.io_image import default_out_path
from ...core.merge.models import Song
from ...result import FeatureResult, image_result


async def draw_global_chart(
    song: Song,
    level_index: int,
    text: str,
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"ginfo_{song.song_id}_{level_index}", out)
    t0 = time.perf_counter()
    image = await song_global_data(song, level_index)
    return image_result(image, path, text=text, t0=t0)
