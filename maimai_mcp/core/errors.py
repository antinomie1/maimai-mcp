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
    未在水鱼查分器找到此玩家，请确保此玩家的用户名和查分器中的用户名相同。
    如未绑定，请前往查分器官网进行绑定。
    https://www.diving-fish.com/maimaidx/prober/
""").strip()

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
        return "查询的用户不存在。"
    if isinstance(exc, DivingFishUserDisabledQueryError):
        return "该用户禁止了其他人获取数据或未同意用户协议。"
    if isinstance(
        exc,
        (DivingFishTokenDisableError, DivingFishTokenNotFoundError, DivingFishTokenError),
    ):
        log.error("水鱼开发者Token异常，请自行检查。")
        return "请检查水鱼查分器相关配置，暂时无法查询。"
    if isinstance(exc, LXNSTokenError):
        return "落雪查分器授权错误，请尝试重新绑定授权。"
    if isinstance(exc, LXNSPermissionDeniedError):
        return "使用落雪查分器的权限不足，请检查相关配置。"
    if isinstance(exc, LXNSNotFoundError):
        return "未找到落雪查分器相关资源，请检查相关配置。"
    if isinstance(exc, LXNSTooManyRequestsError):
        return "使用落雪查分器的请求次数过多，请稍后再试。"
    if isinstance(exc, LXNSParamsError):
        log.error(f"请求参数错误。\n{traceback.format_exc()}")
        return "使用落雪查分器请求时发生错误，请检查相关配置。"
    if isinstance(exc, LXNSOAuthError):
        return "落雪查分器授权错误，请重试，依旧错误请重新绑定授权。"
    if isinstance(exc, MusicNotPlayError):
        return "您未游玩过曲目。"
    if isinstance(exc, NotMusicRecommendationError):
        return "没有乐曲推荐呢。可能是您太强了。"
    if isinstance(exc, PermissionError):
        return str(exc) or "权限不足。"
    if isinstance(exc, ValueError) and str(exc):
        return str(exc)
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
