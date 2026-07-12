"""Single song play score (minfo) query."""

from __future__ import annotations

from ...core.database.qq import User
from ...core.domain import get_play_score_ref, resolve_song
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import Song
from ...core.user import resolve_player


@handle_errors
async def query_play_score(
    song_key: str,
    qq: int | None = None,
    *,
    username: str | None = None,
) -> tuple[User, Song, list]:
    ref = await resolve_player(
        qq, username, require_lxns_auth=not bool((username or "").strip())
    )
    assert ref is not None
    song = resolve_song(song_key)
    if song is None:
        raise ValidationError("未找到曲目")
    if isinstance(song, list):
        raise ValidationError(
            "找到多个曲目，请使用 ID 查询：\n"
            + "\n".join(f"{s.song_id}：{s.song_name}" for s in song if s)
        )
    play_result = await get_play_score_ref(ref, song)
    return ref.user, song, play_result
