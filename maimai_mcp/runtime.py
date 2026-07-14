"""Bootstrap helpers and player arg guards for MCP tools."""

from __future__ import annotations

from typing import TypeVar

from maimai_mcp.bootstrap import bootstrap
from maimai_mcp.core.errors import ValidationError, as_result
from maimai_mcp.core.qq_identity_store import looks_like_group_id
from maimai_mcp.result import FeatureResult

from .schemas import PlayerArgs

TPlayer = TypeVar("TPlayer", bound=PlayerArgs)

GROUP_AS_QQ_MSG = (
    "看起来传入的是群号而不是玩家 QQ。"
    "请传发送者 QQ（或被查对象 QQ），群号不要填进 qq。"
)


async def ensure_ready(*, load_music: bool = True) -> None:
    await bootstrap(load_music=load_music, quiet=True, preload_assets=None)


def normalize_player(args: TPlayer) -> TPlayer:
    """Reject group-id-like qq; do not fill defaults from session."""
    if args.qq is not None and looks_like_group_id(args.qq):
        raise ValidationError(GROUP_AS_QQ_MSG)
    return args


def guard_qq(qq: int | None) -> int | None:
    """Validate optional qq for settings tools (no session fill)."""
    if qq is not None and looks_like_group_id(qq):
        raise ValidationError(GROUP_AS_QQ_MSG)
    return qq


async def run_fr(coro) -> FeatureResult:
    return await as_result(coro)
