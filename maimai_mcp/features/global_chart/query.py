"""ginfo: global chart stats (text) + song for pie chart."""

from __future__ import annotations

from textwrap import dedent

from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import Song
from ...core.domain import resolve_song


@handle_errors
async def query_global_chart(
    song_key: str,
    level_index: int = 3,
) -> tuple[Song, int, str]:
    song = resolve_song(song_key)
    if song is None:
        raise ValidationError("未找到曲目")
    if isinstance(song, list):
        raise ValidationError(
            "找到多个曲目，请使用 ID：\n"
            + "\n".join(f"{s.song_id}：{s.song_name}" for s in song if s)
        )
    if level_index >= len(song.difficulties):
        raise ValidationError("该乐曲没有这个等级")
    stats = song.difficulties[level_index].stats
    if not stats:
        raise ValidationError("该乐曲还没有统计信息")
    info = dedent(f"""\
        游玩次数：{round(stats.cnt)}
        拟合难度：{stats.fit_diff:.2f}
        平均达成率：{stats.avg:.2f}%
        平均 DX 分数：{stats.avg_dx:.1f}
        谱面成绩标准差：{stats.std_dev:.2f}""")
    return song, level_index, info
