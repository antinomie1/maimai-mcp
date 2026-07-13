from typing import overload

from ...config import log
from ..clients.divingfish.models import PlayInfoDefault, PlayInfoDev
from ..clients.lxns.models import Score, SongType
from ..service import mai
from .models import NotPlayedResult, PlayedResult, Song


def _lxns_song_id(v: Score) -> int:
    if v.type == SongType.STANDARD:
        return v.id
    if v.type == SongType.DX:
        return v.id + 10000
    return v.id


def df_format_result(
    v: PlayInfoDefault | PlayInfoDev, level_value: float = 0
) -> PlayedResult:
    ds = v.ds if level_value == 0 else level_value
    return PlayedResult(
        song_id=v.song_id,
        song_name=v.title,
        level=v.level,
        level_index=v.level_index,
        level_value=ds,
        type=v.type,
        rating=v.ra,
        achievements=v.achievements,
        fc=v.fc,
        fs=v.fs,
        rate=v.rate,
        dx_score=v.dxScore,
    )


@overload
def df_to_playresult(data: list[Score]) -> list[PlayedResult]: ...
@overload
def df_to_playresult(
    data: list[Score], *, song: Song | None = None
) -> list[PlayedResult | NotPlayedResult]: ...
def df_to_playresult(
    data: list[PlayInfoDefault] | list[PlayInfoDev], *, song: Song | None = None
) -> list[PlayedResult | NotPlayedResult]:
    if song:
        r = [
            NotPlayedResult(
                level_value=v.level_value,
                song_id=song.song_id,
                level_index=v.level_index,
            )
            for v in song.difficulties
        ]
    else:
        r = []

    for v in data:
        if song:
            r[v.level_index] = df_format_result(v, r[v.level_index].level_value)
        else:
            r.append(df_format_result(v))

    return r


def lxns_format_result(
    v: Score, *, level_value: float | None = None
) -> PlayedResult | None:
    """Convert one Lxns score. Returns None if chart is missing from local catalog."""
    song_id = _lxns_song_id(v)
    li = int(v.level_index)
    ds = level_value
    if ds is None:
        ds = mai.resolve_level_value(song_id, li)
    if ds is None:
        log.warning(
            f"曲库缺少谱面 {song_id}-{li}（{v.song_name}），已跳过该成绩"
        )
        return None
    rating = int(v.dx_rating) if v.dx_rating is not None else 0
    return PlayedResult(
        song_id=song_id,
        song_name=v.song_name,
        level=v.level,
        level_index=v.level_index,
        type=v.type,
        rating=rating,
        achievements=v.achievements,
        fc=v.fc,
        fs=v.fs,
        rate=v.rate,
        dx_score=v.dx_score,
        level_value=ds,
    )


@overload
def lxns_to_playresult(data: list[Score]) -> list[PlayedResult]: ...
@overload
def lxns_to_playresult(
    data: list[Score], *, song: Song | None = None
) -> list[PlayedResult | NotPlayedResult]: ...
def lxns_to_playresult(
    data: list[Score], *, song: Song | None = None
) -> list[PlayedResult | NotPlayedResult]:
    if song:
        r: list[PlayedResult | NotPlayedResult] = [
            NotPlayedResult(
                level_value=v.level_value,
                song_id=song.song_id,
                level_index=v.level_index,
            )
            for v in song.difficulties
        ]
    else:
        r = []
    for v in data:
        override_lv: float | None = None
        if song is not None:
            try:
                override_lv = r[v.level_index].level_value
            except (IndexError, TypeError):
                override_lv = None
        result = lxns_format_result(v, level_value=override_lv)
        if result is None:
            continue
        if song:
            r[v.level_index] = result
        else:
            r.append(result)
    return r
