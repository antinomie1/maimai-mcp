"""Composite tools that call other tool implementations (in-process)."""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP

from maimai_mcp.result import FeatureResult

from ..formatters import result_to_json
from ..runtime import ensure_ready, run_fr, with_session_player
from ..schemas import (
    B50Input,
    ChartInput,
    LookupSongInput,
    PlayerOverviewInput,
    PlateInput,
    PushPlanInput,
    RiseInput,
    SearchInput,
    SongKeyInput,
)
from .catalog import chart_impl, search_impl
from .player import b50_impl, minfo_impl, rise_impl
from .tables import plate_impl


async def lookup_song_impl(params: LookupSongInput) -> FeatureResult:
    """search → chart (and optional minfo)."""
    await ensure_ready()
    params = with_session_player(params)
    search_fr = await search_impl(
        SearchInput(
            query=params.query,
            mode="标题",
            format="json",
            qq=params.qq,
            username=params.username,
        )
    )
    if not search_fr.ok:
        return search_fr
    songs = search_fr.data or []
    if not songs:
        # try as id/alias via chart directly
        return await chart_impl(
            ChartInput(
                song=params.query,
                qq=params.qq,
                username=params.username,
                format=params.format,
                out=params.out,
            )
        )
    if len(songs) > 1:
        return FeatureResult.success(
            text=f"找到 {len(songs)} 首，请用 song_id 调用 maimai_chart",
            data=songs[:20],
        )
    song_id = str(songs[0]["song_id"])
    chart_fr = await chart_impl(
        ChartInput(
            song=song_id,
            qq=params.qq,
            username=params.username,
            format=params.format,
            out=params.out,
        )
    )
    if not params.with_minfo:
        return chart_fr
    minfo_fr = await minfo_impl(
        SongKeyInput(
            song=song_id,
            qq=params.qq,
            username=params.username,
            format="json",
        )
    )
    # Prefer chart image; attach minfo data
    if chart_fr.ok:
        data = {
            "chart": chart_fr.data,
            "minfo": minfo_fr.data if minfo_fr.ok else {"error": minfo_fr.error},
            "song": songs[0],
        }
        return FeatureResult.success(
            text=chart_fr.text,
            data=data,
            image_path=chart_fr.image_path,
            draw_seconds=chart_fr.draw_seconds,
        )
    return chart_fr


async def player_overview_impl(params: PlayerOverviewInput) -> FeatureResult:
    await ensure_ready()
    params = with_session_player(params)
    b50_fr = await b50_impl(
        B50Input(
            qq=params.qq,
            username=params.username,
            format=params.format,
            out=params.out,
            include_image_b64=params.include_image_b64,
        )
    )
    if not params.with_rise:
        return b50_fr
    rise_fr = await rise_impl(
        RiseInput(
            qq=params.qq,
            username=params.username,
            format="json",
        )
    )
    if not b50_fr.ok:
        return b50_fr
    return FeatureResult.success(
        text=b50_fr.text,
        data={
            "b50": b50_fr.data,
            "rise": rise_fr.data if rise_fr.ok else {"error": rise_fr.error},
        },
        image_path=b50_fr.image_path,
        draw_seconds=b50_fr.draw_seconds,
    )


async def push_plan_impl(params: PushPlanInput) -> FeatureResult:
    """b50 (json) + rise + chart for first SD recommendation if any."""
    await ensure_ready()
    params = with_session_player(params)
    b50_fr = await b50_impl(
        B50Input(qq=params.qq, username=params.username, format="json")
    )
    rise_fr = await rise_impl(
        RiseInput(
            qq=params.qq,
            username=params.username,
            level=params.level,
            score=params.score,
            format="json",
        )
    )
    chart_fr: FeatureResult | None = None
    if rise_fr.ok and isinstance(rise_fr.data, dict):
        sd = rise_fr.data.get("sd") or []
        dx = rise_fr.data.get("dx") or []
        first = (sd[0] if sd else None) or (dx[0] if dx else None)
        if first is not None:
            sid = getattr(first, "song_id", None) or (
                first.get("song_id") if isinstance(first, dict) else None
            )
            if sid is not None:
                chart_fr = await chart_impl(
                    ChartInput(
                        song=str(sid),
                        qq=params.qq,
                        username=params.username,
                        format=params.format,
                        out=params.out,
                    )
                )
    return FeatureResult.success(
        text=(chart_fr.text if chart_fr and chart_fr.ok else None)
        or "push plan ready",
        data={
            "b50_ok": b50_fr.ok,
            "rise": rise_fr.data if rise_fr.ok else {"error": rise_fr.error},
            "chart": chart_fr.data if chart_fr and chart_fr.ok else None,
        },
        image_path=chart_fr.image_path if chart_fr and chart_fr.ok else None,
        draw_seconds=chart_fr.draw_seconds if chart_fr and chart_fr.ok else None,
    )


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_lookup_song",
        annotations={
            "title": "Lookup song (search+chart)",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_lookup_song(params: LookupSongInput) -> str:
        """Resolve a title/alias: search then draw chart; optional with_minfo for personal scores.

        Internally calls the same logic as maimai_search + maimai_chart (+ maimai_minfo).
        """
        return result_to_json(
            await run_fr(lookup_song_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_player_overview",
        annotations={
            "title": "Player overview (b50+rise)",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_player_overview(params: PlayerOverviewInput) -> str:
        """Compose b50 image/data with optional rise recommendation JSON."""
        return result_to_json(
            await run_fr(player_overview_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_push_plan",
        annotations={
            "title": "Push plan (rise + chart)",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_push_plan(params: PushPlanInput) -> str:
        """Recommend rating gains and draw the first recommended chart."""
        return result_to_json(
            await run_fr(push_plan_impl(params)),
            include_image_b64=params.include_image_b64,
        )

    @mcp.tool(
        name="maimai_plate_status",
        annotations={
            "title": "Plate status workflow",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": True,
        },
    )
    async def maimai_plate_status(params: PlateInput) -> str:
        """Plate progress/table (thin alias of maimai_plate for agent discoverability)."""
        return result_to_json(
            await run_fr(plate_impl(params)),
            include_image_b64=params.include_image_b64,
        )
