"""Fortune: only attach jacket art via original song_chart path (no custom layout)."""

from __future__ import annotations

from pathlib import Path

from PIL import Image

from ...core.image.tools import song_chart
from ...core.io_image import default_out_path, save_image
from ...core.merge.models import Song
from ...result import FeatureResult


def draw_fortune_jacket(
    song: Song,
    text: str,
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"fortune_{song.song_id}", out)
    img = Image.open(song_chart(song.song_id))
    return FeatureResult(text=text, image_path=save_image(img, path))
