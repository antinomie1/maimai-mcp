"""Session identity and user settings tools."""

from __future__ import annotations

from textwrap import dedent

from mcp.server.fastmcp import FastMCP

from maimai_mcp.config import lxnsconfig
from maimai_mcp.core.database.qq import update_user
from maimai_mcp.core.domain import bind_lxns
from maimai_mcp.core.merge.models import ServiceName, Theme
from maimai_mcp.core.user import resolve_user
from maimai_mcp.result import FeatureResult

from ..context import session
from ..formatters import result_to_json
from ..runtime import ensure_ready, run_fr
from ..schemas import BindInput, IdentityInput, SourceInput, ThemeInput


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_set_identity",
        annotations={
            "title": "Set session player identity",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_set_identity(params: IdentityInput) -> str:
        """Set default QQ and/or Diving-Fish username for subsequent tools in this session."""
        session.set_identity(qq=params.qq, username=params.username)
        return result_to_json(
            FeatureResult.success(text="identity updated", data=session.snapshot())
        )

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
        """Return current session defaults (qq, username, last songs)."""
        return result_to_json(FeatureResult.success(data=session.snapshot()))

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
            user = await resolve_user(qq if qq is not None else session.default_qq)
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
            user = await resolve_user(params.qq if params.qq is not None else session.default_qq)
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
            user = await resolve_user(params.qq if params.qq is not None else session.default_qq)
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
            user = await resolve_user(params.qq if params.qq is not None else session.default_qq)
            msg = await bind_lxns(user, params.code)
            return FeatureResult.success(text=msg)

        return result_to_json(await run_fr(_go()))
