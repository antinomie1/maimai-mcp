"""Random song / rise-oriented recommendation."""

from __future__ import annotations

from ...core.domain import get_mai_what_song
from ...core.errors import handle_errors
from ...core.merge.models import Song
from ...core.service import mai
from ...core.user import resolve_player


@handle_errors
async def query_mai_what(
    *,
    qq: int | None = None,
    username: str | None = None,
    rise: bool = False,
) -> Song:
    song = mai.total_list.random()
    if rise:
        try:
            ref = await resolve_player(qq, username, require_lxns_auth=False)
            if ref is not None:
                picked = await get_mai_what_song(
                    ref.user, username=ref.username
                )
                if picked is not None:
                    song = picked
        except Exception:
            pass
    return song
