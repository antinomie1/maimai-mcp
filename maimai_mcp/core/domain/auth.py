"""Lxns OAuth / token helpers."""

from __future__ import annotations

from ..clients.lxns.client import LxnsAPI, OAuth2
from ..clients.lxns.models import BaseToken, OAuth2Token
from ..database.qq import User, update_user


def get_token(user: User) -> BaseToken:
    return BaseToken(access_token=user.access_token, refresh_token=user.refresh_token)


async def get_friend_code(qqid: int, token: OAuth2Token | BaseToken) -> int:
    api = LxnsAPI(qqid, token=token)
    player = await api.player()
    return player.friend_code


async def bind_lxns(user: User, code: str) -> str:
    oauth = OAuth2()
    token = await oauth.fetch_token(code)
    friend_code = await get_friend_code(user.qqid, token)
    update = await update_user(user.qqid, friend_code=friend_code, token=token)
    if update is None:
        return "数据库错误。"
    return "授权完成。"
