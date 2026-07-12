"""Score line (分数线) pure calculation."""

from __future__ import annotations

from textwrap import dedent

from ...core.errors import ValidationError, handle_errors
from ...core.service import mai

HELP = dedent("""\
    此功能为查找某首歌分数线设计。
    命令格式：分数线「难度+歌曲id」「分数线」
    例如：分数线 紫799 100
    命令将返回分数线允许的「TAP」「GREAT」容错，
    以及「BREAK」50落等价的「TAP」「GREAT」数。
""").strip()


@handle_errors
async def query_score_line(diff_label: str, song_id: int, line: float) -> str:
    level_labels = ["绿", "黄", "红", "紫", "白"]
    level_labels2 = ["Basic", "Advanced", "Expert", "Master", "Re:MASTER"]
    if diff_label not in level_labels:
        raise ValidationError("难度应为 绿/黄/红/紫/白")
    level_index = level_labels.index(diff_label)
    song = mai.total_list.by_id(song_id)
    if song is None or level_index >= len(song.difficulties):
        raise ValidationError("曲目或难度无效")
    chart = song.difficulties[level_index]
    tap = int(chart.notes.tap)
    slide = int(chart.notes.slide)
    hold = int(chart.notes.hold)
    touch = int(chart.notes.touch)
    brk = int(chart.notes.brk)
    total_score = tap * 500 + slide * 1500 + hold * 1000 + touch * 500 + brk * 2500
    break_bonus = 0.01 / brk
    break_50_reduce = total_score * break_bonus / 4
    reduce = 101 - line
    if reduce <= 0 or reduce >= 101:
        raise ValidationError("分数线无效")
    return (
        f"{song.song_name}「{level_labels2[level_index]}」\n"
        f"分数线「{line}%」\n允许的最多「TAP」「GREAT」数量为\n"
        f"「{(total_score * reduce / 10000):.2f}」(每个-{10000 / total_score:.4f}%),\n"
        f"「BREAK」50落(一共「{brk}」个)\n"
        f"等价于「{(break_50_reduce / 100):.3f}」个「TAP」"
        f"「GREAT」(-{break_50_reduce / total_score * 100:.4f}%)"
    )
