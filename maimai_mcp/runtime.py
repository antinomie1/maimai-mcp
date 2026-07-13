"""Bootstrap helpers and player resolution with session defaults."""

from __future__ import annotations

from typing import TypeVar

from maimai_mcp.bootstrap import bootstrap
from maimai_mcp.core.errors import ValidationError, as_result
from maimai_mcp.core.qq_identity_store import looks_like_group_id
from maimai_mcp.result import FeatureResult

from .context import GROUP_AS_QQ_MSG, session
from .schemas import PlayerArgs

TPlayer = TypeVar("TPlayer", bound=PlayerArgs)


async def ensure_ready(*, load_music: bool = True) -> None:
    await bootstrap(load_music=load_music, quiet=True, preload_assets=None)
    session.bootstrapped = True


def with_session_player(args: TPlayer) -> TPlayer:
    """Fill qq/username from session; sticky-update when explicit; block group-as-qq."""
    if args.qq is not None:
        session.assert_player_qq(args.qq)
        if looks_like_group_id(args.qq):
            raise ValidationError(GROUP_AS_QQ_MSG)
        session.default_qq = args.qq
    if args.username is not None:
        session.default_username = args.username.strip() or None

    qq = args.qq if args.qq is not None else session.default_qq
    username = (
        args.username if args.username is not None else session.default_username
    )
    session.assert_player_qq(qq)
    return args.model_copy(update={"qq": qq, "username": username})


async def run_fr(coro) -> FeatureResult:
    return await as_result(coro)
