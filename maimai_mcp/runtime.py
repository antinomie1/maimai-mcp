"""Bootstrap helpers and player resolution with session defaults."""

from __future__ import annotations

from maimai_mcp.bootstrap import bootstrap
from maimai_mcp.core.errors import as_result
from maimai_mcp.core.user import PlayerRef, resolve_player
from maimai_mcp.result import FeatureResult

from .context import session
from .schemas import PlayerArgs


async def ensure_ready(*, load_music: bool = True) -> None:
    if session.bootstrapped and (not load_music or True):
        # still call bootstrap — it no-ops if already loaded
        pass
    await bootstrap(load_music=load_music, quiet=True, preload_assets=False)
    session.bootstrapped = True


def effective_player_args(args: PlayerArgs | None = None) -> tuple[int | None, str | None]:
    qq = args.qq if args and args.qq is not None else session.default_qq
    username = (
        args.username
        if args and args.username is not None
        else session.default_username
    )
    return qq, username


async def resolve_from_args(
    args: PlayerArgs | None = None,
    *,
    require_lxns_auth: bool = False,
    optional: bool = False,
) -> PlayerRef | None:
    qq, username = effective_player_args(args)
    ref = await resolve_player(
        qq,
        username,
        require_lxns_auth=require_lxns_auth and not (username or "").strip(),
        optional=optional,
    )
    if ref is not None:
        session.last_player_label = ref.label
    return ref


async def run_fr(coro) -> FeatureResult:
    return await as_result(coro)
