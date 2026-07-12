"""Random chart filter."""

from __future__ import annotations

import random

from ...core.errors import ValidationError, handle_errors
from ...core.merge.models import Song
from ...core.service import mai


@handle_errors
async def query_random_song(
    *,
    level: str,
    chart_type: str | None = None,
    color: str | None = None,
) -> Song:
    if chart_type == "dx":
        type_ = ["DX"]
    elif chart_type in ("sd", "标准"):
        type_ = ["SD"]
    else:
        type_ = ["SD", "DX"]
    songs = mai.total_list.filter(level=level, type=type_)
    if color:
        ci = "绿黄红紫白".index(color)
        songs = [
            s
            for s in songs
            if len(s.difficulties) > ci and s.difficulties[ci].level == level
        ]
    if not songs:
        raise ValidationError("没有这样的乐曲哦。")
    return random.choice(songs)
