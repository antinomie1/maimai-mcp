"""Ranking list as text image (original text_to_image)."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.image.tools import image_to_base64, text_to_image
from ...core.io_image import default_out_path
from ...result import FeatureResult, image_result


def draw_ranking_list(text: str, *, out: Path | str | None = None) -> FeatureResult:
    path = default_out_path("ranking", out)
    t0 = time.perf_counter()
    image = image_to_base64(text_to_image(text))
    return image_result(image, path, text=text, t0=t0)
