"""Ranking list as text image (original text_to_image)."""

from __future__ import annotations

from pathlib import Path

from ...core.image.tools import image_to_base64, text_to_image
from ...core.io_image import default_out_path, save_image
from ...result import FeatureResult


def draw_ranking_list(text: str, *, out: Path | str | None = None) -> FeatureResult:
    path = default_out_path("ranking", out)
    image = image_to_base64(text_to_image(text))
    return FeatureResult(text=text, image_path=save_image(image, path))
