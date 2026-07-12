"""minfo drawing via original song_play_data."""

from __future__ import annotations

from pathlib import Path

from ...core.database.qq import User
from ...core.image.info import song_play_data
from ...core.io_image import default_out_path, save_image
from ...core.merge.models import Song
from ...result import FeatureResult


def draw_play_score(
    user: User,
    song: Song,
    play_result,
    *,
    out: Path | str | None = None,
) -> FeatureResult:
    path = default_out_path(f"minfo_{user.qqid}_{song.song_id}", out)
    image = song_play_data(
        user.service, user.theme, song=song, play_result=play_result
    )
    saved = save_image(image, path)
    return FeatureResult(text=f"{song.song_id} {song.song_name}", image_path=saved)
