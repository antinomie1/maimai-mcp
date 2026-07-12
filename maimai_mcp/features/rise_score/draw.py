"""Rise score drawing — original DrawScore.draw_rise."""

from __future__ import annotations

from pathlib import Path

from ...core.database.qq import User
from ...core.image.score import DrawScore
from ...core.image.tools import tricolor_gradient_prism_plus
from ...core.io_image import default_out_path, save_image
from ...core.merge.models import RiseResult
from ...result import FeatureResult


def draw_rise_score(
    user: User,
    sd: list[RiseResult],
    sd_low: int,
    dx: list[RiseResult],
    dx_low: int,
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"rise_{user.qqid}", out)
    background_bg = tricolor_gradient_prism_plus(1400, 960)
    ds = DrawScore(user.service, background_bg)
    image = ds.draw_rise(sd, sd_low, dx, dx_low, 960)
    return FeatureResult(text="上分推荐", image_path=save_image(image, path))
