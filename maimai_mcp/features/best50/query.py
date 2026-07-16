"""Best50 / AP50 data query."""

from __future__ import annotations

from ...core.database.qq import User
from ...core.domain import get_best50_ref
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import Best50, Player, ServiceName
from ...core.player_cache import write_rating
from ...core.qq_identity_store import upsert_waterfish_profile
from ...core.user import resolve_player


@handle_errors
async def query_best50(
    qq: int | None = None,
    *,
    username: str | None = None,
    all_perfect: bool = False,
    source: str | None = None,
) -> tuple[User, Player, Best50, bool]:
    """
    Returns:
        user, player, best50, is_username_query
    """
    ref = await resolve_player(
        qq,
        username,
        require_lxns_auth=not bool((username or "").strip()),
        source=source,
    )
    assert ref is not None
    if all_perfect and (
        ref.use_username or ref.user.service == ServiceName.DIVINGFISH
    ):
        raise ValidationError("仅落雪查分器支持 AP50（且需 --qq 绑定落雪）")
    player, best50 = await get_best50_ref(ref, all_perfect=all_perfect)

    # Side-effect: feed group-rank local cache (no extra API call).
    cache_qq = ref.user.qqid or qq
    if cache_qq and not all_perfect:
        write_rating(
            cache_qq,
            rating=player.rating,
            name=player.name,
            source=ref.user.service.value if ref.user.service else None,
        )
        upsert_waterfish_profile(cache_qq, nickname=player.name)

    return ref.user, player, best50, ref.use_username
