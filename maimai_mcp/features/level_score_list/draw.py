"""Score list drawing — original DrawScore.draw_score_list."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.database.qq import User
from ...core.image.score import DrawScore
from ...core.image.tools import tricolor_gradient_prism_plus
from ...core.io_image import default_out_path
from ...core.merge.models import PlayedResult
from ...result import FeatureResult, image_result


def draw_level_score_list(
    user: User,
    rating: str | float,
    page: int,
    new_play_result: list[PlayedResult],
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"score_list_{rating}_p{page}", out)
    t0 = time.perf_counter()
    result_sum = len(new_play_result)
    end_page = max(1, (result_sum + 79) // 80)
    page = max(1, min(page, end_page))
    to_page = 80 if page < end_page else (result_sum % 80 or 80)
    line = (to_page + 4) // 5
    if page < end_page:
        plc = line * 109 + 130 * 4
    else:
        multiplier = (to_page + 19) // 20
        actual_line = 4 if to_page <= 20 else line
        plc = actual_line * 109 + 130 * multiplier

    background_bg = tricolor_gradient_prism_plus(1400, 280 + plc)
    score = DrawScore(user.service, background_bg)
    image = score.draw_score_list(rating, new_play_result, page, end_page)
    return image_result(
        image,
        path,
        text=f"{rating} 分数列表",
        t0=t0,
    )
