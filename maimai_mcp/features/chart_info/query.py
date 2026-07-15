"""Chart info data (id / 是什么歌 context)."""

from __future__ import annotations

from ...core.database.qq import User
from ...core.domain import get_chart_calc_context_ref, resolve_song
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import Song
from ...core.user import resolve_player


@handle_errors
async def query_chart_info(
    song_key: str | int,
    qq: int | None = None,
    *,
    username: str | None = None,
    optional_user: bool = True,
    source: str | None = None,
) -> tuple[Song, dict, User | None]:
    key = str(song_key)
    song = resolve_song(key)
    if song is None:
        raise ValidationError(f"未找到曲目「{key}」")
    if isinstance(song, list):
        raise ValidationError(
            "找到多个曲目，请使用 ID：\n"
            + "\n".join(f"{s.song_id}：{s.song_name}" for s in song if s)
        )
    ref = await resolve_player(
        qq,
        username,
        require_lxns_auth=False,
        optional=optional_user,
        source=source,
    )
    ctx = await get_chart_calc_context_ref(ref, song)
    return song, ctx, ref.user if ref else None
