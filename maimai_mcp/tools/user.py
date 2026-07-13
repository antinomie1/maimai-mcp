"""Session identity and user settings tools."""

from __future__ import annotations

from textwrap import dedent

from mcp.server.fastmcp import FastMCP

from maimai_mcp.config import lxnsconfig
from maimai_mcp.core.database.qq import update_user
from maimai_mcp.core.domain import bind_lxns
from maimai_mcp.core.errors import ValidationError
from maimai_mcp.core.merge.models import ServiceName, Theme
from maimai_mcp.core.qq_identity_store import get_identity, resolve_identities
from maimai_mcp.core.user import resolve_user
from maimai_mcp.result import FeatureResult

from ..context import session
from ..formatters import result_to_json
from ..runtime import ensure_ready, run_fr, with_session_player
from ..schemas import (
    BindInput,
    GetQqIdentityInput,
    IdentityInput,
    PlayerArgs,
    ResolveQqInput,
    SourceInput,
    ThemeInput,
)


def _qq_from_session(qq: int | None = None) -> int | None:
    """Resolve qq for user-settings tools (with group-as-qq guard)."""
    return with_session_player(PlayerArgs(qq=qq)).qq


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_set_identity",
        annotations={
            "title": "Set session player + group context",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_set_identity(params: IdentityInput) -> str:
        """Set player QQ / username and optional group_id for this MCP process.

        group_id is context only (never used as score qq). Call at conversation start:
        qq=sender_id, group_id=group number.
        """

        async def _go():
            try:
                session.set_identity(
                    qq=params.qq, username=params.username, group_id=params.group_id
                )
            except ValidationError as e:
                return FeatureResult.failure(e.message, code=e.code)
            return FeatureResult.success(
                text="identity updated", data=session.snapshot()
            )

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_get_session",
        annotations={
            "title": "Get session state",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_get_session() -> str:
        """Return session defaults: default_qq, group_id, username, last songs."""
        return result_to_json(FeatureResult.success(data=session.snapshot()))

    @mcp.tool(
        name="maimai_resolve_qq",
        annotations={
            "title": "Resolve QQ from nickname cache",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_resolve_qq(params: ResolveQqInput) -> str:
        """Lookup player QQ via optional identity_cache.json.

        Needs QQ_IDENTITY_CACHE_DIR or ./qq-identity-cache with identity_cache.json.
        """
        data = resolve_identities(
            params.query,
            group_id=params.group_id if params.group_id is not None else session.group_id,
            max_results=params.max_results,
        )
        n = len(data.get("matches") or [])
        text = (
            f"找到 {n} 个候选"
            + ("（重名请让用户确认）" if data.get("ambiguous") else "")
            if n
            else f"缓存中未找到：{params.query}"
        )
        return result_to_json(FeatureResult.success(text=text, data=data))

    @mcp.tool(
        name="maimai_get_qq_identity",
        annotations={
            "title": "Get cached QQ identity",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_get_qq_identity(params: GetQqIdentityInput) -> str:
        """Read nickname / group card for a QQ from identity cache if present."""
        ident = get_identity(
            params.qq,
            params.group_id if params.group_id is not None else session.group_id,
        )
        if not ident:
            return result_to_json(
                FeatureResult.success(
                    text=f"QQ {params.qq} 不在身份缓存中",
                    data={"qq": params.qq, "identity": None},
                )
            )
        return result_to_json(
            FeatureResult.success(
                text=f"QQ={ident.get('qq')} nick={ident.get('qqNickname')}",
                data=ident,
            )
        )

    @mcp.tool(
        name="maimai_user_show",
        annotations={
            "title": "Show local user settings",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_user_show(qq: int | None = None) -> str:
        """Show theme/service for a QQ (session default if omitted)."""

        async def _go():
            await ensure_ready(load_music=False)
            user = await resolve_user(_qq_from_session(qq))
            return FeatureResult.success(
                text=f"QQ={user.qqid} service={user.service.value} theme={user.theme.value}",
                data={
                    "qq": user.qqid,
                    "service": user.service.value,
                    "theme": user.theme.value,
                },
            )

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_user_set_theme",
        annotations={
            "title": "Set user theme",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_user_set_theme(params: ThemeInput) -> str:
        """Set theme: circle (default) or prism_plus. Stored per QQ in user.db."""

        async def _go():
            await ensure_ready(load_music=False)
            theme = Theme.get_by_index(params.value)
            if theme is None:
                return FeatureResult.failure(
                    f"unknown theme:\n{Theme.get_help()}", code="validation_error"
                )
            user = await resolve_user(_qq_from_session(params.qq))
            await update_user(user.qqid, theme=theme)
            return FeatureResult.success(text=f"theme={theme.value}")

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_user_set_source",
        annotations={
            "title": "Set score data source",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_user_set_source(params: SourceInput) -> str:
        """Set Diving-Fish or Lxns as default score source for a QQ."""

        async def _go():
            await ensure_ready(load_music=False)
            source = ServiceName.get_by_index(params.value)
            if source is None:
                return FeatureResult.failure(
                    f"unknown source:\n{ServiceName.get_help()}",
                    code="validation_error",
                )
            user = await resolve_user(_qq_from_session(params.qq))
            await update_user(user.qqid, service=source)
            return FeatureResult.success(text=f"source={source.value}")

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_user_bind_lxns",
        annotations={
            "title": "Bind Lxns OAuth",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": False,
            "openWorldHint": True,
        },
    )
    async def maimai_user_bind_lxns(params: BindInput) -> str:
        """Get Lxns authorize URL, or submit code to finish binding."""

        async def _go():
            await ensure_ready(load_music=False)
            if not params.code:
                url = (
                    "https://maimai.lxns.net/oauth/authorize"
                    "?response_type=code"
                    f"&client_id={lxnsconfig.lx_client_id}"
                    f"&redirect_uri={lxnsconfig.redirect_uri}"
                    "&scope=read_player+read_user_profile+write_player"
                )
                return FeatureResult.success(
                    text=dedent(
                        f"""
                        Open the URL, then call again with code=XXXX-XXXX-XXXX
                        {url}
                        """
                    ).strip(),
                    data={"authorize_url": url},
                )
            user = await resolve_user(_qq_from_session(params.qq))
            msg = await bind_lxns(user, params.code)
            return FeatureResult.success(text=msg)

        return result_to_json(await run_fr(_go()))
