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
    """Resolve local user row by QQ."""
    if qq is None:
        raise ValidationError(
            "未指定玩家 QQ。请传 qq（发送者/被查对象，不是群号）；也可用 username。"
        )
    qqid = qq

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


def apply_source_override(user: User, source: str | None) -> User:
    """Return user with service overridden for this query only (no DB write)."""
    if not source or not str(source).strip():
        return user
    svc = ServiceName.get_by_index(str(source).strip())
    if svc is None:
        raise ValidationError(
            f"未知数据源：{source}。可选 水鱼/divingfish 或 落雪/lxns。"
        )
    if user.service == svc:
        return user
    return user.model_copy(update={"service": svc})


async def resolve_player(
    qq: int | None = None,
    username: str | None = None,
    *,
    auto_create: bool = True,
    require_lxns_auth: bool = False,
    optional: bool = False,
    source: str | None = None,
) -> PlayerRef | None:
    """
    Resolve player for score queries.

    - ``username`` → 水鱼用户名查询（可同时 ``qq`` 只取本地主题）
    - 仅 ``qq`` → 本地用户 + 其设定中的数据源（默认水鱼）
    - ``source`` → 本次覆盖数据源（不写库）；Agent 根据用户意图传入
    - ``optional=True`` 时两者都没有则返回 None
    """
    override: ServiceName | None = None
    if source and str(source).strip():
        override = ServiceName.get_by_index(str(source).strip())
        if override is None:
            raise ValidationError(
                f"未知数据源：{source}。可选 水鱼/divingfish 或 落雪/lxns。"
            )
        if override == ServiceName.DIVINGFISH:
            require_lxns_auth = False

    uname = (username or "").strip() or None

    if uname:
        user = ephemeral_user()
        if qq is not None:
            try:
                user = await resolve_user(
                    qq, auto_create=auto_create, require_lxns_auth=False
                )
            except Exception:
                user = ephemeral_user()
        if override is not None:
            user = apply_source_override(user, source)
        return PlayerRef(user=user, username=uname)

    try:
        user = await resolve_user(
            qq, auto_create=auto_create, require_lxns_auth=require_lxns_auth
        )
        if override is not None:
            user = apply_source_override(user, source)
            if (
                user.service == ServiceName.LXNS
                and user.access_token is None
                and user.refresh_token is None
            ):
                raise PermissionError(AUTHORIZE_ERROR)
        return PlayerRef(user=user, username=None)
    except ValidationError:
        if optional:
            return None
        raise
