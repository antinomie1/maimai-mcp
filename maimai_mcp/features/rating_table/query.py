"""Rating table / completion table query."""

from __future__ import annotations

from ...constants import LEVEL_LIST
from ...core.database.qq import User
from ...core.domain import get_player_result_ref
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import PlayedResult
from ...core.user import resolve_player


@handle_errors
async def query_rating_table(
    rating: str,
    *,
    qq: int | None = None,
    username: str | None = None,
    with_progress: bool = False,
) -> tuple[str, User | None, list[PlayedResult] | None, bool]:
    if rating in LEVEL_LIST[:6]:
        raise ValidationError("只支持查询 lv7-15 的定数表。")
    if rating not in LEVEL_LIST[6:]:
        raise ValidationError("无法识别的定数。")
    if not with_progress:
        return rating, None, None, False
    ref = await resolve_player(
        qq, username, require_lxns_auth=not bool((username or "").strip())
    )
    assert ref is not None
    play_result = await get_player_result_ref(ref)
    return rating, ref.user, play_result, True
