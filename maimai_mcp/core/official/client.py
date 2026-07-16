from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from .allnet import AllnetError, initialize_allnet, local_wall_clock_epoch
from .protocol import (
    DEFAULT_BASE_URL,
    ChimeSession,
    ChimeSessionError,
    OfficialTitleClient,
    OfficialTitleServerError,
    client_id_from_keychip,
    resolve_chime_session,
)

ACCEPTED_LOGIN_RETURN_CODES = {1, 100}
DEFAULT_KEYCHIP_ID = "A63E01E11890000"
DEFAULT_REGION_ID = 8
DEFAULT_PLACE_ID = 3496
DEFAULT_GAME_ID = "MAID"
DEFAULT_TITLE_KEY = "SDGB"
DEFAULT_NUMBER_HEADER = 46


def _extract_rating(value: Any) -> int:
    if not isinstance(value, dict):
        return 0
    for key in ("rating", "playerRating", "musicRating", "totalRating"):
        try:
            rating = int(value.get(key) or 0)
        except (TypeError, ValueError):
            rating = 0
        if rating > 0:
            return rating
    for nested_key in ("userData", "userRating"):
        rating = _extract_rating(value.get(nested_key))
        if rating:
            return rating
    return 0


@dataclass(slots=True)
class OfficialFetchResult:
    session: ChimeSession
    endpoint: str
    preview: dict[str, Any]
    user_data: dict[str, Any]
    rating_data: dict[str, Any]
    music_details: list[dict[str, Any]]
    music_groups: list[dict[str, Any]]

    @property
    def rating(self) -> int:
        for source in (self.user_data, self.rating_data, self.preview):
            rating = _extract_rating(source)
            if rating:
                return rating
        return 0

    def to_raw_export(self) -> dict[str, Any]:
        return {
            "GetUserMusicApi": {
                "userMusicList": self.music_groups,
                "length": len(self.music_groups),
                "nextIndex": 0,
            },
            "GetUserDataApi": self.user_data or {},
            "GetUserRatingApi": self.rating_data or {},
        }


class OfficialSessionHandle:
    def __init__(
        self,
        *,
        session: ChimeSession,
        client: OfficialTitleClient,
        endpoint: str,
        preview: dict[str, Any],
        login: dict[str, Any],
        region_id: int = DEFAULT_REGION_ID,
        place_id: int = DEFAULT_PLACE_ID,
        cabinet_client_id: str = "",
    ) -> None:
        self.session = session
        self.client = client
        self.endpoint = endpoint
        self.preview = preview
        self.login = login
        self.region_id = int(region_id or DEFAULT_REGION_ID)
        self.place_id = int(place_id or DEFAULT_PLACE_ID)
        self.cabinet_client_id = (cabinet_client_id or client.client_id).strip()
        self._closed = False
        self._logged_out = False

    def __enter__(self) -> OfficialSessionHandle:
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()

    def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        try:
            self.logout_best_effort()
        finally:
            self.client.close()

    def get_user_data(self) -> dict[str, Any]:
        return self.client.get_user_data(self.session.user_id)

    def get_music_groups(self) -> list[dict[str, Any]]:
        return self.client.get_user_music_groups(self.session.user_id)

    def get_rating(self) -> dict[str, Any]:
        return self.client.get_user_rating(self.session.user_id)

    def user_logout(
        self, *, login_date_time: int = 0, logout_type: int = 5
    ) -> dict[str, Any]:
        return self.client.user_logout(
            self.session.user_id,
            login_date_time=login_date_time,
            logout_type=logout_type,
            region_id=self.region_id,
            place_id=self.place_id,
            client_id=self.cabinet_client_id,
        )

    def logout_best_effort(self, *, logout_type: int = 5) -> None:
        if self._logged_out:
            return
        self._logged_out = True
        try:
            login_date_time = int(
                self.login.get("_loginDateTime")
                or self.login.get("loginDateTime")
                or 0
            )
        except (TypeError, ValueError):
            login_date_time = 0
        try:
            self.user_logout(
                login_date_time=login_date_time, logout_type=logout_type
            )
        except Exception:
            return


class MaimaiOfficialClient:
    def __init__(
        self,
        *,
        keychip_id: str = DEFAULT_KEYCHIP_ID,
        region_id: int = DEFAULT_REGION_ID,
        place_id: int = DEFAULT_PLACE_ID,
        game_id: str = DEFAULT_GAME_ID,
        title_key: str = DEFAULT_TITLE_KEY,
        timeout: float = 30.0,
        verify_tls: bool = True,
        number_header: int = DEFAULT_NUMBER_HEADER,
        debug: bool = False,
        use_allnet: bool = True,
    ) -> None:
        self.keychip_id = keychip_id or DEFAULT_KEYCHIP_ID
        self.region_id = int(region_id or DEFAULT_REGION_ID)
        self.place_id = int(place_id or DEFAULT_PLACE_ID)
        self.game_id = game_id or DEFAULT_GAME_ID
        self.title_key = title_key or DEFAULT_TITLE_KEY
        self.timeout = float(timeout or 30.0)
        self.verify_tls = verify_tls
        self.number_header = (
            int(number_header)
            if number_header is not None
            else DEFAULT_NUMBER_HEADER
        )
        self.debug = debug
        self.use_allnet = use_allnet

    @property
    def client_id(self) -> str:
        return client_id_from_keychip(self.keychip_id)

    @property
    def cabinet_client_id(self) -> str:
        return client_id_from_keychip(self.keychip_id)

    def resolve_session(self, sgid: str) -> ChimeSession:
        return resolve_chime_session(
            sgid,
            self.keychip_id,
            title_key=self.title_key,
            timeout=self.timeout,
        )

    def open(self, session: ChimeSession) -> OfficialSessionHandle:
        place_id = self.place_id
        region_id = self.region_id
        auth_time: int | None = None
        base_url = DEFAULT_BASE_URL

        if self.use_allnet:
            try:
                allnet = initialize_allnet(self.client_id, timeout=self.timeout)
                place_id = allnet.place_id
                region_id = allnet.region_id
                auth_time = allnet.auth_time_epoch
                base_url = allnet.game_server_uri
                if self.debug:
                    print(
                        f"allnet_ok place={place_id} region={region_id} "
                        f"base={base_url} auth={auth_time}"
                    )
            except AllnetError as exc:
                if self.debug:
                    print(f"allnet_failed=true reason={exc}")
                # fall back to configured place/region/base

        settings_client = OfficialTitleClient(
            base_url=base_url,
            client_id=self.client_id,
            timeout=self.timeout,
            verify_tls=self.verify_tls,
            number_header=self.number_header,
            debug=self.debug,
        )
        client: OfficialTitleClient | None = None
        returning = False
        try:
            runtime = settings_client.resolve_runtime_base_url(place_id=place_id)
            client = (
                settings_client
                if runtime == settings_client.base_url
                else settings_client.with_base_url(runtime)
            )
            preview = client.get_user_preview(session)
            if int(preview.get("errorId") or 0) != 0:
                raise OfficialTitleServerError(
                    f"preview rejected: error_id={preview.get('errorId')}"
                )
            if preview.get("isLogin"):
                raise OfficialTitleServerError("account already logged in")

            login_date_time = local_wall_clock_epoch()
            login = client.user_login(
                session,
                region_id=region_id,
                place_id=place_id,
                client_id=self.cabinet_client_id,
                date_time=auth_time,
                login_date_time=login_date_time,
            )
            code = int(login.get("returnCode") or 0)
            if code not in ACCEPTED_LOGIN_RETURN_CODES:
                raise OfficialTitleServerError(
                    f"login rejected: return_code={code}"
                )

            handle = OfficialSessionHandle(
                session=session,
                client=client,
                endpoint=runtime,
                preview=preview,
                login=login,
                region_id=region_id,
                place_id=place_id,
                cabinet_client_id=self.cabinet_client_id,
            )
            if client is not settings_client:
                settings_client.close()
            returning = True
            return handle
        except Exception:
            if client is not None and client is not settings_client:
                client.close()
            raise
        finally:
            if not returning:
                settings_client.close()

    def fetch(self, session: ChimeSession) -> OfficialFetchResult:
        handle = self.open(session)
        try:
            try:
                user_data = handle.get_user_data()
            except Exception:
                user_data = {}
            music_groups = handle.get_music_groups()
            music_details: list[dict[str, Any]] = []
            for g in music_groups:
                music_details.extend(g.get("userMusicDetailList") or [])
            try:
                rating_data = handle.get_rating()
            except Exception:
                rating_data = {}
            return OfficialFetchResult(
                session=session,
                endpoint=handle.endpoint,
                preview=handle.preview,
                user_data=user_data if isinstance(user_data, dict) else {},
                rating_data=rating_data if isinstance(rating_data, dict) else {},
                music_details=music_details,
                music_groups=music_groups,
            )
        finally:
            handle.close()

    def fetch_from_sgid(self, sgid: str) -> OfficialFetchResult:
        return self.fetch(self.resolve_session(sgid))
