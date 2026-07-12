"""Rise score recommendation query."""

from __future__ import annotations

from ...constants import LEVEL_LIST
from ...core.clients.exceptions import NotMusicRecommendationError
from ...core.clients.lxns.models import SongType
from ...core.database.qq import User
from ...core.domain import get_best50_ref, get_player_result_ref, get_rise_score_list
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import RiseResult
from ...core.user import resolve_player


@handle_errors
async def query_rise_score(
    *,
    qq: int | None = None,
    username: str | None = None,
    level: str | None = None,
    score: int | None = None,
) -> tuple[User, list[RiseResult], int, list[RiseResult], int]:
    if level and level not in LEVEL_LIST:
        raise ValidationError("无此等级")
    ref = await resolve_player(
        qq, username, require_lxns_auth=not bool((username or "").strip())
    )
    assert ref is not None
    _, best50 = await get_best50_ref(ref)
    play_result = await get_player_result_ref(ref)
    old_records = {(v.song_id, v.level_index): v for v in play_result}
    sd, sd_low = get_rise_score_list(
        old_records, SongType.STANDARD, best50.sd, level, score
    )
    dx, dx_low = get_rise_score_list(
        old_records, SongType.DX, best50.dx, level, score
    )
    if not sd and not dx:
        raise NotMusicRecommendationError
    return ref.user, sd, sd_low, dx, dx_low
