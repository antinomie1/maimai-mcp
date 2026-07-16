from __future__ import annotations

import hashlib
import http.cookies
import json
import re
import socket
import ssl
import time
import zlib
from dataclasses import dataclass
from datetime import datetime
from typing import Any
from urllib.parse import urljoin, urlparse

import httpx

try:
    import requests
except ImportError as exc:  # pragma: no cover
    raise SystemExit("missing dependency: requests") from exc

from .aes_cbc import aes_cbc_decrypt, aes_cbc_encrypt


MAI_ENCODING = "1.55"
API_PREFIX = "MaimaiChn"
API_SUFFIX = "MaimaiChn"
OBFUSCATE_PARAM = "8bF76dE9"
DEFAULT_RUNTIME_SERVER_URI = "mE2s3Jhd/"
AUTH_TIME_OFFSET_SECONDS = 60 * 60
CABINET_TIME_OFFSET_SECONDS = 8 * 60 * 60
AES_KEY = b"FKM2JX:VjZNK6hc:A0<JU:i5oR7LA]9W"
AES_IV = b"F>;24DjU9W6ZsRH["
AES_BLOCK = 16

DEFAULT_BASE_URL = "https://maimai-gm.wahlap.com:42081/Maimai2Servlet/"
DEFAULT_WC_AIME_URLS = (
    "http://ai.sys-allnet.cn/wc_aime/api/get_data",
    "http://ai.sys-all.cn/wc_aime/api/get_data",
)
DEFAULT_CHIME_COMMON_KEY = "XcW5FW4cPArBXEk4vzKz3CIrMuA5EVVW"
DEFAULT_CHIME_TITLE_KEY = "SDGB"

SGWC_QR_RE = re.compile(
    r"^SGWC(?P<open_game_id>[A-Z0-9]{4})(?P<timestamp>\d{12})(?P<payload>[0-9A-Fa-f]{64})$"
)

FIXED_PATH = {
    "GetGameSettingApi": "83a5d5a20b062c5c2ad817460b8f7f76",
    "GetUserPreviewApi": "c8879fabbc5690846938b6b4290914bc",
    "UserLoginApi": "04dd218d2070685e728719b974f9b8c0",
    "UserLogoutApi": "aaa0817a628ae9cc3df35b120ecb33a4",
}


class OfficialProtocolError(RuntimeError):
    pass


class OfficialProtocolUnavailableError(OfficialProtocolError):
    pass


class ChimeSessionError(OfficialProtocolError):
    pass


class OfficialTitleServerError(OfficialProtocolError):
    pass


@dataclass(slots=True)
class ChimeSession:
    user_id: int
    token: str


@dataclass(frozen=True, slots=True)
class OfficialTitleEndpoint:
    base_url: str
    host_header: str = ""
    verify_tls: bool = True


DEFAULT_OFFICIAL_TITLE_ENDPOINTS: tuple[OfficialTitleEndpoint, ...] = (
    OfficialTitleEndpoint(DEFAULT_BASE_URL),
)




def is_official_sgid(sgid: str, game_id: str = "MAID") -> bool:
    value = (sgid or "").strip()
    return value.startswith("SGWC") and value[4:8] == game_id and len(value) > 20


def official_api_name(api: str) -> str:
    if api.endswith(API_SUFFIX):
        return api
    if api.startswith(API_PREFIX):
        api = api[len(API_PREFIX) :]
    return f"{api}{API_SUFFIX}"


def obfuscate_api(api: str) -> str:
    if api in FIXED_PATH:
        return FIXED_PATH[api]
    source = f"{official_api_name(api)}{OBFUSCATE_PARAM}".encode("utf-8")
    return hashlib.md5(source).hexdigest()


route_hash = obfuscate_api




def _pad(data: bytes) -> bytes:
    n = AES_BLOCK - (len(data) % AES_BLOCK)
    return data + bytes([n]) * n


def _unpad(data: bytes) -> bytes:
    n = data[-1]
    if n < 1 or n > AES_BLOCK or data[-n:] != bytes([n]) * n:
        raise OfficialTitleServerError("bad PKCS7 padding")
    return data[:-n]


def zlib_wrap_raw_deflate(data: bytes) -> bytes:
    compressor = zlib.compressobj(level=zlib.Z_DEFAULT_COMPRESSION, wbits=-15)
    compressed = compressor.compress(data) + compressor.flush()
    checksum = zlib.adler32(data) & 0xFFFFFFFF
    return b"\x78\x9c" + compressed + checksum.to_bytes(4, "big")


def zlib_unwrap_raw_deflate(data: bytes) -> bytes:
    if len(data) < 6:
        return b""
    return zlib.decompress(data[2:-4], wbits=-15)


def encode_request_payload(payload: dict[str, Any]) -> bytes:
    body = json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return aes_cbc_encrypt(AES_KEY, AES_IV, _pad(zlib_wrap_raw_deflate(body)))


def decode_response_payload(payload: bytes) -> dict[str, Any]:
    plain = _unpad(aes_cbc_decrypt(AES_KEY, AES_IV, payload))
    body = zlib_unwrap_raw_deflate(plain)
    if not body:
        return {}
    data = json.loads(body.decode("utf-8"))
    if not isinstance(data, dict):
        raise OfficialTitleServerError("invalid official title response")
    return data


def combo_status_to_fc_name(combo_status: int) -> str | None:
    return {1: "FC", 2: "FCP", 3: "AP", 4: "APP"}.get(int(combo_status or 0))


def sync_status_to_fs_name(sync_status: int) -> str | None:
    return {5: "SYNC", 1: "FS", 2: "FSP", 3: "FSD", 4: "FSDP"}.get(
        int(sync_status or 0)
    )




def compact_keychip_id(keychip_id: str) -> str:
    return "".join(ch for ch in (keychip_id or "").strip() if ch.isalnum())


def keychip_tail(keychip_id: str) -> str:
    value = (keychip_id or "").strip()
    if "-" in value:
        return compact_keychip_id(value.rsplit("-", 1)[-1])
    return compact_keychip_id(value)


def client_id_from_keychip(keychip_id: str) -> str:
    return compact_keychip_id(keychip_id)[:11]


def chip_id_for_chime(keychip_id: str) -> str:
    """Full hex keychip for wc_aime (not only the segment after '-')."""
    hexed = re.sub(r"[^0-9A-Fa-f]", "", (keychip_id or "").strip()).upper()
    return hexed or compact_keychip_id(keychip_id)


short_client_id = client_id_from_keychip


@dataclass(slots=True)
class _GameHttpResponse:
    status_code: int
    headers: dict[str, str]
    raw_headers: list[tuple[str, str]]
    content: bytes
    set_cookie: str


def _parse_raw_http_header(
    raw_header: bytes,
) -> tuple[int, str, dict[str, str], list[tuple[str, str]]]:
    lines = raw_header.decode("iso-8859-1", errors="replace").split("\r\n")
    status_line = lines[0] if lines else ""
    parts = status_line.split(" ", 2)
    status = int(parts[1]) if len(parts) >= 2 and parts[1].isdigit() else 0
    reason = parts[2] if len(parts) >= 3 else ""
    headers: dict[str, str] = {}
    raw_headers: list[tuple[str, str]] = []
    for line in lines[1:]:
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        stripped = value.strip()
        headers[key.strip().lower()] = stripped
        raw_headers.append((key.strip(), stripped))
    return status, reason, headers, raw_headers


def _read_response_header(
    sock: socket.socket,
) -> tuple[int, str, dict[str, str], list[tuple[str, str]], bytes]:
    data = b""
    while b"\r\n\r\n" not in data:
        chunk = sock.recv(4096)
        if not chunk:
            break
        data += chunk
    marker = data.find(b"\r\n\r\n")
    if marker < 0:
        raise OfficialTitleServerError("HTTP response header was incomplete")
    status, reason, headers, raw_headers = _parse_raw_http_header(data[:marker])
    return status, reason, headers, raw_headers, data[marker + 4 :]


def _read_body(
    sock: socket.socket, headers: dict[str, str], prefix: bytes
) -> bytes:
    if "chunked" in (headers.get("transfer-encoding") or "").lower():
        raise OfficialTitleServerError("chunked response bodies are not supported")
    content_length = headers.get("content-length")
    if content_length is None:
        chunks = [prefix]
        while True:
            chunk = sock.recv(65536)
            if not chunk:
                break
            chunks.append(chunk)
        return b"".join(chunks)
    total = int(content_length)
    body = prefix
    while len(body) < total:
        chunk = sock.recv(min(65536, total - len(body)))
        if not chunk:
            break
        body += chunk
    return body


def _extract_set_cookie(raw_headers: list[tuple[str, str]]) -> str:
    pairs: list[str] = []
    for name, value in raw_headers:
        if name.lower() != "set-cookie":
            continue
        parsed = http.cookies.SimpleCookie()
        parsed.load(value)
        pairs.extend(f"{morsel.key}={morsel.value}" for morsel in parsed.values())
    return "; ".join(pairs)


def send_continue_post(
    url: str,
    headers: dict[str, str],
    payload: bytes,
    timeout: float,
    verify_tls: bool,
) -> _GameHttpResponse:
    """POST with Expect: 100-continue (required by title server for non-empty bodies)."""
    parsed = urlparse(url)
    if parsed.scheme not in ("http", "https") or not parsed.hostname:
        raise OfficialTitleServerError("game API URL must be http(s) with host")
    port = parsed.port or (443 if parsed.scheme == "https" else 80)
    path = parsed.path or "/"
    if parsed.query:
        path += "?" + parsed.query
    host_header = parsed.netloc
    merged = dict(headers)
    merged["Content-Length"] = str(len(payload))
    merged["Expect"] = merged.get("Expect", "100-continue")
    merged["Host"] = host_header
    order = (
        "number",
        "Content-Type",
        "User-Agent",
        "charset",
        "Mai-Encoding",
        "Content-Encoding",
        "Content-Length",
        "Expect",
        "Host",
        "Cookie",
    )
    ordered = [(key, merged.pop(key)) for key in order if key in merged]
    ordered.extend(merged.items())
    request_lines = [f"POST {path} HTTP/1.1"]
    request_lines.extend(f"{key}: {value}" for key, value in ordered)
    request = ("\r\n".join(request_lines) + "\r\n\r\n").encode("iso-8859-1")
    raw_sock: socket.socket | None = None
    try:
        raw_sock = socket.create_connection((parsed.hostname, port), timeout=timeout)
        raw_sock.settimeout(timeout)
        if parsed.scheme == "https":
            context = (
                ssl.create_default_context()
                if verify_tls
                else ssl._create_unverified_context()
            )
            sock = context.wrap_socket(raw_sock, server_hostname=parsed.hostname)
            raw_sock = None
        else:
            sock = raw_sock
            raw_sock = None
        with sock:
            sock.settimeout(timeout)
            sock.sendall(request)
            status, reason, response_headers, raw_headers, body_prefix = (
                _read_response_header(sock)
            )
            if status == 100:
                sock.sendall(payload)
                status, reason, response_headers, raw_headers, body_prefix = (
                    _read_response_header(sock)
                )
            body = _read_body(sock, response_headers, body_prefix)
    finally:
        if raw_sock is not None:
            raw_sock.close()
    return _GameHttpResponse(
        status,
        response_headers,
        raw_headers,
        body,
        _extract_set_cookie(raw_headers),
    )




def resolve_chime_session(
    sgid: str,
    keychip_id: str,
    *,
    urls: tuple[str, ...] = DEFAULT_WC_AIME_URLS,
    common_key: str = DEFAULT_CHIME_COMMON_KEY,
    title_key: str = DEFAULT_CHIME_TITLE_KEY,
    timeout: float = 30.0,
) -> ChimeSession:
    value = (sgid or "").strip()
    m = SGWC_QR_RE.match(value)
    if not m:
        raise ChimeSessionError("invalid SGID format for official chime API")
    chip = chip_id_for_chime(keychip_id)
    ts = datetime.now().strftime("%y%m%d%H%M%S")
    key = hashlib.sha256(f"{chip}{ts}{common_key}".encode("ascii")).hexdigest().upper()
    body = {
        "chipID": chip,
        "openGameID": m.group("open_game_id"),
        "key": key,
        "qrCode": m.group("payload").upper(),
        "timestamp": ts,
        "titlekey": title_key,
    }
    raw = json.dumps(body, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    last: Exception | None = None
    for url in urls:
        try:
            r = requests.post(
                url,
                data=raw,
                headers={"User-Agent": "WC_AIME_LIB"},
                timeout=timeout,
            )
            if r.status_code < 200 or r.status_code >= 300:
                raise ChimeSessionError(f"wc_aime HTTP {r.status_code}")
            data = r.json()
            if int(data.get("errorID") or 0) != 0:
                raise ChimeSessionError(f"wc_aime errorID={data.get('errorID')}")
            uid = int(data.get("userID") or 0)
            token = str(data.get("token") or "")
            if not uid or not token:
                raise ChimeSessionError("empty user_id or token")
            return ChimeSession(user_id=uid, token=token)
        except Exception as exc:  # noqa: BLE001
            last = exc
    raise ChimeSessionError(f"official chime session failed: {last}")




class OfficialTitleClient:
    def __init__(
        self,
        *,
        base_url: str,
        client_id: str,
        timeout: float = 30.0,
        verify_tls: bool = True,
        number_header: int = 46,
        debug: bool = False,
    ) -> None:
        if not base_url:
            raise OfficialProtocolUnavailableError(
                "official title base URL is not configured"
            )
        self.base_url = base_url.rstrip("/") + "/"
        self.client_id = client_id or ""
        self.timeout = float(timeout or 30.0)
        self.verify_tls = bool(verify_tls)
        self.number_header = int(number_header if number_header is not None else 46)
        self.debug = debug
        # kept for compatibility; title posts use raw 100-continue sockets
        self._client = httpx.Client(
            timeout=self.timeout, verify=self.verify_tls, trust_env=False
        )
        self._cookies: dict[str, str] = {}

    def close(self) -> None:
        self._client.close()

    def with_base_url(self, base_url: str) -> OfficialTitleClient:
        other = OfficialTitleClient(
            base_url=base_url,
            client_id=self.client_id,
            timeout=self.timeout,
            verify_tls=self.verify_tls,
            number_header=self.number_header,
            debug=self.debug,
        )
        other._cookies = dict(self._cookies)
        return other

    def _cookie_header(self) -> str:
        return "; ".join(f"{k}={v}" for k, v in self._cookies.items())

    def _store_set_cookie(self, set_cookie: str) -> None:
        if not set_cookie:
            return
        for part in set_cookie.split(";"):
            piece = part.strip()
            if "=" in piece:
                ck, cv = piece.split("=", 1)
                # only first name=value of each cookie pair list entry
                if ck.lower() in {
                    "path",
                    "domain",
                    "expires",
                    "max-age",
                    "secure",
                    "httponly",
                    "samesite",
                }:
                    continue
                self._cookies[ck.strip()] = cv.strip()

    def post(self, api: str, user_id: int, payload: dict[str, Any]) -> dict[str, Any]:
        api_hash = obfuscate_api(api)
        agent_id = str(int(user_id)) if int(user_id or 0) else self.client_id
        headers = {
            "Content-Type": "application/json",
            "charset": "UTF-8",
            "Mai-Encoding": MAI_ENCODING,
            "Content-Encoding": "deflate",
            "User-Agent": f"{api_hash}#{agent_id}",
            "number": str(self.number_header),
            "Expect": "100-continue",
        }
        cookie = self._cookie_header()
        if cookie:
            headers["Cookie"] = cookie
        body = encode_request_payload(payload)
        url = f"{self.base_url}{api_hash}"
        try:
            response = send_continue_post(
                url, headers, body, self.timeout, self.verify_tls
            )
        except OfficialTitleServerError:
            raise
        except Exception as exc:
            raise OfficialTitleServerError(f"{api} transport failed: {exc}") from exc
        if self.debug:
            print(
                f"http_trace api={api} status={response.status_code} "
                f"bytes={len(response.content)}"
            )
        if response.status_code < 200 or response.status_code >= 300:
            raise OfficialTitleServerError(f"{api} HTTP {response.status_code}")
        self._store_set_cookie(response.set_cookie)
        if not response.content:
            raise OfficialTitleServerError("empty official title response")
        try:
            return decode_response_payload(response.content)
        except OfficialTitleServerError:
            raise
        except Exception as exc:
            raise OfficialTitleServerError("invalid official title response") from exc

    def get_user_preview(self, session: ChimeSession) -> dict[str, Any]:
        return self.post(
            "GetUserPreviewApi",
            session.user_id,
            {
                "userId": session.user_id,
                "segaIdAuthKey": "",
                "token": session.token,
                "clientId": self.client_id,
            },
        )

    def get_game_setting(self, *, place_id: int = 0) -> dict[str, Any]:
        return self.post(
            "GetGameSettingApi",
            0,
            {"placeId": int(place_id or 0), "clientId": self.client_id},
        )

    def resolve_runtime_base_url(self, *, place_id: int = 0) -> str:
        try:
            response = self.get_game_setting(place_id=place_id)
        except Exception as exc:  # noqa: BLE001
            print(f"game_setting_failed=true reason={exc}")
            return self.base_url
        game_setting = response.get("gameSetting") or {}
        movie = str(game_setting.get("movieServerUri") or "").strip()
        if not movie:
            movie = DEFAULT_RUNTIME_SERVER_URI
        if urlparse(movie).scheme:
            return movie.rstrip("/") + "/"
        return urljoin(self.base_url, movie).rstrip("/") + "/"

    def user_login(
        self,
        session: ChimeSession,
        *,
        access_code: str = "",
        region_id: int = 8,
        place_id: int = 0,
        generic_flag: int = 0,
        client_id: str = "",
        date_time: int | None = None,
        login_date_time: int | None = None,
    ) -> dict[str, Any]:
        now = int(time.time())
        auth_date_time = (
            int(date_time)
            if date_time is not None
            else now + AUTH_TIME_OFFSET_SECONDS
        )
        login_dt = (
            int(login_date_time)
            if login_date_time is not None
            else now + CABINET_TIME_OFFSET_SECONDS
        )
        response = self.post(
            "UserLoginApi",
            session.user_id,
            {
                "userId": session.user_id,
                "accessCode": access_code or "",
                "regionId": int(region_id or 0),
                "placeId": int(place_id or 0),
                "clientId": client_id or self.client_id,
                "dateTime": auth_date_time,
                "loginDateTime": login_dt,
                "isContinue": False,
                "genericFlag": int(generic_flag or 0),
                "token": session.token,
            },
        )
        response.setdefault("_loginDateTime", login_dt)
        return response

    def user_logout(
        self,
        user_id: int,
        *,
        login_date_time: int = 0,
        logout_type: int = 5,
        access_code: str = "",
        region_id: int = 8,
        place_id: int = 0,
        client_id: str = "",
    ) -> dict[str, Any]:
        return self.post(
            "UserLogoutApi",
            int(user_id or 0),
            {
                "userId": int(user_id or 0),
                "accessCode": access_code or "",
                "regionId": int(region_id or 0),
                "placeId": int(place_id or 0),
                "clientId": client_id or self.client_id,
                "loginDateTime": int(login_date_time or 0),
                "type": int(logout_type or 5),
            },
        )

    def get_user_music(
        self, user_id: int, *, max_count: int = 50
    ) -> list[dict[str, Any]]:
        details: list[dict[str, Any]] = []
        next_index = 0
        while True:
            response = self.post(
                "GetUserMusicApi",
                user_id,
                {
                    "userId": user_id,
                    "nextIndex": next_index,
                    "maxCount": int(max_count or 50),
                },
            )
            for music in response.get("userMusicList") or []:
                if isinstance(music, dict):
                    details.extend(music.get("userMusicDetailList") or [])
            next_index = int(response.get("nextIndex") or 0)
            if next_index == 0:
                return details

    def get_user_music_groups(
        self, user_id: int, *, max_count: int = 50
    ) -> list[dict[str, Any]]:
        groups: list[dict[str, Any]] = []
        next_index = 0
        while True:
            response = self.post(
                "GetUserMusicApi",
                user_id,
                {
                    "userId": user_id,
                    "nextIndex": next_index,
                    "maxCount": int(max_count or 50),
                },
            )
            for music in response.get("userMusicList") or []:
                if isinstance(music, dict):
                    groups.append(music)
            next_index = int(response.get("nextIndex") or 0)
            if next_index == 0:
                return groups

    def get_user_rating(self, user_id: int) -> dict[str, Any]:
        response = self.post("GetUserRatingApi", user_id, {"userId": user_id})
        return response.get("userRating") or response

    def get_user_data(self, user_id: int) -> dict[str, Any]:
        response = self.post("GetUserDataApi", user_id, {"userId": user_id})
        return response.get("userData") or response
