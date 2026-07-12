"""Backward-compatible re-exports. Prefer ``maimai_mcp.core.domain``."""

from .domain import (
    PLAN_MAP,
    bind_lxns,
    build_level_progress_data,
    get_best50,
    get_chart_calc_context,
    get_mai_what_song,
    get_play_score,
    get_player_result,
    get_rise_score_list,
    get_rows,
    get_token,
    resolve_song,
)
from .clients.exceptions import NotMusicRecommendationError
from ..constants import VERSION_MAP
from .merge.models import Category

__all__ = [
    "PLAN_MAP",
    "VERSION_MAP",
    "Category",
    "NotMusicRecommendationError",
    "bind_lxns",
    "build_level_progress_data",
    "get_best50",
    "get_chart_calc_context",
    "get_mai_what_song",
    "get_play_score",
    "get_player_result",
    "get_rise_score_list",
    "get_rows",
    "get_token",
    "resolve_song",
]
