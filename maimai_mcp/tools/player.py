"""Player score tools: b50, minfo, rise, ranking, fortune, mai_what."""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP

from maimai_mcp.features.best50.draw import draw_best50
from maimai_mcp.features.best50.query import query_best50
from maimai_mcp.features.chart_info.draw import draw_chart_info
from maimai_mcp.features.chart_info.query import query_chart_info
from maimai_mcp.features.fortune.draw import draw_fortune_jacket
from maimai_mcp.features.fortune.query import query_fortune
from maimai_mcp.features.mai_what.query import query_mai_what
from maimai_mcp.features.play_score.draw import draw_play_score
from maimai_mcp.features.play_score.query import query_play_score
from maimai_mcp.features.ranking.draw import draw_ranking_list
from maimai_mcp.features.ranking.query import query_ranking
from maimai_mcp.features.rise_score.draw import draw_rise_score
from maimai_mcp.features.rise_score.query import query_rise_score
from maimai_mcp.result import FeatureResult

from ..formatters import result_to_json
from ..runtime import ensure_ready, run_fr, with_session_player
from ..schemas import (
    B50Input,
    FortuneInput,
    MaiWhatInput,
    RankingInput,
    RiseInput,
    SongKeyInput,
)


async def b50_impl(params: B50Input) -> FeatureResult:
    await ensure_ready(load_music=True)
    params = with_session_player(params)
    user, player, best50, by_name = await query_best50(
        params.qq, username=params.username, all_perfect=params.all_perfect
    )
    if params.format == "json" and not params.out:
        return FeatureResult.success(
            data={
                "qq": user.qqid,
                "username": params.username,
                "player": player,
                "best50": best50,
            }
        )
    return await draw_best50(
        user, player, best50, is_username=by_name, out=params.out
    )


async def minfo_impl(params: SongKeyInput) -> FeatureResult:
    await ensure_ready()
    params = with_session_player(params)
    user, song, play_result = await query_play_score(
        params.song, params.qq, username=params.username
    )
    if params.format == "json" and not params.out:
        return FeatureResult.success(
            data={"song_id": song.song_id, "play_result": play_result}
        )
    return draw_play_score(user, song, play_result, out=params.out)


async def rise_impl(params: RiseInput) -> FeatureResult:
    await ensure_ready()
    params = with_session_player(params)
    user, sd, sd_low, dx, dx_low = await query_rise_score(
        qq=params.qq,
        username=params.username,
        level=params.level,
        score=params.score,
    )
    if params.format == "json" and not params.out:
        return FeatureResult.success(data={"sd": sd, "dx": dx})
    return draw_rise_score(user, sd, sd_low, dx, dx_low, out=params.out)


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_b50",
        annotations={
            "title": "Best50 / AP50",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_b50(params: B50Input) -> str:
        """Fetch and optionally draw Best50. Use username for Diving-Fish or qq for bound service.

        AP50 requires Lxns + bound QQ (all_perfect=true). Default image theme from local user.
        """
        return result_to_json(
            await run_fr(b50_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_minfo",
        annotations={
            "title": "Personal song play info",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_minfo(params: SongKeyInput) -> str:
        """Personal scores for one song (minfo). Requires player identity."""
        return result_to_json(
            await run_fr(minfo_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_rise",
        annotations={
            "title": "Rating rise recommendations",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_rise(params: RiseInput) -> str:
        """Recommend charts to gain rating (上分). Optional level/score filter."""
        return result_to_json(
            await run_fr(rise_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_ranking",
        annotations={
            "title": "Diving-Fish rating ranking",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_ranking(params: RankingInput) -> str:
        """List or search Diving-Fish RA ranking. Use my=true with qq, or name/username."""

        async def _go():
            await ensure_ready(load_music=False)
            p = with_session_player(params)
            rank_name = p.name or (p.username or "")
            data = await query_ranking(
                name=rank_name,
                page=p.page,
                my_qq=p.qq if p.my and not rank_name else None,
            )
            if data.get("mode") == "list":
                return draw_ranking_list(data["text"])
            return FeatureResult.success(text=data.get("text"), data=data)

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_fortune",
        annotations={
            "title": "Daily fortune",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_fortune(params: FortuneInput) -> str:
        """Daily fortune + jacket for qq or username seed."""

        async def _go():
            await ensure_ready()
            p = with_session_player(params)
            text, song = await query_fortune(p.qq, username=p.username)
            if p.format == "json":
                return FeatureResult.success(
                    text=text,
                    data={"song_id": song.song_id, "song_name": song.song_name},
                )
            return draw_fortune_jacket(song, text, out=p.out)

        return result_to_json(
            await run_fr(_go()), include_image_b64=params.include_image_b64
        )

    @mcp.tool(
        name="maimai_mai_what",
        annotations={
            "title": "Random / rise-biased song",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": False,
            "openWorldHint": True,
        },
    )
    async def maimai_mai_what(params: MaiWhatInput) -> str:
        """Random song; with rise=true bias toward charts that push rating."""

        async def _go():
            await ensure_ready()
            p = with_session_player(params)
            song = await query_mai_what(
                qq=p.qq, username=p.username, rise=p.rise
            )
            song2, ctx, _ = await query_chart_info(
                str(song.song_id), p.qq, username=p.username
            )
            return draw_chart_info(song2, ctx, out=p.out)

        return result_to_json(
            await run_fr(_go()), include_image_b64=params.include_image_b64
        )
