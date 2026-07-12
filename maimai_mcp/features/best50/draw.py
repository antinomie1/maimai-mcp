"""Best50 drawing — uses original PlayerBest50 (coordinates untouched)."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.database.qq import User
from ...core.image.best50 import PlayerBest50
from ...core.io_image import default_out_path
from ...core.merge.models import Best50, Player
from ...result import FeatureResult, image_result


async def draw_best50(
    user: User,
    player: Player,
    best50: Best50,
    *,
    is_username: bool = False,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"b50_{user.qqid}", out)
    b50 = PlayerBest50(
        user, player=player, best50=best50, is_username=is_username
    )
    t0 = time.perf_counter()
    image_b64 = await b50.draw()
    return image_result(
        image_b64,
        path,
        text="可使用 user_settings 更换主题/数据源。",
        image_b64=image_b64 if isinstance(image_b64, str) else None,
        t0=t0,
    )
