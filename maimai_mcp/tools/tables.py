"""Table / progress tools."""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP

from maimai_mcp.features.level_progress.draw import draw_level_progress
from maimai_mcp.features.level_progress.query import query_level_progress
from maimai_mcp.features.level_score_list.draw import draw_level_score_list
from maimai_mcp.features.level_score_list.query import query_level_score_list
from maimai_mcp.features.plate_table.draw import draw_plate_progress, draw_plate_table
from maimai_mcp.features.plate_table.query import query_plate
from maimai_mcp.features.rating_table.draw import (
    draw_rating_table_progress,
    draw_rating_table_text,
)
from maimai_mcp.features.rating_table.query import query_rating_table
from maimai_mcp.result import FeatureResult

from ..formatters import result_to_json
from ..runtime import ensure_ready, run_fr, normalize_player
from ..schemas import (
    LevelProgressInput,
    PlateInput,
    RatingTableInput,
    ScoreListInput,
)


async def plate_impl(params: PlateInput) -> FeatureResult:
    await ensure_ready()
    params = normalize_player(params)
    user, play_result, ver, version_name, plan = await query_plate(
        params.ver,
        params.plan,
        params.qq,
        username=params.username,
        source=params.source,
    )
    kwargs = dict(
        service=user.service,
        play_result=play_result,
        plan=plan,
        version=ver,
        version_name=version_name,
        page=params.page,
        out=params.out,
    )
    if params.mode == "progress":
        return draw_plate_progress(**kwargs)
    return draw_plate_table(**kwargs)


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_rating_table",
        annotations={
            "title": "Level rating table",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_rating_table(params: RatingTableInput) -> str:
        """Level constant table; progress=true draws personal completion."""

        async def _go():
            await ensure_ready()
            p = normalize_player(params)
            rating, user, play_result, with_p = await query_rating_table(
                p.level,
                qq=p.qq,
                username=p.username,
                with_progress=p.progress or p.plan,
                source=p.source,
            )
            if with_p and user and play_result is not None:
                return draw_rating_table_progress(
                    rating,
                    user.service,
                    play_result,
                    plan=p.plan,
                    out=p.out,
                )
            return draw_rating_table_text(rating, out=p.out)

        return result_to_json(
            await run_fr(_go()), include_image_b64=params.include_image_b64
        )

    @mcp.tool(
        name="maimai_plate",
        annotations={
            "title": "Plate completion / progress",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_plate(params: PlateInput) -> str:
        """Plate table or progress (e.g. ver=祝 plan=将 mode=progress)."""
        return result_to_json(
            await run_fr(plate_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_level_progress",
        annotations={
            "title": "Level achievement progress",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_level_progress(params: LevelProgressInput) -> str:
        """Progress for a level + plan (e.g. level=14 plan=ap)."""

        async def _go():
            await ensure_ready()
            p = normalize_player(params)
            user, level, plan, cat, page, c, u, n = await query_level_progress(
                p.level,
                p.plan,
                qq=p.qq,
                username=p.username,
                category=p.category,
                page=p.page,
                source=p.source,
            )
            return draw_level_progress(
                user, level, plan, cat, page, c, u, n, out=p.out
            )

        return result_to_json(
            await run_fr(_go()), include_image_b64=params.include_image_b64
        )

    @mcp.tool(
        name="maimai_score_list",
        annotations={
            "title": "Score list by level/constant",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_score_list(params: ScoreListInput) -> str:
        """Personal score list for a level or constant (e.g. 14.0)."""

        async def _go():
            await ensure_ready()
            p = normalize_player(params)
            user, rating, page, results = await query_level_score_list(
                p.rating,
                qq=p.qq,
                username=p.username,
                page=p.page,
                source=p.source,
            )
            if p.format == "json" and not p.out:
                return FeatureResult.success(data=results)
            return draw_level_score_list(
                user, rating, page, results, out=p.out
            )

        return result_to_json(
            await run_fr(_go()), include_image_b64=params.include_image_b64
        )
