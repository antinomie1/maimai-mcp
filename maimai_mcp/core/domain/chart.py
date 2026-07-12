"""Song resolution and chart-info calculation context."""

from __future__ import annotations

from typing import Any

from ..clients.divingfish.client import DivingFishAPI
from ..clients.lxns.client import LxnsAPI
from ..database.qq import User
from ..merge.models import ServiceName, Song, Theme
from ..merge.player import df_to_best50, lxns_to_best50
from ..service import mai
from ..user import PlayerRef
from .auth import get_token
from .player import _df_api


def resolve_song(data: str) -> Song | list[Song] | None:
    """Resolve song by id / exact title / alias. Multiple alias hits -> list."""
    data = data.strip().lower()
    if data.isdigit() and (by_id := mai.total_list.by_id(int(data))):
        return by_id
    if by_t := mai.total_list.by_name(data):
        return by_t
    aliases = mai.total_alias_list.by_alias(data)
    if not aliases:
        return None
    if len(aliases) != 1:
        return [
            s
            for a in aliases
            if (s := mai.total_list.by_id(a.song_id)) is not None
        ]
    return mai.total_list.by_id(aliases[0].song_id)


async def get_chart_calc_context(
    user: User | None,
    song: Song,
    *,
    username: str | None = None,
) -> dict[str, Any]:
    """Context for chart info push-score calculation (feeds song_chart_info)."""
    if song.song_id >= 100000:
        return {"banquet": True, "song": song}

    calc = False
    is_full = False
    best_list: list = []
    theme = Theme.CIRCLE

    if user is not None or username:
        if user is not None:
            theme = user.theme
        try:
            if username or (user and user.service == ServiceName.DIVINGFISH):
                api = _df_api(user or User(qqid=0, service=ServiceName.DIVINGFISH), username)
                userinfo = await api.query_user_b50()
                best50 = df_to_best50(userinfo)
                calc = True
            elif user and user.service == ServiceName.LXNS:
                token = get_token(user)
                api = LxnsAPI(user.qqid, token)
                best50 = lxns_to_best50(await api.best50())
                calc = True
            else:
                raise ValueError
            if calc:
                if song.isnew:
                    best_list = best50.dx
                    is_full = len(best_list) == 15
                else:
                    best_list = best50.sd
                    is_full = len(best_list) == 35
        except Exception:
            calc = False

    return {
        "banquet": False,
        "calc": calc,
        "is_full": is_full,
        "best_list": best_list,
        "theme": theme,
        "song": song,
    }


async def get_chart_calc_context_ref(
    ref: PlayerRef | None, song: Song
) -> dict[str, Any]:
    if ref is None:
        return await get_chart_calc_context(None, song)
    return await get_chart_calc_context(
        ref.user, song, username=ref.username
    )
