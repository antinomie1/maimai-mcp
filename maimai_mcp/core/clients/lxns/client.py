from httpx import Response

from ....config import lxnsconfig
from ...database.qq import update_user
from ..exceptions import UnknownError
from ..http import ApiClient
from .exceptions import (
    LXNSNotFoundError,
    LXNSOAuthError,
    LXNSParamsError,
    LXNSPermissionDeniedError,
    LXNSTokenError,
    LXNSTooManyRequestsError,
)
from .models import (
    Aliases,
    APIResult,
    BaseToken,
    Best50,
    Collection,
    LevelIndex,
    OAuth2Token,
    Player,
    RatingTrend,
    Score,
    Song,
    Songs,
    SongType,
)


class OAuth2(ApiClient):
    def __init__(self):
        super().__init__(
            base_url="https://maimai.lxns.net",
        )
        self.client_id = lxnsconfig.lx_client_id
        self.client_secret = lxnsconfig.lx_client_secret
        self.redirect_uri = lxnsconfig.redirect_uri
        self.token: OAuth2Token | BaseToken | None = None

    async def fetch_token(self, code: str) -> OAuth2Token:
        """通过授权码获取 `access_token`"""
        json = {
            "client_id": self.client_id,
            "client_secret": self.client_secret,
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": self.redirect_uri,
        }
        result = await self._request_data("POST", "/api/v0/oauth/token", json=json)
        self.token = OAuth2Token.model_validate(result)
        return self.token

    async def refresh_token(self) -> OAuth2Token:
        if not self.token:
            raise LXNSTokenError

        json = {
            "client_id": self.client_id,
            "client_secret": self.client_secret,
            "grant_type": "refresh_token",
            "refresh_token": self.token.refresh_token,
        }
        result = await self._request_data("POST", "/api/v0/oauth/token", json=json)
        self.token = OAuth2Token.model_validate(result)
        return self.token

    async def _request_data(self, method: str, endpoint: str, **kwargs) -> dict:
        return await self._request(method, endpoint, **kwargs)

    def _handle_error(self, resp: Response) -> None:
        if resp.status_code == 200:
            return
        elif resp.status_code == 401:
            raise LXNSTokenError
        else:
            raise LXNSOAuthError


class LxnsClient(ApiClient):
    def __init__(
        self,
        *,
        base_url: str,
        headers: dict[str, str],
        user_id: str,
        token: OAuth2Token | BaseToken | None = None,
    ):
        super().__init__(base_url=base_url, headers=headers)
        self.user_id = user_id
        self._token = token
        self._friend_code: int | None = None

    async def _on_unauthorized(self) -> bool:
        """
        刷新 token
        """
        if not self._token:
            return False

        oauth = OAuth2()
        oauth.token = self._token

        try:
            new_token = await oauth.refresh_token()
            await update_user(self.user_id, token=new_token)
        except Exception:
            self._token = None
            return False

        self._token = new_token
        self.headers["Authorization"] = (
            f"{new_token.token_type} {new_token.access_token}"
        )

        self._friend_code = None

        return True

    def _error_detail(self, resp: Response) -> str:
        try:
            data = resp.json()
            if isinstance(data, dict):
                msg = data.get("message") or data.get("msg") or data.get("error")
                code = data.get("code")
                if msg and code is not None:
                    return f"code={code} {msg}"
                if msg:
                    return str(msg)
                return str(data)[:500]
        except Exception:
            pass
        text = (resp.text or "").strip()
        return text[:500] if text else f"HTTP {resp.status_code}"

    def _handle_error(self, resp: Response):
        detail = self._error_detail(resp)
        match resp.status_code:
            case 200:
                return
            case 400:
                raise LXNSParamsError(detail, body=detail)
            case 401:
                self._friend_code = None
                raise LXNSOAuthError(detail)
            case 403:
                raise LXNSPermissionDeniedError(detail)
            case 404:
                raise LXNSNotFoundError(detail)
            case 429:
                raise LXNSTooManyRequestsError(detail)
            case _:
                raise UnknownError(detail)

    async def _request_data(self, method: str, endpoint: str, **kwargs) -> APIResult:
        data = await self._request(method, endpoint, **kwargs)
        return APIResult.model_validate(data)

    async def _request_base_data(self, method: str, endpoint: str, **kwargs) -> dict:
        return await self._request(method, endpoint, **kwargs)


class LxnsAPI:
    def __init__(
        self, user_id: str | None = None, token: OAuth2Token | BaseToken | None = None
    ):
        self._oauth_client = (
            LxnsClient(
                base_url="https://maimai.lxns.net/api/v0/user/maimai/player",
                headers={"Authorization": f"Bearer {token.access_token}"},
                user_id=user_id,
                token=token,
            )
            if token
            else None
        )

        self._dev_client = LxnsClient(
            base_url="https://maimai.lxns.net/api/v0/maimai",
            headers={"Authorization": lxnsconfig.lxns_dev_token},
            user_id=user_id,
            token=None,
        )

    async def music_data(self) -> Songs:
        """获取曲目数据"""
        result = await self._dev_client._request_base_data(
            "GET", "/song/list", params={"notes": True}
        )
        return Songs.model_validate(result)

    async def single_music_data(self, song_id: str) -> Song:
        """获取单个曲目数据"""
        result = await self._dev_client._request_base_data("GET", f"/song/{song_id}")
        return Song.model_validate(result)

    async def music_alias_data(self) -> Aliases:
        """获取别名列表"""
        result = await self._dev_client._request_base_data("GET", "/alias/list")
        return Aliases.model_validate(result)

    async def player(
        self, *, friend_code: int | None = None, qq: int | None = None
    ) -> Player:
        """获取玩家信息"""

        if friend_code is not None:
            result = await self._dev_client._request_data(
                "GET", f"/player/{friend_code}"
            )
        elif qq is not None:
            result = await self._dev_client._request_data("GET", f"/player/qq/{qq}")
        else:
            result = await self._oauth_client._request_data("GET", "")

        return Player.model_validate(result.data)

    async def single_best(
        self, song_id: int, level_index: LevelIndex, song_type: SongType
    ) -> Score:
        """
        获取曲目指定难度成绩
        """
        params = {
            "song_id": song_id,
            "level_index": level_index.value,
            "song_type": song_type.value,
        }
        result = await self._oauth_client._request_data("GET", "/best", params=params)
        return Score.model_validate(result.data)

    async def best50(self) -> Best50:
        """
        获取 `b50`
        """
        result = await self._oauth_client._request_data("GET", "/bests")
        return Best50.model_validate(result.data)

    async def ap50(self, friend_code: int) -> Best50:
        """
        获取 `ap50`
        """
        result = await self._dev_client._request_data(
            "GET", f"/player/{friend_code}/bests/ap"
        )
        return Best50.model_validate(result.data)

    async def song_bests(self, song_id: int, song_type: SongType) -> list[Score]:
        """
        获取指定曲目所有难度成绩
        """
        params = {"song_id": song_id, "song_type": song_type.value}
        result = await self._oauth_client._request_data("GET", "/bests", params=params)
        return [Score.model_validate(s) for s in result.data]

    async def recent50(self) -> list[Score]:
        """
        获取最近游玩的 50 个成绩
        """
        result = await self._oauth_client._request_data("GET", "/recents")
        return [Score.model_validate(s) for s in result.data]

    async def all_best(self) -> list[Score]:
        """
        获取所有成绩
        """
        result = await self._oauth_client._request_data("GET", "/scores")
        return [Score.model_validate(s) for s in result.data]

    async def upload_scores(self, scores: list[dict]) -> APIResult:
        """个人 API：上传成绩 POST /api/v0/user/maimai/player/scores

        需要用户 OAuth（scope 含 write_player）。

        **默认整包一次 POST**（不拆批、不二分）。坏数据应在 convert 阶段滤掉。
        仅当 429 时退避后整包重试一次。
        """
        import asyncio

        if not self._oauth_client:
            raise LXNSTokenError("未绑定落雪 OAuth，无法上传成绩")
        n = len(scores)
        if n == 0:
            return APIResult(
                success=True, code=0, message="empty", data={"uploaded": 0, "total": 0}
            )

        async def post_all() -> None:
            await self._oauth_client._request_data(
                "POST", "/scores", json={"scores": scores}
            )

        try:
            await post_all()
        except LXNSTooManyRequestsError:
            await asyncio.sleep(3.0)
            try:
                await post_all()
            except LXNSTooManyRequestsError as exc:
                detail = getattr(exc, "message", None) or str(exc) or "too many requests"
                raise LXNSTooManyRequestsError(
                    f"{detail}（请稍后再试；本上传仅 1 次请求）"
                ) from exc

        return APIResult(
            success=True,
            code=0,
            message=f"uploaded={n}/{n} requests=1",
            data={"uploaded": n, "total": n, "requests": 1},
        )

    async def heatmap(self) -> dict[str, int]:
        """
        获取玩家上传热力图
        """
        result = await self._oauth_client._request_data("GET", "/heatmap")
        return result.data

    async def trend(self, version: int) -> list[RatingTrend]:
        """
        获取玩家 DX Rating 趋势
        """
        params = {"version": version}
        result = await self._oauth_client._request_data("GET", "/trend", params=params)
        return [RatingTrend.model_validate(s) for s in result.data]

    async def history(
        self, song_id: int, song_type: SongType, level_index: LevelIndex
    ) -> list[Score]:
        """
        获取玩家成绩游玩历史记录
        """
        params = {
            "song_id": song_id,
            "song_type": song_type.value,
            "level_index": level_index.value,
        }
        result = await self._oauth_client._request_data(
            "GET", "/score/history", params=params
        )
        return [Score.model_validate(s) for s in result.data]

    async def collection(self, collection_type: str, collection_id: int) -> Collection:
        """
        获取玩家收藏品进度
        """
        result = await self._oauth_client._request_data(
            "GET", f"/{collection_type}/{collection_id}"
        )
        return Collection.model_validate(result.data)
