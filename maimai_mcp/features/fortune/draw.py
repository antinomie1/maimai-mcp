"""Fortune: only attach jacket art via original song_chart path (no custom layout)."""

from __future__ import annotations

import time
from pathlib import Path

from PIL import Image

from ...core.image.tools import song_chart
from ...core.io_image import default_out_path
from ...core.merge.models import Song
from ...result import FeatureResult, image_result


def draw_fortune_jacket(
    song: Song,
    text: str,
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"fortune_{song.song_id}", out)
    t0 = time.perf_counter()
    img = Image.open(song_chart(song.song_id))
    return image_result(img, path, text=text, t0=t0)
