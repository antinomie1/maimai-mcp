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
from ..runtime import ensure_ready, run_fr, normalize_player
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
    params = normalize_player(params)
    user, player, best50, by_name = await query_best50(
        params.qq,
        username=params.username,
        all_perfect=params.all_perfect,
        source=params.source,
    )
    svc_label = (
        "水鱼"
        if user.service.value in ("Diving-Fish", "divingfish")
        else "落雪"
        if "Lxns" in user.service.value or user.service.value.lower() == "lxns"
        else user.service.value
    )
    if params.format == "json" and not params.out:
        return FeatureResult.success(
            text=f"数据源：{svc_label}",
            data={
                "qq": user.qqid,
                "username": params.username,
                "service": user.service.value,
                "source_label": svc_label,
                "player": player,
                "best50": best50,
            },
        )
    fr = await draw_best50(
        user, player, best50, is_username=by_name, out=params.out
    )
    base = fr.text or ""
    note = f"数据源：{svc_label}"
    fr.text = f"{note}\n{base}".strip() if base else note
    return fr


async def minfo_impl(params: SongKeyInput) -> FeatureResult:
    await ensure_ready()
    params = normalize_player(params)
    user, song, play_result = await query_play_score(
        params.song,
        params.qq,
        username=params.username,
        source=params.source,
    )
    if params.format == "json" and not params.out:
        return FeatureResult.success(
            data={"song_id": song.song_id, "play_result": play_result}
        )
    return draw_play_score(user, song, play_result, out=params.out)


async def rise_impl(params: RiseInput) -> FeatureResult:
    await ensure_ready()
    params = normalize_player(params)
    user, sd, sd_low, dx, dx_low = await query_rise_score(
        qq=params.qq,
        username=params.username,
        level=params.level,
        score=params.score,
        source=params.source,
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
        """Best50 image/data.

        params.source: one-shot divingfish/水鱼 or lxns/落雪 (agent maps user intent).
        Omit → QQ saved default. Do not use set_source for one-shot「水鱼b50」.
        AP50: all_perfect=true, Lxns only.
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
        """Personal scores for one song (minfo). Use params.source for one-shot 水鱼/落雪."""
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
        """Recommend charts to gain rating (上分). params.source for one-shot prober."""
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
            p = normalize_player(params)
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
            p = normalize_player(params)
            text, song = await query_fortune(
                p.qq, username=p.username, source=p.source
            )
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
            p = normalize_player(params)
            song = await query_mai_what(
                qq=p.qq, username=p.username, rise=p.rise, source=p.source
            )
            song2, ctx, _ = await query_chart_info(
                str(song.song_id),
                p.qq,
                username=p.username,
                source=p.source,
            )
            return draw_chart_info(song2, ctx, out=p.out)

        return result_to_json(
            await run_fr(_go()), include_image_b64=params.include_image_b64
        )
