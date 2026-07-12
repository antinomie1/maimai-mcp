"""Rating table drawing — original DrawRatingTable."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.image.rating_table import DrawRatingTable
from ...core.io_image import default_out_path
from ...core.merge.models import PlayedResult, ServiceName
from ...result import FeatureResult, image_result


def draw_rating_table_text(rating: str, *, out: Path | str | None = None) -> FeatureResult:
    path = default_out_path(f"rating_table_{rating}", out)
    t0 = time.perf_counter()
    table = DrawRatingTable(rating, level_text=True)
    image = table.draw()
    return image_result(image, path, text=f"{rating}定数表", t0=t0)


def draw_rating_table_progress(
    rating: str,
    service: ServiceName,
    play_result: list[PlayedResult],
    *,
    plan: bool = False,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"rating_table_{rating}_{'plan' if plan else 'pfm'}", out)
    t0 = time.perf_counter()
    table = DrawRatingTable(
        rating, service=service, play_result=play_result, plan=plan
    )
    image = table.draw()
    return image_result(image, path, text=f"{rating}完成表", t0=t0)
