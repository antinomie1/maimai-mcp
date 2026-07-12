"""Level progress drawing — original DrawScore layout (coords untouched)."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.database.qq import User
from ...core.domain import get_rows
from ...core.image.score import DrawScore
from ...core.image.tools import tricolor_gradient_prism_plus
from ...core.io_image import default_out_path
from ...core.merge.models import Category, NotPlayedResult, PlayedResult
from ...result import FeatureResult, image_result


def draw_level_progress(
    user: User,
    level: str,
    plan: str,
    category: Category,
    page: int,
    completed: list[PlayedResult],
    unfinished: list[PlayedResult],
    notplayed: list[NotPlayedResult],
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"level_prog_{level}_{plan}_{category}_{page}", out)
    t0 = time.perf_counter()

    def get_played_rows(count: int) -> int:
        return max(4, get_rows(count, 5))

    def get_notplayed_rows(count: int) -> int:
        return max(4, get_rows(count, 20))

    if category == Category.DEFAULT:
        comp_limit = 60 if not unfinished and not notplayed else 30
        c_row = len(completed[:comp_limit])
        c_y = get_played_rows(c_row) * 109 + 140
        u_row = len(unfinished[:30])
        u_y = get_played_rows(u_row) * 109 + 140
        n_row = len(notplayed[:100])
        n_y = get_notplayed_rows(n_row) * 65 + 140
        background_bg = tricolor_gradient_prism_plus(1400, 150 + c_y + u_y + n_y)
        ds = DrawScore(user.service, background_bg)
        image = ds.draw_plan(
            level, completed, c_y, unfinished, u_y, notplayed, plan, comp_limit
        )
    elif category in [Category.COMPLETED, Category.UNFINISHED]:
        data = completed if category == Category.COMPLETED else unfinished
        per_page = 80
        total_page = max(1, (len(data) - 1) // per_page + 1)
        page = max(1, min(page, total_page))
        display_data = data[(page - 1) * per_page : page * per_page]
        y_size = get_played_rows(len(display_data)) * 109
        background_bg = tricolor_gradient_prism_plus(1400, 240 + y_size + 120)
        ds = DrawScore(user.service, background_bg)
        image = ds.draw_category(category, data, page, total_page)
    else:
        y_size = get_notplayed_rows(len(notplayed)) * 65
        height = max(600, 240 + y_size + 120)
        background_bg = tricolor_gradient_prism_plus(1400, height)
        ds = DrawScore(user.service, background_bg)
        image = ds.draw_category(category, notplayed)

    return image_result(
        image,
        path,
        text=f"{level} {plan} 进度",
        t0=t0,
    )
