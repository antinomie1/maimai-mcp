"""Rise-score recommendation logic."""

from __future__ import annotations

import math
import random

from ...constants import (
    ACHIEVEMENT_LIST,
    ALL_VERSION,
    DX_CN_VERSION,
    LEVEL_INDEX_MAP,
)
from ..clients.lxns.models import SongType
from ..database.qq import User
from ..merge.models import PlayedResult, RiseResult, Song
from ..service import mai
from ..utils.calc import compute_rating
from .player import get_best50

RISE_ACHIEVEMENT_LIST = ACHIEVEMENT_LIST[-4:]


def get_rise_score_list(
    old_records: dict[tuple[int, int], PlayedResult],
    type: SongType,
    play_result: list[PlayedResult],
    level: str | None = None,
    score: int | None = None,
) -> tuple[list[RiseResult], int]:
    if not play_result:
        return [], 0

    lowest_ra = play_result[-1].rating
    lowest_level = play_result[-1].level
    lowest_level_index = LEVEL_INDEX_MAP[lowest_level]
    new_level_index = LEVEL_INDEX_MAP[level] if level else lowest_level_index

    if lowest_level_index > new_level_index:
        return [], 0

    target_rise = score or 1
    ignored_song_ids = {p.song_id for p in play_result if p.achievements >= 100.5}
    max_ra_coefficient = ACHIEVEMENT_LIST[-1] / 100 * 22.4
    min_ds = math.ceil((lowest_ra + target_rise) / max_ra_coefficient * 10) / 10
    ds = None if level is not None else (min_ds, min_ds + 1)
    new_version = list(DX_CN_VERSION.values())[-1][-1]
    version = (
        new_version
        if type == SongType.DX
        else ALL_VERSION[: ALL_VERSION.index(new_version)]
    )
    songs = mai.total_list.filter(
        level=level, level_value=ds, version_str=version, all_diff=False
    )
    rise_result: list[RiseResult] = []

    for song in songs:
        song_id = song.song_id
        if song_id >= 100000 or song_id in ignored_song_ids:
            continue
        for diff in song.difficulties:
            if level and LEVEL_INDEX_MAP[diff.level] > new_level_index:
                continue
            old_result = old_records.get((song_id, diff.level_index))
            old_ra = max(old_result.rating, lowest_ra) if old_result else 0
            for achievements in RISE_ACHIEVEMENT_LIST:
                new_ra, new_rate = compute_rating(
                    diff.level_value, achievements, israte=True
                )
                if old_result is None:
                    if new_ra <= lowest_ra:
                        continue
                    rise_result.append(
                        RiseResult(
                            song_id=song_id,
                            song_name=song.song_name,
                            level_index=diff.level_index,
                            type=song.type,
                            rating=new_ra,
                            achievements=achievements,
                            rate=new_rate.lower(),
                            level_value=diff.level_value,
                        )
                    )
                    break
                if new_ra - old_ra < target_rise:
                    continue
                rise_result.append(
                    RiseResult(
                        song_id=song_id,
                        song_name=song.song_name,
                        level_index=diff.level_index,
                        type=song.type,
                        rating=new_ra,
                        achievements=achievements,
                        rate=new_rate.lower(),
                        level_value=diff.level_value,
                        old_rating=old_result.rating,
                        old_achievements=old_result.achievements,
                        old_rate=old_result.rate.value if old_result.rate else "D",
                    )
                )
                break

    sampled = random.sample(rise_result, min(len(rise_result), 5))
    sampled.sort(key=lambda x: x.level_value, reverse=True)
    return sampled, lowest_ra


async def get_mai_what_song(
    user: User, *, username: str | None = None
) -> Song | None:
    _, best50 = await get_best50(user, username=username)
    r = random.randint(0, 1)
    _ra = 0
    ignore: list[int] = []
    if r == 0:
        if sd := best50.sd:
            ignore = [m.song_id for m in sd if m.achievements < 100.5]
            _ra = sd[-1].rating
    else:
        if dx := best50.dx:
            ignore = [m.song_id for m in dx if m.achievements < 100.5]
            _ra = dx[-1].rating
    if _ra != 0:
        ds = round(_ra / 22.4, 1)
        music_list = mai.total_list.filter(level_value=(ds, ds + 1))
        music_list = [m for m in music_list if int(m.song_id) not in ignore]
        if not music_list:
            return None
        return random.choice(music_list)
    return None
