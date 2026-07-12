"""Player score queries (b50, records, single song)."""

from __future__ import annotations

from ..clients.divingfish.client import DivingFishAPI
from ..clients.exceptions import MusicNotPlayError
from ..clients.lxns.client import LxnsAPI
from ..clients.lxns.models import SongType
from ..database.qq import User
from ..errors import ValidationError
from ..merge.models import Best50, PlayedResult, Player, ServiceName, Song
from ..merge.play_result import df_to_playresult, lxns_to_playresult
from ..merge.player import df_to_best50, df_to_player, lxns_to_best50
from ..user import PlayerRef
from .auth import get_token


def _df_api(user: User, username: str | None = None) -> DivingFishAPI:
    if username:
        return DivingFishAPI(username=username)
    if user.qqid:
        return DivingFishAPI(qqid=user.qqid)
    raise ValidationError("水鱼查询需要 --qq 或 --username")


async def get_best50(
    user: User,
    *,
    username: str | None = None,
    all_perfect: bool = False,
) -> tuple[Player, Best50]:
    # Username always goes through Diving-Fish public player API
    if username or user.service == ServiceName.DIVINGFISH:
        api = _df_api(user, username)
        userinfo = await api.query_user_b50()
        player = df_to_player(userinfo)
        best50 = df_to_best50(userinfo)
    elif user.service == ServiceName.LXNS:
        if all_perfect is False and username:
            raise ValidationError("落雪数据源不支持用水鱼用户名查询")
        token = get_token(user)
        api = LxnsAPI(user.qqid, token)
        player = await api.player()
        if all_perfect:
            obj = await api.ap50(player.friend_code)
        else:
            obj = await api.best50()
        best50 = lxns_to_best50(obj)
    else:
        raise ValueError(f"unknown service: {user.service}")
    return player, best50


async def get_best50_ref(
    ref: PlayerRef, *, all_perfect: bool = False
) -> tuple[Player, Best50]:
    return await get_best50(
        ref.user, username=ref.username, all_perfect=all_perfect
    )


async def get_player_result(
    user: User,
    version: list[str] | None = None,
    *,
    username: str | None = None,
) -> list[PlayedResult]:
    if username or user.service == ServiceName.DIVINGFISH:
        api = _df_api(user, username)
        if version is not None:
            data = await api.query_user_plate(version)
        else:
            result = await api.query_user_get_dev()
            data = result.records
        return df_to_playresult(data)
    if user.service == ServiceName.LXNS:
        if username:
            raise ValidationError("落雪数据源不支持用水鱼用户名查询，请使用 --qq")
        token = get_token(user)
        api = LxnsAPI(user.qqid, token)
        data = await api.all_best()
        return lxns_to_playresult(data)
    raise ValueError(f"unknown service: {user.service}")


async def get_player_result_ref(
    ref: PlayerRef, version: list[str] | None = None
) -> list[PlayedResult]:
    return await get_player_result(ref.user, version, username=ref.username)


async def get_play_score(
    user: User, song: Song, *, username: str | None = None
) -> list[PlayedResult]:
    """Fetch single-song play results (minfo)."""
    if username or user.service == ServiceName.DIVINGFISH:
        api = _df_api(user, username)
        data = await api.query_user_post_dev(song_id=song.song_id)
        if not data:
            raise MusicNotPlayError
        return df_to_playresult(data, song=song)
    if user.service == ServiceName.LXNS:
        if username:
            raise ValidationError("落雪数据源不支持用水鱼用户名查询，请使用 --qq")
        token = get_token(user)
        api = LxnsAPI(user.qqid, token)
        if song.song_id < 10000:
            song_type = SongType.STANDARD
        elif song.song_id < 100000:
            song_type = SongType.DX
        else:
            song_type = SongType.UTAGE
        song_id = song.song_id % 10000
        data = await api.song_bests(song_id, song_type)
        if not data:
            raise MusicNotPlayError
        return lxns_to_playresult(data, song=song)
    raise ValueError(f"unknown service: {user.service}")


async def get_play_score_ref(ref: PlayerRef, song: Song) -> list[PlayedResult]:
    return await get_play_score(ref.user, song, username=ref.username)
