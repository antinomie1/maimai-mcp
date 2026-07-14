"""Catalog / song lookup tools."""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP

from maimai_mcp.core.service import mai
from maimai_mcp.features.alias_query.query import add_local_alias, query_aliases
from maimai_mcp.features.chart_info.draw import draw_chart_info
from maimai_mcp.features.chart_info.query import query_chart_info
from maimai_mcp.features.random_song.query import query_random_song
from maimai_mcp.features.score_line.query import query_score_line
from maimai_mcp.features.search_song.draw import draw_search_result
from maimai_mcp.features.search_song.query import query_search
from maimai_mcp.result import FeatureResult

from ..formatters import result_to_json
from ..runtime import ensure_ready, run_fr, normalize_player
from ..schemas import (
    AliasAddInput,
    AliasQueryInput,
    ChartInput,
    RandomInput,
    ScoreLineInput,
    SearchInput,
    UpdateInput,
)


async def search_impl(params: SearchInput) -> FeatureResult:
    await ensure_ready()
    params = normalize_player(params)
    mode = None if params.mode == "标题" else params.mode
    songs, page = await query_search(params.query, mode=mode, page=params.page)
    if params.format == "json" and not params.out:
        return FeatureResult.success(
            data=[{"song_id": s.song_id, "song_name": s.song_name} for s in songs]
        )
    return await draw_search_result(
        songs,
        page,
        qq=params.qq,
        username=params.username,
        out=params.out,
    )


async def chart_impl(params: ChartInput) -> FeatureResult:
    await ensure_ready()
    params = normalize_player(params)
    song, ctx, _ = await query_chart_info(
        params.song, params.qq, username=params.username
    )
    if params.format == "json" and not params.out:
        theme = ctx.get("theme")
        return FeatureResult.success(
            data={
                "song_id": song.song_id,
                "song_name": song.song_name,
                "calc": ctx.get("calc"),
                "theme": getattr(theme, "value", theme),
            }
        )
    return draw_chart_info(song, ctx, out=params.out)


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_search",
        annotations={
            "title": "Search maimai songs",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_search(params: SearchInput) -> str:
        """Search songs by title, level value (定数), bpm, artist, or charter.

        Returns JSON list or chart image when one match. Prefer song_id with maimai_chart next.
        """
        return result_to_json(
            await run_fr(search_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_chart",
        annotations={
            "title": "Song chart info image",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_chart(params: ChartInput) -> str:
        """Draw chart info for a song id/title/alias. Optional player identity for push calc."""
        return result_to_json(
            await run_fr(chart_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_score_line",
        annotations={
            "title": "Score line tolerance calc",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_score_line(params: ScoreLineInput) -> str:
        """Compute TAP GREAT tolerance for a score line (diff color + song_id + line%)."""

        async def _go():
            await ensure_ready()
            text = await query_score_line(params.diff, params.song_id, params.line)
            return FeatureResult.success(text=text)

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_random",
        annotations={
            "title": "Random chart",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": False,
            "openWorldHint": False,
        },
    )
    async def maimai_random(params: RandomInput) -> str:
        """Random chart by level/type/color, then draw chart info."""

        async def _go():
            await ensure_ready()
            song = await query_random_song(
                level=params.level,
                chart_type=params.chart_type,
                color=params.color,
            )
            p = normalize_player(params)
            return await chart_impl(
                ChartInput(
                    song=str(song.song_id),
                    qq=p.qq,
                    username=p.username,
                    format=p.format,
                    out=p.out,
                    include_image_b64=p.include_image_b64,
                )
            )

        return result_to_json(
            await run_fr(_go()), include_image_b64=params.include_image_b64
        )

    @mcp.tool(
        name="maimai_alias_query",
        annotations={
            "title": "Query song aliases",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_alias_query(params: AliasQueryInput) -> str:
        """List aliases for a song name or id."""

        async def _go():
            await ensure_ready()
            text = await query_aliases(params.name, by_id=params.by_id)
            return FeatureResult.success(text=text)

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_alias_local_add",
        annotations={
            "title": "Add local alias",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_alias_local_add(params: AliasAddInput) -> str:
        """Add a local-only alias (not remote vote)."""

        async def _go():
            await ensure_ready()
            text = await add_local_alias(params.song_id, params.alias)
            return FeatureResult.success(text=text)

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_update_catalog",
        annotations={
            "title": "Update music/alias/table assets",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_update_catalog(params: UpdateInput) -> str:
        """Refresh music data, aliases, and/or regenerate rating/plate table images."""

        async def _go():
            await ensure_ready(load_music=False)
            what = list(params.what) if params.what else ["all"]
            if "all" in what:
                what = ["music", "alias", "tables"]
            msgs: list[str] = []
            if "music" in what:
                await mai.get_music()
                await mai.get_plate_json()
                msgs.append("music/plates updated")
            if "alias" in what:
                if not mai.loaded and not await mai.load_from_cache():
                    await mai.get_music()
                await mai.get_music_alias()
                msgs.append("aliases updated")
            if "tables" in what:
                await ensure_ready(load_music=True)
                from maimai_mcp.core.image.update_table import UpdateTable

                u = UpdateTable()
                await u.update_rating_table()
                await u.update_level_15_rating_table()
                await u.update_plate_table()
                await u.update_wu_plate_table()
                msgs.append("tables regenerated")
            mai._loaded = True
            return FeatureResult.success(text="; ".join(msgs))

        return result_to_json(await run_fr(_go()))
