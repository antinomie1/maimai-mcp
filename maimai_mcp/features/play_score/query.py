"""Single song play score (minfo) query."""

from __future__ import annotations

from ...core.database.qq import User
from ...core.domain import get_play_score_ref, resolve_song
from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import Song
from ...core.player_cache import chart_to_cache_dict, write_song_scores
from ...core.user import resolve_player


@handle_errors
async def query_play_score(
    song_key: str,
    qq: int | None = None,
    *,
    username: str | None = None,
    source: str | None = None,
) -> tuple[User, Song, list]:
    ref = await resolve_player(
        qq,
        username,
        require_lxns_auth=not bool((username or "").strip()),
        source=source,
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

    # Side-effect: feed group song-rank local cache (no extra API call).
    cache_qq = ref.user.qqid or qq
    if cache_qq and play_result:
        write_song_scores(
            cache_qq,
            song_id=song.song_id,
            song_name=song.song_name,
            charts=[chart_to_cache_dict(c) for c in play_result],
            source=ref.user.service.value if ref.user.service else None,
        )

    return ref.user, song, play_result
