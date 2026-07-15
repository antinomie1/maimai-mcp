"""Level progress query."""

from __future__ import annotations

from ...constants import COMBO_PLUS, LEVEL_LIST, RANK_PLUS, SYNC_PLUS
from ...core.database.qq import User
from ...core.domain import build_level_progress_data, get_player_result_ref
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import Category, NotPlayedResult, PlayedResult
from ...core.user import resolve_player

CATEGORY_ALIAS = {
    "已完成": Category.COMPLETED,
    "未完成": Category.UNFINISHED,
    "未开始": Category.NOTPLAYED,
    "未游玩": Category.NOTPLAYED,
}


@handle_errors
async def query_level_progress(
    level: str,
    plan: str,
    *,
    qq: int | None = None,
    username: str | None = None,
    category: str | None = None,
    page: int = 1,
    source: str | None = None,
) -> tuple[
    User,
    str,
    str,
    Category,
    int,
    list[PlayedResult],
    list[PlayedResult],
    list[NotPlayedResult],
]:
    plan_l = plan.lower()
    if level not in LEVEL_LIST:
        raise ValidationError("无此等级")
    if plan_l not in RANK_PLUS + COMBO_PLUS + SYNC_PLUS:
        raise ValidationError("无此评价等级")
    if LEVEL_LIST.index(level) < 11 or (
        plan_l in RANK_PLUS and RANK_PLUS.index(plan_l) < 8
    ):
        raise ValidationError("兄啊，有点志向好不好")

    cat = Category.DEFAULT
    if category:
        if category not in CATEGORY_ALIAS:
            raise ValidationError(f"无法指定查询「{category}」")
        cat = CATEGORY_ALIAS[category]

    ref = await resolve_player(
        qq,
        username,
        require_lxns_auth=not bool((username or "").strip()),
        source=source,
    )
    assert ref is not None
    play_result = await get_player_result_ref(ref)
    completed, unfinished, notplayed, _ = build_level_progress_data(
        play_result, level, plan_l
    )
    return ref.user, level, plan_l, cat, page, completed, unfinished, notplayed
