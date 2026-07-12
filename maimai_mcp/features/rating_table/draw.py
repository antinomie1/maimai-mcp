"""Rating table drawing — original DrawRatingTable."""

from __future__ import annotations

from pathlib import Path

from ...core.image.rating_table import DrawRatingTable
from ...core.io_image import default_out_path, save_image
from ...core.merge.models import PlayedResult, ServiceName
from ...result import FeatureResult


def draw_rating_table_text(rating: str, *, out: Path | str | None = None) -> FeatureResult:
    path = default_out_path(f"rating_table_{rating}", out)
    table = DrawRatingTable(rating, level_text=True)
    image = table.draw()
    return FeatureResult(image_path=save_image(image, path), text=f"{rating}定数表")


def draw_rating_table_progress(
    rating: str,
    service: ServiceName,
    play_result: list[PlayedResult],
    *,
    plan: bool = False,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"rating_table_{rating}_{'plan' if plan else 'pfm'}", out)
    table = DrawRatingTable(
        rating, service=service, play_result=play_result, plan=plan
    )
    image = table.draw()
    return FeatureResult(image_path=save_image(image, path), text=f"{rating}完成表")
