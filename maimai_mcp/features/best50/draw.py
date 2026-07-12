"""Best50 drawing — uses original PlayerBest50 (coordinates untouched)."""

from __future__ import annotations

from pathlib import Path

from ...core.database.qq import User
from ...core.image.best50 import PlayerBest50
from ...core.io_image import default_out_path, save_image
from ...core.merge.models import Best50, Player
from ...result import FeatureResult


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
    image_b64 = await b50.draw()
    saved = save_image(image_b64, path)
    return FeatureResult(
        text="可使用 user_settings 更换主题/数据源。",
        image_path=saved,
        image_b64=image_b64 if isinstance(image_b64, str) else None,
    )
