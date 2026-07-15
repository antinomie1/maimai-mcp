from ..exceptions import HTTPError, TokenError, UserNotFoundError


class LXNSParamsError(HTTPError):
    """参数错误"""

    def __init__(self, message: str = "参数错误", *, body: object | None = None):
        self.message = message
        self.body = body
        super().__init__(message)


class LXNSPermissionDeniedError(HTTPError):
    """权限不足"""

    def __init__(self, message: str = "权限不足"):
        self.message = message
        super().__init__(message)


class LXNSNotFoundError(HTTPError):
    """未找到资源"""

    def __init__(self, message: str = "未找到资源"):
        self.message = message
        super().__init__(message)


class LXNSTooManyRequestsError(HTTPError):
    """过多的请求"""

    def __init__(self, message: str = "请求过于频繁"):
        self.message = message
        super().__init__(message)


class LXNSOAuthError(HTTPError):
    """OAuth2错误"""

    def __init__(self, message: str = "OAuth2 错误"):
        self.message = message
        super().__init__(message)


class LXNSTokenError(TokenError):
    """用户Token错误"""

    def __init__(self, message: str = "Token 错误或失效"):
        self.message = message
        super().__init__(message)


class LXNSUserNotFoundError(UserNotFoundError):
    """未找到用户"""
