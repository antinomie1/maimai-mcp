"""minfo drawing via original song_play_data."""

from __future__ import annotations

import time
from pathlib import Path

from ...core.database.qq import User
from ...core.image.info import song_play_data
from ...core.io_image import default_out_path
from ...core.merge.models import Song
from ...result import FeatureResult, image_result


def draw_play_score(
    user: User,
    song: Song,
    play_result,
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"minfo_{user.qqid}_{song.song_id}", out)
    t0 = time.perf_counter()
    image = song_play_data(
        user.service, user.theme, song=song, play_result=play_result
    )
    return image_result(
        image,
        path,
        text=f"{song.song_id} {song.song_name}",
        t0=t0,
    )
