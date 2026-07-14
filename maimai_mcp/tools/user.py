"""User settings and NapCat identity-cache tools."""

from __future__ import annotations

from textwrap import dedent

from mcp.server.fastmcp import FastMCP

from maimai_mcp.config import lxnsconfig
from maimai_mcp.core.database.qq import update_user
from maimai_mcp.core.domain import bind_lxns
from maimai_mcp.core.errors import ValidationError
from maimai_mcp.core.merge.models import ServiceName, Theme
from maimai_mcp.core.qq_identity_store import (
    IdentityRefreshError,
    cache_status,
    get_identity,
    refresh_identity_cache,
    resolve_identities,
)
from maimai_mcp.core.user import resolve_user
from maimai_mcp.result import FeatureResult

from ..formatters import result_to_json
from ..runtime import ensure_ready, guard_qq, run_fr
from ..schemas import (
    BindInput,
    GetQqIdentityInput,
    RefreshIdentityInput,
    ResolveQqInput,
    SourceInput,
    ThemeInput,
)


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_refresh_identity",
        annotations={
            "title": "Refresh QQ identity cache via OneBot",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_refresh_identity(params: RefreshIdentityInput) -> str:
        """Pull friend list, groups, and group members via NapCat HTTP API.

        Requires NAPCAT_BASE_URL (or ONEBOT_BASE_URL). Writes identity_cache.json.
        """

        async def _go():
            try:
                status = await refresh_identity_cache(
                    base_url=params.base_url,
                    no_cache=params.no_cache,
                    timeout_ms=params.timeout_ms,
                    group_delay_ms=params.group_delay_ms,
                    max_groups=params.max_groups,
                )
            except IdentityRefreshError as e:
                return FeatureResult.failure(e.message, code=e.code)
            stats = status.get("stats") or {}
            text = (
                "身份缓存已刷新："
                f"好友 {stats.get('friendCount', 0)}，"
                f"群 {stats.get('groupCount', 0)}，"
                f"唯一用户 {stats.get('uniqueUsers', 0)}"
            )
            return FeatureResult.success(text=text, data=status)

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_identity_status",
        annotations={
            "title": "Identity cache status",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_identity_status() -> str:
        """Show identity cache path, stats, and configured NapCat base URL."""
        status = cache_status()
        stats = status.get("stats") or {}
        text = (
            f"缓存={'有' if status.get('cacheExists') else '无'}，"
            f"好友 {stats.get('friendCount', 0)}，"
            f"群 {stats.get('groupCount', 0)}，"
            f"用户 {stats.get('uniqueUsers', 0)}"
        )
        return result_to_json(FeatureResult.success(text=text, data=status))

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
        """Lookup player QQ from identity cache (nickname / card / waterfish name).

        Run maimai_refresh_identity first if cache is empty.
        """
        data = resolve_identities(
            params.query,
            group_id=params.group_id,
            max_results=params.max_results,
        )
        n = len(data.get("matches") or [])
        text = (
            f"找到 {n} 个候选"
            + ("（重名请让用户确认）" if data.get("ambiguous") else "")
            if n
            else "缓存中未找到匹配（可先 maimai_refresh_identity）"
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
        ident = get_identity(params.qq, params.group_id)
        if not ident:
            return result_to_json(
                FeatureResult.success(
                    text="该用户不在身份缓存中（可先刷新缓存）",
                    data={"identity": None},
                )
            )
        nick = ident.get("qqNickname") or ident.get("friendNickname") or ""
        return result_to_json(
            FeatureResult.success(
                text=f"缓存命中 nick={nick}" if nick else "缓存命中",
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
        """Show theme/service for a QQ (DEFAULT_QQ if omitted)."""

        async def _go():
            await ensure_ready(load_music=False)
            try:
                user = await resolve_user(guard_qq(qq))
            except ValidationError as e:
                return FeatureResult.failure(e.message, code=e.code)
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
            try:
                user = await resolve_user(guard_qq(params.qq))
            except ValidationError as e:
                return FeatureResult.failure(e.message, code=e.code)
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
            try:
                user = await resolve_user(guard_qq(params.qq))
            except ValidationError as e:
                return FeatureResult.failure(e.message, code=e.code)
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
            try:
                user = await resolve_user(guard_qq(params.qq))
            except ValidationError as e:
                return FeatureResult.failure(e.message, code=e.code)
            msg = await bind_lxns(user, params.code)
            return FeatureResult.success(text=msg)

        return result_to_json(await run_fr(_go()))
