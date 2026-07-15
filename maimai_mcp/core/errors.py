"""Business errors and user-facing message mapping."""

from __future__ import annotations

import traceback
from functools import wraps
from textwrap import dedent
from typing import Any, Awaitable, Callable, TypeVar

from ..config import log
from ..result import FeatureResult
from .clients.divingfish.exceptions import (
    DivingFishTokenDisableError,
    DivingFishTokenError,
    DivingFishTokenNotFoundError,
    DivingFishUserDisabledQueryError,
    DivingFishUserNotFoundError,
)
from .clients.exceptions import (
    MusicNotPlayError,
    NotMusicRecommendationError,
    UserNotExistsError,
)
from .clients.lxns.exceptions import (
    LXNSNotFoundError,
    LXNSOAuthError,
    LXNSParamsError,
    LXNSPermissionDeniedError,
    LXNSTokenError,
    LXNSTooManyRequestsError,
)

NOTFOUNDUSER = dedent("""
    未在水鱼查分器找到此玩家，请确认用户名与查分器绑定一致。
    若尚未绑定，请到水鱼查分器官网完成绑定：
    https://www.diving-fish.com/maimaidx/prober/
""").strip()

_RESOURCE_DOWNLOAD = (
    "资源包见 README「下载静态资源」"
    "（Cloudreve / OneDrive），解压后将 STATIC_PATH 指向其中的 static 目录。"
)

T = TypeVar("T")


class MaimaiError(Exception):
    """Base library error with stable ``code`` and user-facing ``message``."""

    def __init__(
        self,
        message: str,
        *,
        code: str = "maimai_error",
        cause: BaseException | None = None,
    ) -> None:
        super().__init__(message)
        self.message = message
        self.code = code
        self.cause = cause
        if cause is not None:
            self.__cause__ = cause


class ValidationError(MaimaiError):
    def __init__(self, message: str) -> None:
        super().__init__(message, code="validation_error")


class NotFoundError(MaimaiError):
    def __init__(self, message: str) -> None:
        super().__init__(message, code="not_found")


class AuthError(MaimaiError):
    def __init__(self, message: str, *, code: str = "auth_error") -> None:
        super().__init__(message, code=code)


def exception_to_message(exc: BaseException) -> str:
    """Map known API / domain exceptions to user-facing Chinese messages."""
    if isinstance(exc, MaimaiError):
        return exc.message
    if isinstance(exc, DivingFishUserNotFoundError):
        return NOTFOUNDUSER
    if isinstance(exc, UserNotExistsError):
        return (
            "查询的用户不存在。请确认已传入正确的 qq 或水鱼 username，"
            "并在查分器完成绑定：https://www.diving-fish.com/maimaidx/prober/"
        )
    if isinstance(exc, DivingFishUserDisabledQueryError):
        return (
            "该用户禁止了其他人获取数据，或未同意用户协议。"
            "请玩家在水鱼查分器个人设置中开放查询权限："
            "https://www.diving-fish.com/maimaidx/prober/"
        )
    if isinstance(
        exc,
        (DivingFishTokenDisableError, DivingFishTokenNotFoundError, DivingFishTokenError),
    ):
        log.error("水鱼开发者Token异常，请自行检查。")
        return (
            "水鱼开发者 Token 无效或未配置，暂时无法查询。"
            "请在环境变量 DIVINGFISH_TOKEN 中填写开发者 Token"
            "（水鱼查分器开发者设置；注意不是个人成绩 Import-Token）。"
        )
    if isinstance(exc, LXNSTokenError):
        return (
            "落雪查分器授权错误。请用 maimai_user_bind_lxns 重新获取授权链接并提交 code，"
            "或 CLI：maimai user bind --qq <QQ>。"
        )
    if isinstance(exc, LXNSPermissionDeniedError):
        return (
            "落雪查分器权限不足。请重新绑定并确认 OAuth 权限包含所需 scope"
            "（上传成绩需 write_player）。"
            "工具：maimai_user_bind_lxns；CLI：maimai user bind。"
        )
    if isinstance(exc, LXNSNotFoundError):
        return (
            "未在落雪查分器找到对应资源。请确认已绑定落雪且玩家存在；"
            "绑定：maimai_user_bind_lxns。"
        )
    if isinstance(exc, LXNSTooManyRequestsError):
        return "落雪查分器请求过于频繁，请稍后再试。"
    if isinstance(exc, LXNSParamsError):
        log.error(f"请求参数错误。\n{traceback.format_exc()}")
        return (
            "落雪查分器请求参数错误。请检查绑定状态与请求内容；"
            "必要时重新 maimai_user_bind_lxns。"
        )
    if isinstance(exc, LXNSOAuthError):
        return (
            "落雪 OAuth 授权失败。请重试 maimai_user_bind_lxns 获取新链接并提交 code；"
            "仍失败请检查 LX_CLIENT_ID / LX_CLIENT_SECRET / REDIRECT_URI。"
        )
    if isinstance(exc, MusicNotPlayError):
        return "您未游玩过该曲目。"
    if isinstance(exc, NotMusicRecommendationError):
        return "没有乐曲推荐呢。可能是您太强了。"
    if isinstance(exc, PermissionError):
        return str(exc) or "权限不足。"
    if isinstance(exc, ValueError) and str(exc):
        return str(exc)
    if isinstance(exc, KeyError):
        key = exc.args[0] if exc.args else "?"
        log.error(f"发生错误: {traceback.format_exc()}")
        return (
            f"曲库缺少谱面数据（{key}）。"
            "请执行 CLI：maimai update music（或 MCP：maimai_update_catalog）；"
            "若仍失败请反馈该 key。"
        )
    if isinstance(exc, FileNotFoundError):
        log.error(f"发生错误: {traceback.format_exc()}")
        missing = getattr(exc, "filename", None) or (
            exc.args[0] if exc.args else str(exc)
        )
        return (
            f"缺少资源文件：{missing}。"
            "请确认 STATIC_PATH 指向完整 static 目录（含 mai/、font/ 等）。"
            f"{_RESOURCE_DOWNLOAD}"
            " 表图为空时可运行：maimai update tables。"
        )
    log.error(f"发生错误: {traceback.format_exc()}")
    return f"发生未知错误：{type(exc).__name__}"

def exception_to_code(exc: BaseException) -> str:
    if isinstance(exc, MaimaiError):
        return exc.code
    if isinstance(exc, DivingFishUserNotFoundError):
        return "df_user_not_found"
    if isinstance(exc, UserNotExistsError):
        return "user_not_exists"
    if isinstance(exc, DivingFishUserDisabledQueryError):
        return "df_user_disabled"
    if isinstance(
        exc,
        (DivingFishTokenDisableError, DivingFishTokenNotFoundError, DivingFishTokenError),
    ):
        return "df_token_error"
    if isinstance(exc, (LXNSTokenError, LXNSOAuthError)):
        return "lxns_auth_error"
    if isinstance(exc, LXNSPermissionDeniedError):
        return "lxns_permission"
    if isinstance(exc, LXNSNotFoundError):
        return "lxns_not_found"
    if isinstance(exc, LXNSTooManyRequestsError):
        return "lxns_rate_limit"
    if isinstance(exc, LXNSParamsError):
        return "lxns_params"
    if isinstance(exc, MusicNotPlayError):
        return "music_not_played"
    if isinstance(exc, NotMusicRecommendationError):
        return "no_recommendation"
    if isinstance(exc, PermissionError):
        return "permission_error"
    if isinstance(exc, ValidationError):
        return "validation_error"
    return type(exc).__name__


def wrap_exception(exc: BaseException) -> MaimaiError:
    """Normalize any exception into MaimaiError (idempotent)."""
    if isinstance(exc, MaimaiError):
        return exc
    return MaimaiError(
        exception_to_message(exc),
        code=exception_to_code(exc),
        cause=exc,
    )


def handle_errors(func: Callable[..., Any]) -> Callable[..., Any]:
    """Decorator: re-raise as MaimaiError with mapped message (preserves cause)."""

    @wraps(func)
    async def wrapper(*args: Any, **kwargs: Any) -> Any:
        try:
            return await func(*args, **kwargs)
        except MaimaiError:
            raise
        except Exception as e:
            raise wrap_exception(e) from e

    return wrapper


async def as_result(coro: Awaitable[Any]) -> FeatureResult:
    """Await a coroutine and always return FeatureResult (never raises)."""
    try:
        value = await coro
        if isinstance(value, FeatureResult):
            return value
        return FeatureResult.success(data=value)
    except Exception as e:
        err = wrap_exception(e)
        return FeatureResult.failure(err.message, code=err.code)
