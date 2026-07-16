"""Group rating / song ranking tools (fetch on query, reuse fresh cache)."""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP

from maimai_mcp.core.player_cache import cache_status as player_cache_status
from maimai_mcp.features.group_rank.query import (
    query_group_member_rank,
    query_group_rating_rank,
    query_group_song_rank,
)
from maimai_mcp.result import FeatureResult

from ..formatters import result_to_json
from ..runtime import ensure_ready, run_fr
from ..schemas import (
    GroupMemberRankInput,
    GroupRatingRankInput,
    GroupSongRankInput,
)


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_group_rating_rank",
        annotations={
            "title": "Group rating rank",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_group_rating_rank(params: GroupRatingRankInput) -> str:
        """本群 Rating 榜。查榜时按需拉成员成绩；新鲜本地缓存会复用。

        名册来自身份缓存。不在后台预拉全群；仅本次查榜触发请求。
        group_id 为群号，勿与玩家 qq 混淆。
        """

        async def _go():
            data = await query_group_rating_rank(
                params.group_id,
                sort_order=params.sort_order,
                output_limit=params.output_limit,
                start_rank=params.start_rank,
                end_rank=params.end_rank,
                rating_min=params.rating_min,
                rating_max=params.rating_max,
                force_refresh=params.force_refresh,
                max_concurrency=params.max_concurrency,
                query_delay_ms=params.query_delay_ms,
                max_members=params.max_members,
            )
            return FeatureResult.success(text=data.get("text"), data=data)

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_group_song_rank",
        annotations={
            "title": "Group song score rank",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_group_song_rank(params: GroupSongRankInput) -> str:
        """本群单曲成绩榜。查本榜时才按成员拉该曲成绩；新鲜缓存复用。

        song 支持 id/标题/别名。默认不高并发，避免打爆查分器。
        """

        async def _go():
            await ensure_ready(load_music=True)
            data = await query_group_song_rank(
                params.group_id,
                params.song,
                level_index=params.level_index,
                sort_by=params.sort_by,
                sort_order=params.sort_order,
                output_limit=params.output_limit,
                start_rank=params.start_rank,
                end_rank=params.end_rank,
                force_refresh=params.force_refresh,
                max_concurrency=params.max_concurrency,
                query_delay_ms=params.query_delay_ms,
                max_members=params.max_members,
            )
            return FeatureResult.success(text=data.get("text"), data=data)

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_group_member_rank",
        annotations={
            "title": "One member's group rank",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_group_member_rank(params: GroupMemberRankInput) -> str:
        """某人在本群 Rating 或某曲榜上的名次。

        会按需构建整榜（缺成绩的成员在查榜时拉取）。传 qq 或 target；可选 song。
        """

        async def _go():
            if params.song:
                await ensure_ready(load_music=True)
            data = await query_group_member_rank(
                group_id=params.group_id,
                qq=params.qq,
                target=params.target,
                song=params.song,
                level_index=params.level_index,
                context_size=params.context_size,
                force_refresh=params.force_refresh,
                max_concurrency=params.max_concurrency,
                query_delay_ms=params.query_delay_ms,
                max_members=params.max_members,
            )
            return FeatureResult.success(text=data.get("text"), data=data)

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_player_cache_status",
        annotations={
            "title": "Local player score cache status",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_player_cache_status() -> str:
        """本地成绩缓存覆盖（群榜复用；无网络请求）。"""
        status = player_cache_status()
        text = (
            f"成绩缓存目录 {status.get('cacheDir')}："
            f"玩家 {status.get('playerCount')}，"
            f"含 Rating {status.get('ratingCount')}，"
            f"单曲条目 {status.get('songEntryCount')}"
        )
        return result_to_json(FeatureResult.success(text=text, data=status))
