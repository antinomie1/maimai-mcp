"""User / player identity resolution (QQ and/or Diving-Fish username)."""

from __future__ import annotations

from dataclasses import dataclass

from ..config import maiconfig
from .clients.exceptions import UserNotBindError
from .database.qq import User, get_user, update_user
from .errors import ValidationError
from .merge.models import ServiceName, Theme

AUTHORIZE_ERROR = (
    f"您尚未授权「{maiconfig.bot_name}」访问您的落雪查分器数据，"
    "请先运行 bind_lxns 完成绑定。"
)


@dataclass
class PlayerRef:
    """Resolved identity for score queries.

    - ``user``: local prefs (theme / default service / tokens)
    - ``username``: if set, Diving-Fish APIs query by username (not QQ)
    """

    user: User
    username: str | None = None

    @property
    def use_username(self) -> bool:
        return bool(self.username)

    @property
    def label(self) -> str:
        if self.username:
            return self.username
        return str(self.user.qqid)


def ephemeral_user(*, theme: Theme = Theme.CIRCLE) -> User:
    """In-memory user for username-only queries (not written to DB)."""
    return User(
        qqid=0,
        service=ServiceName.DIVINGFISH,
        theme=theme,
    )


async def resolve_user(
    qq: int | None = None,
    *,
    auto_create: bool = True,
    require_lxns_auth: bool = False,
) -> User:
    """Resolve local user row by QQ (falls back to DEFAULT_QQ)."""
    qqid = qq if qq is not None else maiconfig.default_qq
    if qqid is None:
        raise ValidationError(
            "未指定 QQ，请传入 --qq 或在 .env 中配置 DEFAULT_QQ"
            "（也可使用 --username 水鱼用户名查询）"
        )

    try:
        user = await get_user(qqid)
    except UserNotBindError:
        if not auto_create:
            raise
        user = await update_user(qqid)

    if require_lxns_auth and user.service == ServiceName.LXNS:
        if user.access_token is None and user.refresh_token is None:
            raise PermissionError(AUTHORIZE_ERROR)

    return user


async def resolve_player(
    qq: int | None = None,
    username: str | None = None,
    *,
    auto_create: bool = True,
    require_lxns_auth: bool = False,
    optional: bool = False,
) -> PlayerRef | None:
    """
    Resolve player for score queries.

    - ``--username`` → 水鱼用户名查询（可同时 ``--qq`` 只取本地主题）
    - 仅 ``--qq`` / DEFAULT_QQ → 本地用户 + 绑定数据源
    - 仅 DEFAULT_USERNAME → 水鱼用户名
    - ``optional=True`` 时两者都没有则返回 None
    """
    uname = (username or "").strip() or None
    if uname is None and qq is None:
        uname = (maiconfig.default_username or "").strip() or None

    if uname:
        user = ephemeral_user()
        if qq is not None or maiconfig.default_qq is not None:
            try:
                user = await resolve_user(
                    qq, auto_create=auto_create, require_lxns_auth=False
                )
            except Exception:
                user = ephemeral_user()
        return PlayerRef(user=user, username=uname)

    try:
        user = await resolve_user(
            qq, auto_create=auto_create, require_lxns_auth=require_lxns_auth
        )
        return PlayerRef(user=user, username=None)
    except ValidationError:
        if optional:
            return None
        raise
