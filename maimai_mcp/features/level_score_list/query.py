"""Level / constant score list query."""

from __future__ import annotations

import re

from ...constants import LEVEL_LIST
from ...core.database.qq import User
from ...core.domain import get_player_result_ref
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import PlayedResult
from ...core.user import resolve_player


@handle_errors
async def query_level_score_list(
    rating: str,
    *,
    qq: int | None = None,
    username: str | None = None,
    page: int = 1,
) -> tuple[User, str | float, int, list[PlayedResult]]:
    if "." in rating:
        if not re.fullmatch(r"[0-9]+\.[0-9]", rating):
            raise ValidationError("输入有误，定数仅有一位小数。")
        rating_v: str | float = round(float(rating), 1)
    elif rating not in LEVEL_LIST:
        raise ValidationError("无此等级")
    else:
        rating_v = rating

    ref = await resolve_player(
        qq, username, require_lxns_auth=not bool((username or "").strip())
    )
    assert ref is not None
    play_result = await get_player_result_ref(ref)
    new_play_result = sorted(
        filter(
            (lambda x: x.level == rating_v)
            if isinstance(rating_v, str)
            else (lambda x: x.level_value == rating_v),
            play_result,
        ),
        key=lambda y: y.achievements,
        reverse=True,
    )
    return ref.user, rating_v, page, new_play_result
