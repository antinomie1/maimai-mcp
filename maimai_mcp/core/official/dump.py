#!/usr/bin/env python3
"""
Standalone SDGB155 full user-data dump with verified logout.

This is intentionally a single-file tool. It does not import sibling project
modules or read project JSON config files. Runtime dependencies are only:
  - requests
  - pycryptodome

Flow:
  QR/dummylogin -> wc_aime userId/token -> ALL.Net init -> GetGameSettingApi
  -> GetUserPreviewApi -> UserLoginApi -> full GetUser* dump -> UserLogoutApi
  -> GetUserPreviewApi release check, retrying logout up to --logout-attempts.
"""

from __future__ import annotations

import argparse
import configparser
import hashlib
import http.cookies
import json
import os
import random
import re
import socket
import ssl
import sys
import time
import zlib
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Iterable
from urllib.parse import unquote, urlparse

try:
    import requests
except ImportError as exc:  # pragma: no cover - operator-facing dependency check
    raise SystemExit("missing dependency: requests. Install with: python -m pip install requests") from exc

from .aes_cbc import aes_cbc_decrypt, aes_cbc_encrypt

SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_OUTPUT_DIR = SCRIPT_DIR / "session_exports"

DEFAULT_INITIALIZE_URL = "http://at.sys-all.cn/net/initialize"
DEFAULT_DELIVERY_URL = "http://at.sys-all.cn/net/delivery/instruction"
DEFAULT_ALLNET_CONNECT_HOST = "at.sys-allnet.cn"
DEFAULT_ALLNET_USER_AGENT = "SDGB;Windows/Lite"
DEFAULT_ALLNET_AES_KEY_HEX = "2f3f6a6f2b224c265c437239283d6b47"
DEFAULT_TITLE_ID = "SDGB"
DEFAULT_TITLE_VER = "1.55.00"
DEFAULT_KEYCHIP_ID = "A63E01D90630000"
DEFAULT_GAME_SERVER_PATH = "Maimai2Servlet"

DEFAULT_WC_AIME_URLS = (
    "http://ai.sys-allnet.cn/wc_aime/api/get_data",
    "http://ai.sys-all.cn/wc_aime/api/get_data",
)
DEFAULT_CHIME_COMMON_KEY = "XcW5FW4cPArBXEk4vzKz3CIrMuA5EVVW"
DEFAULT_CHIME_TITLE_KEY = "SDGB"
DEFAULT_CHIME_USER_AGENT = "WC_AIME_LIB"
SGWC_QR_RE = re.compile(r"^SGWC(?P<open_game_id>[A-Z0-9]{4})(?P<timestamp>\d{12})(?P<payload>[0-9A-Fa-f]{64})$")

GAME_AES_IV = b"F>;24DjU9W6ZsRH["
GAME_AES_KEY = b"FKM2JX:VjZNK6hc:A0<JU:i5oR7LA]9W"
AES_BLOCK_SIZE = 16

HASH_SALT = "8bF76dE9"
GAME_SETTING_PATH_HASH = "83a5d5a20b062c5c2ad817460b8f7f76"
PREVIEW_PATH_HASH = "c8879fabbc5690846938b6b4290914bc"
LOGIN_PATH_HASH = "04dd218d2070685e728719b974f9b8c0"
LOGOUT_PATH_HASH = "aaa0817a628ae9cc3df35b120ecb33a4"
PING_PATH_HASH = "3bae8c1162b3148abcfc38a89c03e3e7"
DEFAULT_NUMBER_HEADER = 46

API_KIND_ORDER = [
    "GetUserDataApi",
    "GetUserCharacterApi",
    "GetUserItemApi",
    "GetUserFavoriteApi",
    "GetUserGhostApi",
    "GetUserMapApi",
    "GetUserLoginBonusApi",
    "GetUserRegionApi",
    "GetUserRecommendRateMusicApi",
    "GetUserRecommendSelectMusicApi",
    "GetUserOptionApi",
    "GetUserExtendApi",
    "GetUserRatingApi",
    "GetUserMusicApi",
    "GetUserPortraitApi",
    "GetUserActivityApi",
    "GetUserFavoriteItemApi",
    "GetUserRivalDataApi",
    "GetUserRivalMusicApi",
    "GetUserMissionDataApi",
    "GetUserFriendBonusApi",
    "GetUserIntimateApi",
    "GetUserShopStockApi",
    "GetUserKaleidxScopeApi",
    "UploadUserPlaylogListApi",
    "UpsertUserAllApi",
]
API_KIND_INDEX = {name: index for index, name in enumerate(API_KIND_ORDER)}

USER_ITEM_KINDS = (1, 2, 10, 3, 4, 11, 12, 5, 6, 7, 8, 14)
FAVORITE_KINDS = (3, 1, 2, 10, 11)
FAVORITE_ITEM_KINDS = (2, 1)
SHOP_ITEM_IDS = [
    1030002, 1030003, 2000011, 2000012, 2000013, 2000014, 2000015, 2000016,
    3000701, 3000702, 3000703, 3000704, 3000705, 3000706, 3000707, 3000708,
    3000709, 3050101, 3050102, 3050103, 3050104, 3050105, 3050201, 3050202,
    3050203, 3050204, 3050205, 3050401, 3050402, 3050403, 3050404, 3050405,
    3050601, 3050602, 3050603, 3050604, 3050605, 3050606, 3050607, 3050701,
    3050702, 3050703, 3050704, 3050705, 3050801, 3050802, 3050803, 3050804,
    3100101, 3100102, 3100103, 3100104, 3100105, 3100301, 3100302, 3100303,
    3100304, 3100305, 3100401, 3100402, 3100403, 3100404, 3100405, 3100501,
    3100502, 3100503, 3100504, 3100505, 3100601, 3100602, 3100603, 3100604,
    3100605, 3150101, 3150102, 3150103, 3150104, 3150105, 3150106, 3150201,
    3150202, 3150203, 3150301, 3150302, 3150303, 3150304, 3150305, 3150401,
    3150402, 3150403, 3150404, 3150405, 3150501, 3150502, 3150503, 3150504,
    3150505, 3200101, 3200102, 3200103, 3200104, 3200105, 3200201, 3200202,
    3200203, 3200204, 3200205, 3200301, 3200302, 3200303, 3200304, 3200305,
    3200401, 3200402, 3200403, 3200404, 3200405, 3200601, 3200602, 3200603,
    3200604, 3200605, 4050101, 4050201, 4050301, 4050401, 4050501, 4050601,
    4050801, 4050901, 4100101, 4100301, 4100401, 4100501, 4100601, 4150101,
    4150201, 4150301, 4150401, 4150501, 4200101, 4200201, 4200301, 4200401,
    4200601, 5100101, 5100301, 5100401, 5100501, 5100601, 5150101, 5150201,
    5150301, 5150401, 5150501, 5200101, 5200201, 5200301, 5200401, 5200601,
    5459504, 7010001, 7010002, 7010003,
]


class FlowError(RuntimeError):
    pass


@dataclass
class RawHttpResponse:
    status_code: int
    reason: str
    headers: dict[str, str]
    raw_headers: list[tuple[str, str]]
    body: bytes


@dataclass
class AllnetExchange:
    endpoint: str
    status_code: int
    request_wire_length: int
    response_wire_length: int
    fields: dict[str, str]


@dataclass
class AllnetResult:
    initialize: AllnetExchange
    delivery: AllnetExchange | None
    auth_time_epoch: int
    auth_time_local_wall_epoch: int
    game_server_uri: str
    place_id: int
    region_id: int


@dataclass
class GameResponse:
    status_code: int
    headers: dict[str, str]
    raw_headers: list[tuple[str, str]]
    content: bytes
    set_cookie: str


def bool_text(value: bool) -> str:
    return str(value).lower()


def env(name: str, default: str | None = None) -> str | None:
    value = os.environ.get(name)
    return value if value not in (None, "") else default


def int_env(name: str, default: int | None = None) -> int | None:
    value = env(name)
    return int(value) if value is not None else default


def coerce_int(value: Any) -> int | None:
    if value in (None, ""):
        return None
    try:
        return int(str(value), 10)
    except ValueError:
        return None


def compact_hex_identifier(value: str | None) -> str:
    if not value:
        return ""
    text = re.sub(r"[^0-9A-Fa-f]", "", value).upper()
    return text if text else value.strip()


def client_id_from_keychip_id(value: str | None) -> str:
    compact = compact_hex_identifier(value)
    return compact[:11] if len(compact) >= 11 else ""


def normalize_client_id(value: str | None) -> str:
    compact = compact_hex_identifier(value)
    if len(compact) > 11:
        return compact[:11]
    return compact


def normalize_allnet_title_ver(value: str | None) -> str:
    text = (value or "").strip()
    match = re.fullmatch(r"(\d+\.\d+)\.0+", text)
    if match:
        return match.group(1)
    return text


def local_wall_clock_epoch() -> int:
    return int((datetime.now() - datetime(1970, 1, 1)).total_seconds())


def safe_local_defaults(game_root: str | None) -> dict[str, str]:
    if not game_root:
        return {}
    root = Path(game_root).expanduser()
    package = root / "Package" if (root / "Package").is_dir() else root
    defaults: dict[str, str] = {}
    segatools = package / "segatools.ini"
    if segatools.is_file():
        parser = configparser.ConfigParser()
        parser.optionxform = str
        parser.read(segatools, encoding="utf-8")
        if parser.has_section("keychip"):
            keychip = parser["keychip"]
            if keychip.get("id"):
                defaults["keychip_id"] = str(keychip.get("id"))
            if keychip.get("gameId"):
                defaults["title_id"] = str(keychip.get("gameId"))
    return defaults


def pkcs7_pad(data: bytes) -> bytes:
    pad_len = AES_BLOCK_SIZE - (len(data) % AES_BLOCK_SIZE)
    return data + bytes([pad_len]) * pad_len


def pkcs7_unpad(data: bytes) -> bytes:
    if not data:
        raise ValueError("empty AES plaintext")
    pad_len = data[-1]
    if pad_len < 1 or pad_len > AES_BLOCK_SIZE:
        raise ValueError(f"invalid PKCS7 padding length: {pad_len}")
    if data[-pad_len:] != bytes([pad_len]) * pad_len:
        raise ValueError("invalid PKCS7 padding bytes")
    return data[:-pad_len]


def encode_game_payload(body: dict[str, Any]) -> bytes:
    json_bytes = json.dumps(body, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    compressor = zlib.compressobj(level=6, wbits=-zlib.MAX_WBITS)
    raw_deflate = compressor.compress(json_bytes) + compressor.flush()
    adler = zlib.adler32(json_bytes) & 0xFFFFFFFF
    zlib_frame = b"\x78\x9c" + raw_deflate + adler.to_bytes(4, "big")
    return aes_cbc_encrypt(GAME_AES_KEY, GAME_AES_IV, pkcs7_pad(zlib_frame))


def decode_game_payload(payload: bytes) -> dict[str, Any]:
    decrypted = pkcs7_unpad(
        aes_cbc_decrypt(GAME_AES_KEY, GAME_AES_IV, payload)
    )
    if decrypted[:2] != b"\x78\x9c":
        raise ValueError(f"unexpected zlib header: {decrypted[:2].hex()}")
    expected_adler = int.from_bytes(decrypted[-4:], "big")
    json_bytes = zlib.decompress(decrypted[2:-4], wbits=-zlib.MAX_WBITS)
    if (zlib.adler32(json_bytes) & 0xFFFFFFFF) != expected_adler:
        raise ValueError("response Adler32 check failed")
    value = json.loads(json_bytes.decode("utf-8"))
    if not isinstance(value, dict):
        raise ValueError("decoded response JSON was not an object")
    return value


def encrypt_allnet_body(plaintext: bytes, key: bytes) -> bytes:
    iv = os.urandom(AES_BLOCK_SIZE)
    return iv + aes_cbc_encrypt(key, iv, pkcs7_pad(plaintext))


def decrypt_allnet_body(wire_body: bytes, key: bytes) -> bytes:
    if len(wire_body) < AES_BLOCK_SIZE * 2 or len(wire_body) % AES_BLOCK_SIZE:
        raise ValueError(f"invalid encrypted ALL.Net body length: {len(wire_body)}")
    iv = wire_body[:AES_BLOCK_SIZE]
    return pkcs7_unpad(aes_cbc_decrypt(key, iv, wire_body[AES_BLOCK_SIZE:]))


def parse_aes_key_hex(value: str | None) -> bytes:
    text = re.sub(r"[^0-9A-Fa-f]", "", value or "")
    if len(text) != 32:
        raise ValueError("ALL.Net AES key must be 16 bytes / 32 hex characters")
    return bytes.fromhex(text)


def parse_http_response_header(raw_header: bytes) -> tuple[int, str, dict[str, str], list[tuple[str, str]]]:
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


def raw_http10_post(url: str, user_agent: str, body: bytes, timeout: float, connect_host: str | None) -> RawHttpResponse:
    parsed = urlparse(url)
    if parsed.scheme.lower() != "http" or not parsed.hostname:
        raise ValueError("ALL.Net URL must be plain http:// with host")
    port = parsed.port or 80
    path = parsed.path or "/"
    if parsed.query:
        path += "?" + parsed.query
    host_header = parsed.netloc
    lines = [
        f"POST {path} HTTP/1.0",
        "Connection: Close",
        f"User-Agent: {user_agent}",
        f"Host: {host_header}",
        "Content-Type: application/x-www-form-urlencoded",
        f"Content-Length: {len(body)}",
    ]
    request = ("\r\n".join(lines) + "\r\n\r\n").encode("ascii") + body
    actual_host = connect_host or parsed.hostname
    with socket.create_connection((actual_host, port), timeout=timeout) as sock:
        sock.settimeout(timeout)
        sock.sendall(request)
        chunks: list[bytes] = []
        while True:
            try:
                chunk = sock.recv(65536)
            except socket.timeout:
                break
            if not chunk:
                break
            chunks.append(chunk)
    response = b"".join(chunks)
    marker = response.find(b"\r\n\r\n")
    if marker < 0:
        raise FlowError(f"ALL.Net HTTP response had no header, bytes={len(response)}")
    status, reason, headers, raw_headers = parse_http_response_header(response[:marker])
    return RawHttpResponse(status, reason, headers, raw_headers, response[marker + 4:])


def parse_allnet_form(text: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    stripped = text.strip("\ufeff \t\r\n\0")
    for part in stripped.replace("\r", "\n").replace("\n", "&").split("&"):
        if not part or "=" not in part:
            continue
        key, value = part.split("=", 1)
        fields[unquote(key)] = unquote(value)
    return fields


def send_allnet_exchange(
    endpoint: str,
    url: str,
    user_agent: str,
    plaintext: str,
    aes_key: bytes,
    timeout: float,
    connect_host: str | None,
) -> AllnetExchange:
    wire_body = encrypt_allnet_body(plaintext.encode("ascii"), aes_key)
    response = raw_http10_post(url, user_agent, wire_body, timeout, connect_host)
    if response.status_code < 200 or response.status_code >= 300:
        raise FlowError(f"{endpoint} HTTP status {response.status_code} {response.reason}".strip())
    plain = decrypt_allnet_body(response.body, aes_key).decode("utf-8", errors="replace")
    return AllnetExchange(
        endpoint=endpoint,
        status_code=response.status_code,
        request_wire_length=len(wire_body),
        response_wire_length=len(response.body),
        fields=parse_allnet_form(plain),
    )


def parse_utc_time_epoch(value: str | None) -> int | None:
    if not value:
        return None
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        dt = datetime.fromisoformat(text)
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return int(dt.timestamp())


def parse_timezone_offset(value: str | None) -> timezone:
    match = re.fullmatch(r"([+-])(\d{2})(\d{2})", (value or "").strip())
    if not match:
        return timezone.utc
    sign = 1 if match.group(1) == "+" else -1
    return timezone(sign * timedelta(hours=int(match.group(2)), minutes=int(match.group(3))))


def local_wall_epoch_from_utc(value: str | None, timezone_text: str | None) -> int | None:
    true_epoch = parse_utc_time_epoch(value)
    if true_epoch is None:
        return None
    local_dt = datetime.fromtimestamp(true_epoch, timezone.utc).astimezone(parse_timezone_offset(timezone_text)).replace(tzinfo=None)
    return int((local_dt - datetime(1970, 1, 1)).total_seconds())


def parse_allnet_place_id(value: str | None) -> int | None:
    text = (value or "").strip()
    if not text:
        return None
    try:
        if re.fullmatch(r"[0-9A-Fa-f]{4}", text):
            return int(text, 16)
        return int(text, 10)
    except ValueError:
        return None


def join_uri_path(base_uri: str, path: str) -> str:
    return base_uri.rstrip("/") + "/" + path.strip("/") + "/"


def initialize_allnet(args: argparse.Namespace, client_id: str) -> AllnetResult:
    aes_key = parse_aes_key_hex(args.allnet_aes_key_hex)
    token = random.randrange(0, 2**32) if args.allnet_token is None else int(args.allnet_token)
    title_ver = normalize_allnet_title_ver(args.title_ver)
    init_plain = f"title_id={args.title_id}&title_ver={title_ver}&client_id={client_id}&token={token}\r\n"
    init = send_allnet_exchange(
        "initialize",
        args.allnet_initialize_url,
        args.allnet_user_agent,
        init_plain,
        aes_key,
        args.timeout,
        args.allnet_connect_host or None,
    )
    if init.fields.get("result") != "1":
        raise FlowError(f"ALL.Net initialize returned result={init.fields.get('result', 'absent')}")
    delivery = None
    if not args.skip_allnet_delivery:
        delivery_plain = f"title_id={args.title_id}&title_ver={title_ver}&client_id={client_id}\r\n"
        delivery = send_allnet_exchange(
            "delivery",
            args.allnet_delivery_url,
            args.allnet_user_agent,
            delivery_plain,
            aes_key,
            args.timeout,
            args.allnet_connect_host or None,
        )
        if delivery.fields.get("result") != "1":
            raise FlowError(f"ALL.Net delivery returned result={delivery.fields.get('result', 'absent')}")

    auth_epoch = parse_utc_time_epoch(init.fields.get("utc_time"))
    auth_wall = local_wall_epoch_from_utc(init.fields.get("utc_time"), init.fields.get("client_timezone"))
    place_id = parse_allnet_place_id(init.fields.get("place_id"))
    region_id = coerce_int(init.fields.get("region0"))
    uri1 = init.fields.get("uri1")
    if auth_epoch is None or auth_wall is None or place_id is None or region_id is None or not uri1:
        raise FlowError("ALL.Net initialize response missed auth/place/region/uri1 fields")
    return AllnetResult(
        initialize=init,
        delivery=delivery,
        auth_time_epoch=auth_epoch,
        auth_time_local_wall_epoch=auth_wall,
        game_server_uri=join_uri_path(uri1, args.allnet_game_server_path),
        place_id=place_id,
        region_id=region_id,
    )


def parse_sgwc_qr(qr_content: str) -> tuple[str, str, str]:
    match = SGWC_QR_RE.match(qr_content.strip())
    if not match:
        raise ValueError("QR content is not an SGWC... MAID QR payload")
    return match.group("open_game_id"), match.group("timestamp"), match.group("payload").upper()


def now_chime_timestamp() -> str:
    return datetime.now().strftime("%y%m%d%H%M%S")


def resolve_qr(qr_content: str, keychip_id: str, args: argparse.Namespace) -> dict[str, Any]:
    open_game_id, qr_timestamp, qr_payload = parse_sgwc_qr(qr_content)
    chip_id = compact_hex_identifier(keychip_id)
    timestamp = args.chime_timestamp or (qr_timestamp if args.chime_use_qr_timestamp else now_chime_timestamp())
    source = f"{chip_id}{timestamp}{args.chime_common_key}".encode("ascii")
    body = {
        "chipID": chip_id,
        "openGameID": open_game_id,
        "key": hashlib.sha256(source).hexdigest().upper(),
        "qrCode": qr_payload,
        "timestamp": timestamp,
        "titlekey": args.chime_title_key,
    }
    payload = json.dumps(body, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    session = requests.Session()
    session.trust_env = args.chime_trust_env
    session.headers.clear()
    headers = {"User-Agent": args.chime_user_agent}
    last_error: Exception | None = None
    for url in args.wc_aime_url:
        try:
            response = session.post(url, headers=headers, data=payload, timeout=args.timeout)
            if response.status_code < 200 or response.status_code >= 300:
                raise FlowError(f"wc_aime HTTP status {response.status_code}")
            data = response.json()
            if not isinstance(data, dict):
                raise FlowError("wc_aime response JSON was not an object")
            error_id = coerce_int(data.get("errorID"))
            if error_id not in (None, 0):
                raise FlowError(f"wc_aime returned errorID={error_id}")
            user_id = coerce_int(data.get("userID"))
            token = str(data.get("token") or "")
            if not user_id or not token:
                raise FlowError("wc_aime response did not contain userID/token")
            return {"userId": user_id, "token": token, "chimeTimestamp": timestamp, "chimeUrl": url}
        except Exception as exc:  # noqa: BLE001 - keep last URL failure context
            last_error = exc
    raise FlowError(f"QR resolve failed: {last_error}")


def route_hash(api_name: str) -> str:
    return hashlib.md5((api_name + "MaimaiChn" + HASH_SALT).encode("utf-8")).hexdigest()


def build_url(base_url: str, path_hash: str) -> str:
    return base_url.rstrip("/") + "/" + path_hash


def read_response_header(sock: socket.socket) -> tuple[int, str, dict[str, str], list[tuple[str, str]], bytes]:
    data = b""
    while b"\r\n\r\n" not in data:
        chunk = sock.recv(4096)
        if not chunk:
            break
        data += chunk
    marker = data.find(b"\r\n\r\n")
    if marker < 0:
        raise FlowError("HTTP response header was incomplete")
    status, reason, headers, raw_headers = parse_http_response_header(data[:marker])
    return status, reason, headers, raw_headers, data[marker + 4:]


def read_body(sock: socket.socket, headers: dict[str, str], prefix: bytes) -> bytes:
    if "chunked" in (headers.get("transfer-encoding") or "").lower():
        raise FlowError("chunked response bodies are not implemented")
    content_length = headers.get("content-length")
    if content_length is None:
        chunks = [prefix]
        while True:
            chunk = sock.recv(65536)
            if not chunk:
                break
            chunks.append(chunk)
        return b"".join(chunks)
    expected = int(content_length)
    body = prefix
    while len(body) < expected:
        chunk = sock.recv(min(65536, expected - len(body)))
        if not chunk:
            break
        body += chunk
    return body[:expected]


def extract_set_cookie(raw_headers: list[tuple[str, str]]) -> str:
    pairs: list[str] = []
    for name, value in raw_headers:
        if name.lower() != "set-cookie":
            continue
        parsed = http.cookies.SimpleCookie()
        parsed.load(value)
        pairs.extend(f"{morsel.key}={morsel.value}" for morsel in parsed.values())
    return "; ".join(pairs)


def send_continue_post(url: str, headers: dict[str, str], payload: bytes, timeout: float, verify_tls: bool) -> GameResponse:
    parsed = urlparse(url)
    if parsed.scheme not in ("http", "https") or not parsed.hostname:
        raise ValueError("game API URL must be http(s) with host")
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
        "number", "Content-Type", "User-Agent", "charset", "Mai-Encoding",
        "Content-Encoding", "Content-Length", "Expect", "Host", "Cookie",
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
            context = ssl.create_default_context() if verify_tls else ssl._create_unverified_context()
            sock = context.wrap_socket(raw_sock, server_hostname=parsed.hostname)
            raw_sock = None
        else:
            sock = raw_sock
            raw_sock = None
        with sock:
            sock.settimeout(timeout)
            sock.sendall(request)
            status, reason, response_headers, raw_headers, body_prefix = read_response_header(sock)
            if status == 100:
                sock.sendall(payload)
                status, reason, response_headers, raw_headers, body_prefix = read_response_header(sock)
            body = read_body(sock, response_headers, body_prefix)
    finally:
        if raw_sock is not None:
            raw_sock.close()
    return GameResponse(status, response_headers, raw_headers, body, extract_set_cookie(raw_headers))


def make_game_headers(path_hash: str, agent_id: int | str, number_header: int | None, cookie: str = "") -> dict[str, str]:
    headers = {
        "Content-Type": "application/json",
        "User-Agent": f"{path_hash}#{agent_id}",
        "charset": "UTF-8",
        "Mai-Encoding": "1.55",
        "Content-Encoding": "deflate",
        "Expect": "100-continue",
    }
    if number_header is not None:
        headers["number"] = str(number_header)
    if cookie:
        headers["Cookie"] = cookie
    return headers


def response_summary(response: GameResponse, sent_length: int, header_names: Iterable[str]) -> str:
    return (
        f"status={response.status_code} response_bytes={len(response.content)} "
        f"content_length_header={response.headers.get('content-length', 'absent')} "
        f"content_type={response.headers.get('content-type', 'absent')} "
        f"set_cookie_present={bool_text(bool(response.set_cookie))} "
        f"sent_content_length={sent_length} sent_header_names={','.join(header_names)}"
    )


def send_game_api(
    api_name: str,
    body: dict[str, Any],
    *,
    base_url: str,
    agent_id: int | str,
    cookie: str,
    number_header: int | None,
    timeout: float,
    verify_tls: bool,
    debug_http: bool,
    delay_stats: list[dict[str, Any]] | None = None,
) -> tuple[GameResponse, dict[str, Any]]:
    special = {
        "GetGameSettingApi": GAME_SETTING_PATH_HASH,
        "GetUserPreviewApi": PREVIEW_PATH_HASH,
        "UserLoginApi": LOGIN_PATH_HASH,
        "UserLogoutApi": LOGOUT_PATH_HASH,
        "Ping": PING_PATH_HASH,
    }
    path_hash = special.get(api_name) or route_hash(api_name)
    payload = encode_game_payload(body)
    headers = make_game_headers(path_hash, agent_id, number_header, cookie=cookie)
    started = time.monotonic()
    response = send_continue_post(build_url(base_url, path_hash), headers, payload, timeout, verify_tls)
    elapsed_ms = max(int(round((time.monotonic() - started) * 1000)), 0)
    if debug_http:
        print(f"http_trace api={api_name} path_hash={path_hash} {response_summary(response, len(payload), headers.keys())}")
    if response.status_code < 200 or response.status_code >= 300:
        raise FlowError(f"{api_name} HTTP status error: {response_summary(response, len(payload), headers.keys())}")
    try:
        decoded = decode_game_payload(response.content)
    except Exception as exc:
        raise FlowError(f"{api_name} response decode failed: {response_summary(response, len(payload), headers.keys())} detail={exc}") from exc
    if delay_stats is not None and api_name in API_KIND_INDEX:
        delay_stats.append({
            "apiName": api_name,
            "apiIndex": API_KIND_INDEX[api_name],
            "count": 1,
            "size": len(json.dumps(decoded, ensure_ascii=False, separators=(",", ":")).encode("utf-8")),
            "msec": elapsed_ms,
            "retry": 0,
        })
    return response, decoded


def join_base_uri(game_server_uri: str, movie_server_uri: str | None) -> str:
    if not movie_server_uri:
        return game_server_uri.rstrip("/") + "/"
    movie = movie_server_uri.strip()
    if urlparse(movie).scheme:
        return movie.rstrip("/") + "/"
    return game_server_uri.rstrip("/") + "/" + movie.strip("/") + "/"


def get_game_setting(args: argparse.Namespace, game_server_uri: str, place_id: int, client_id: str) -> tuple[str, dict[str, Any]]:
    _, data = send_game_api(
        "GetGameSettingApi",
        {"placeId": place_id, "clientId": client_id},
        base_url=game_server_uri,
        agent_id=client_id,
        cookie="",
        number_header=args.number_header,
        timeout=args.timeout,
        verify_tls=not args.no_verify_tls,
        debug_http=args.debug_http,
    )
    setting = data.get("gameSetting") if isinstance(data, dict) else {}
    movie = setting.get("movieServerUri") if isinstance(setting, dict) else None
    return join_base_uri(game_server_uri, str(movie).strip() if movie else None), data


def check_login_status(args: argparse.Namespace, base_url: str, client_id: str, identity: dict[str, Any]) -> dict[str, Any]:
    _, preview = send_game_api(
        "GetUserPreviewApi",
        {"userId": identity["userId"], "segaIdAuthKey": "", "token": identity["token"], "clientId": client_id},
        base_url=base_url,
        agent_id=identity["userId"],
        cookie="",
        number_header=args.number_header,
        timeout=args.timeout,
        verify_tls=not args.no_verify_tls,
        debug_http=args.debug_http,
    )
    is_login = bool(preview.get("isLogin"))
    print(
        "login_status_checked=true user_id={user_id} is_login={is_login} login_status={status} error_id={error_id} ban_state={ban_state}".format(
            user_id=preview.get("userId", identity["userId"]),
            is_login=bool_text(is_login),
            status="logged_in" if is_login else "released",
            error_id=preview.get("errorId", "absent"),
            ban_state=preview.get("banState", "absent"),
        )
    )
    return preview


def login(args: argparse.Namespace, base_url: str, client_id: str, identity: dict[str, Any], allnet: AllnetResult) -> dict[str, Any]:
    if not args.skip_preview:
        preview = check_login_status(args, base_url, client_id, identity)
        print(f"preview_success=true preview_is_login={bool_text(bool(preview.get('isLogin')))} rating={preview.get('playerRating', 'absent')}")
        if preview.get("isLogin"):
            raise FlowError("preview indicates account is already logged in; no session cookie is available for this standalone tool")
    login_date_time = args.login_date_time if args.login_date_time is not None else local_wall_clock_epoch()
    body = {
        "userId": identity["userId"],
        "accessCode": "",
        "regionId": allnet.region_id,
        "placeId": allnet.place_id,
        "clientId": client_id,
        "dateTime": allnet.auth_time_epoch,
        "loginDateTime": login_date_time,
        "isContinue": False,
        "genericFlag": 0,
        "token": identity["token"],
    }
    response, data = send_game_api(
        "UserLoginApi",
        body,
        base_url=base_url,
        agent_id=identity["userId"],
        cookie="",
        number_header=args.number_header,
        timeout=args.timeout,
        verify_tls=not args.no_verify_tls,
        debug_http=args.debug_http,
    )
    if data.get("returnCode") != 1:
        raise FlowError(f"login failed returnCode={data.get('returnCode', 'absent')}")
    cookie = response.set_cookie
    if not cookie:
        raise FlowError("login succeeded but Set-Cookie was absent")
    print("login_success=true")
    print(f"user_id={identity['userId']}")
    print(f"login_id={data.get('loginId', 'absent')}")
    print("cookie_present=true")
    return {
        "userId": identity["userId"],
        "loginId": data.get("loginId") or int(time.time()),
        "loginDateTime": login_date_time,
        "cookie": cookie,
    }


def paged_user_call(
    api_name: str,
    base_body: dict[str, Any],
    list_key: str,
    *,
    base_url: str,
    login_ctx: dict[str, Any],
    args: argparse.Namespace,
    delay_stats: list[dict[str, Any]],
    start_index: int = 0,
    max_count: int | None = None,
) -> dict[str, Any]:
    merged: list[Any] = []
    next_index = start_index
    last: dict[str, Any] = {}
    while True:
        body = dict(base_body)
        body["nextIndex"] = next_index
        if max_count is not None:
            body["maxCount"] = max_count
        _, last = send_game_api(
            api_name,
            body,
            base_url=base_url,
            agent_id=login_ctx["userId"],
            cookie=login_ctx["cookie"],
            number_header=args.number_header,
            timeout=args.timeout,
            verify_tls=not args.no_verify_tls,
            debug_http=args.debug_http,
            delay_stats=delay_stats,
        )
        value = last.get(list_key)
        if isinstance(value, list):
            merged.extend(value)
        next_index = coerce_int(last.get("nextIndex")) or 0
        if next_index == 0:
            break
    result = dict(last)
    result[list_key] = merged
    result["length"] = len(merged)
    result["nextIndex"] = 0
    return result


def fetch_full_data(args: argparse.Namespace, base_url: str, login_ctx: dict[str, Any], delay_stats: list[dict[str, Any]]) -> dict[str, Any]:
    uid = login_ctx["userId"]
    data: dict[str, Any] = {}
    simple_calls = [
        ("GetUserDataApi", {"userId": uid}),
        ("GetUserCharacterApi", {"userId": uid}),
        ("GetUserChargeApi", {"userId": uid}),
        ("GetUserGhostApi", {"userId": uid}),
        ("GetUserRegionApi", {"userId": uid}),
        ("GetUserRecommendRateMusicApi", {"userId": uid}),
        ("GetUserRecommendSelectMusicApi", {"userId": uid}),
        ("GetUserOptionApi", {"userId": uid}),
        ("GetUserExtendApi", {"userId": uid}),
        ("GetUserRatingApi", {"userId": uid}),
        ("GetUserActivityApi", {"userId": uid}),
        ("GetUserMissionDataApi", {"userId": uid}),
        ("GetUserFriendBonusApi", {"userId": uid}),
        ("GetUserIntimateApi", {"userId": uid}),
        ("GetUserKaleidxScopeApi", {"userId": uid}),
    ]
    for name, body in simple_calls:
        _, data[name] = send_game_api(
            name,
            body,
            base_url=base_url,
            agent_id=uid,
            cookie=login_ctx["cookie"],
            number_header=args.number_header,
            timeout=args.timeout,
            verify_tls=not args.no_verify_tls,
            debug_http=args.debug_http,
            delay_stats=delay_stats,
        )

    data["GetUserItemApi"] = {}
    for kind in USER_ITEM_KINDS:
        body = {"userId": uid, "nextIndex": kind * 10_000_000_000, "maxCount": 100}
        _, data["GetUserItemApi"][str(kind)] = send_game_api(
            "GetUserItemApi",
            body,
            base_url=base_url,
            agent_id=uid,
            cookie=login_ctx["cookie"],
            number_header=args.number_header,
            timeout=args.timeout,
            verify_tls=not args.no_verify_tls,
            debug_http=args.debug_http,
            delay_stats=delay_stats,
        )

    data["GetUserFavoriteApi"] = {}
    for kind in FAVORITE_KINDS:
        _, data["GetUserFavoriteApi"][str(kind)] = send_game_api(
            "GetUserFavoriteApi",
            {"userId": uid, "itemKind": kind},
            base_url=base_url,
            agent_id=uid,
            cookie=login_ctx["cookie"],
            number_header=args.number_header,
            timeout=args.timeout,
            verify_tls=not args.no_verify_tls,
            debug_http=args.debug_http,
            delay_stats=delay_stats,
        )

    data["GetUserFavoriteItemApi"] = {}
    for kind in FAVORITE_ITEM_KINDS:
        body = {"userId": uid, "kind": kind, "nextIndex": 0, "maxCount": 100, "isAllFavoriteItem": False}
        _, data["GetUserFavoriteItemApi"][str(kind)] = send_game_api(
            "GetUserFavoriteItemApi",
            body,
            base_url=base_url,
            agent_id=uid,
            cookie=login_ctx["cookie"],
            number_header=args.number_header,
            timeout=args.timeout,
            verify_tls=not args.no_verify_tls,
            debug_http=args.debug_http,
            delay_stats=delay_stats,
        )

    data["GetUserCourseApi"] = paged_user_call("GetUserCourseApi", {"userId": uid}, "userCourseList", base_url=base_url, login_ctx=login_ctx, args=args, delay_stats=delay_stats)
    data["GetUserMapApi"] = paged_user_call("GetUserMapApi", {"userId": uid}, "userMapList", base_url=base_url, login_ctx=login_ctx, args=args, delay_stats=delay_stats, max_count=1000)
    data["GetUserLoginBonusApi"] = paged_user_call("GetUserLoginBonusApi", {"userId": uid}, "userLoginBonusList", base_url=base_url, login_ctx=login_ctx, args=args, delay_stats=delay_stats, max_count=20)
    data["GetUserMusicApi"] = paged_user_call("GetUserMusicApi", {"userId": uid}, "userMusicList", base_url=base_url, login_ctx=login_ctx, args=args, delay_stats=delay_stats, max_count=50)
    _, data["GetUserShopStockApi"] = send_game_api(
        "GetUserShopStockApi",
        {"userId": uid, "shopItemIdList": SHOP_ITEM_IDS},
        base_url=base_url,
        agent_id=uid,
        cookie=login_ctx["cookie"],
        number_header=args.number_header,
        timeout=args.timeout,
        verify_tls=not args.no_verify_tls,
        debug_http=args.debug_http,
        delay_stats=delay_stats,
    )
    return data


def build_delay_log(delay_stats: list[dict[str, Any]]) -> dict[str, Any]:
    request = [{"count": 0, "size": 0, "msec": 0, "retry": 0} for _ in API_KIND_ORDER]
    for entry in delay_stats:
        index = entry.get("apiIndex")
        if not isinstance(index, int) or index < 0 or index >= len(request):
            continue
        slot = request[index]
        slot["count"] += int(entry.get("count") or 0)
        slot["size"] += int(entry.get("size") or 0)
        slot["msec"] += int(entry.get("msec") or 0)
        slot["retry"] += int(entry.get("retry") or 0)
    return {
        "dlRequests": sum(item["count"] for item in request),
        "dlSize": sum(item["size"] for item in request),
        "dlRetry": sum(item["retry"] for item in request),
        "loginMsec": sum(item["msec"] for item in request),
        "saveMsec": 0,
        "reductionMusic": 0,
        "reductionItem": 0,
        "request": request,
    }


def logout_once(
    args: argparse.Namespace,
    base_url: str,
    client_id: str,
    allnet: AllnetResult,
    login_ctx: dict[str, Any],
    delay_stats: list[dict[str, Any]],
) -> bool:
    body = {
        "userId": login_ctx["userId"],
        "accessCode": "",
        "regionId": allnet.region_id,
        "placeId": allnet.place_id,
        "clientId": client_id,
        "loginDateTime": login_ctx["loginDateTime"],
        "type": args.logout_type,
        "delayLog": build_delay_log(delay_stats),
    }
    _, data = send_game_api(
        "UserLogoutApi",
        body,
        base_url=base_url,
        agent_id=login_ctx["userId"],
        cookie=login_ctx["cookie"],
        number_header=args.number_header,
        timeout=args.timeout,
        verify_tls=not args.no_verify_tls,
        debug_http=args.debug_http,
    )
    ok = data.get("returnCode") == 1
    print(f"logout_attempted=true logout_success={bool_text(ok)} logout_return_code={data.get('returnCode', 'absent')}")
    return ok


def logout_with_retry(
    args: argparse.Namespace,
    base_url: str,
    client_id: str,
    identity: dict[str, Any],
    allnet: AllnetResult,
    login_ctx: dict[str, Any],
    delay_stats: list[dict[str, Any]],
) -> bool:
    attempts = max(args.logout_attempts, 1)
    for attempt in range(1, attempts + 1):
        print(f"logout_verify_attempt={attempt}/{attempts}")
        try:
            logout_once(args, base_url, client_id, allnet, login_ctx, delay_stats)
        except Exception as exc:  # noqa: BLE001 - retry path must continue to status check
            print(f"logout_attempted=true logout_success=false error={exc}")
        try:
            preview = check_login_status(args, base_url, client_id, identity)
            if not preview.get("isLogin"):
                print(f"logout_verified=true attempts={attempt}")
                return True
        except Exception as exc:  # noqa: BLE001 - QR may expire, but report clearly
            print(f"logout_check_success=false error={exc}")
        if attempt < attempts and args.retry_delay > 0:
            print(f"logout_retry_delay_seconds={args.retry_delay:g}")
            time.sleep(args.retry_delay)
    print(f"logout_verified=false attempts={attempts}")
    return False


def write_outputs(args: argparse.Namespace, user_id: int, full_data: dict[str, Any]) -> Path:
    output_dir = Path(args.output_dir).expanduser()
    output_dir.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    path = output_dir / f"sdgb155_full_dump_{user_id}_{stamp}.json"
    path.write_text(json.dumps(full_data, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Standalone QR -> full user data -> verified logout tool.")
    parser.add_argument("qr", nargs="?", default="", help="Raw SGWC QR/dummylogin content. Can also use --qr-content or stdin.")
    parser.add_argument("--qr-content", default=env("SDGB_QR_CONTENT", ""))
    parser.add_argument(
        "--keychip-id",
        "--keyship-id",
        dest="keychip_id",
        default=env("SDGB_KEYCHIP_ID", env("SDGB_KEYSHIP_ID", DEFAULT_KEYCHIP_ID)),
        help=f"Raw keychip/keyship id. Default: {DEFAULT_KEYCHIP_ID}",
    )
    parser.add_argument("--game-root", default=env("SDGB_GAME_ROOT"), help="Optional SDGB155 root/package path used only to read segatools.ini keychip id.")
    parser.add_argument("--client-id", default=env("SDGB_CLIENT_ID"))
    parser.add_argument("--title-id", default=env("SDGB_TITLE_ID", DEFAULT_TITLE_ID))
    parser.add_argument("--title-ver", default=env("SDGB_TITLE_VER", DEFAULT_TITLE_VER))
    parser.add_argument("--output-dir", default=env("SDGB_OUTPUT_DIR", str(DEFAULT_OUTPUT_DIR)))
    parser.add_argument("--timeout", type=float, default=float(env("SDGB_TIMEOUT", "60")))
    parser.add_argument("--number-header", type=int, default=int_env("SDGB_NUMBER_HEADER", DEFAULT_NUMBER_HEADER))
    parser.add_argument("--no-verify-tls", action="store_true")
    parser.add_argument("--debug-http", action="store_true")
    parser.add_argument("--print-json", action="store_true", help="Also print the full fetched JSON to stdout after logout verification.")

    parser.add_argument("--wc-aime-url", action="append", default=None)
    parser.add_argument("--chime-common-key", default=env("SDGB_CHIME_COMMON_KEY", DEFAULT_CHIME_COMMON_KEY))
    parser.add_argument("--chime-title-key", default=env("SDGB_CHIME_TITLE_KEY", DEFAULT_CHIME_TITLE_KEY))
    parser.add_argument("--chime-user-agent", default=env("SDGB_CHIME_USER_AGENT", DEFAULT_CHIME_USER_AGENT))
    parser.add_argument("--chime-timestamp", default=env("SDGB_CHIME_TIMESTAMP"))
    parser.add_argument("--chime-use-qr-timestamp", action="store_true")
    parser.add_argument("--chime-trust-env", action="store_true")

    parser.add_argument("--allnet-aes-key-hex", default=env("SDGB_ALLNET_AES_KEY_HEX", DEFAULT_ALLNET_AES_KEY_HEX))
    parser.add_argument("--allnet-initialize-url", default=env("SDGB_ALLNET_INITIALIZE_URL", DEFAULT_INITIALIZE_URL))
    parser.add_argument("--allnet-delivery-url", default=env("SDGB_ALLNET_DELIVERY_URL", DEFAULT_DELIVERY_URL))
    parser.add_argument("--allnet-connect-host", default=env("SDGB_ALLNET_CONNECT_HOST", DEFAULT_ALLNET_CONNECT_HOST))
    parser.add_argument("--allnet-user-agent", default=env("SDGB_ALLNET_USER_AGENT", DEFAULT_ALLNET_USER_AGENT))
    parser.add_argument("--allnet-game-server-path", default=env("SDGB_ALLNET_GAME_SERVER_PATH", DEFAULT_GAME_SERVER_PATH))
    parser.add_argument("--allnet-token", type=int, default=None)
    parser.add_argument("--skip-allnet-delivery", action="store_true")
    parser.add_argument("--base-url", default=env("SDGB_BASE_URL"), help="Optional final game API base URL. If supplied, ALL.Net and GetGameSetting are skipped.")
    parser.add_argument("--place-id", type=int, default=int_env("SDGB_PLACE_ID"))
    parser.add_argument("--region-id", type=int, default=int_env("SDGB_REGION_ID"))
    parser.add_argument("--auth-time", "--date-time", dest="auth_time", type=int, default=int_env("SDGB_AUTH_TIME", int_env("SDGB_DATE_TIME")))

    parser.add_argument("--login-date-time", type=int, default=int_env("SDGB_LOGIN_DATE_TIME"))
    parser.add_argument("--skip-preview", action="store_true")
    parser.add_argument("--logout-type", type=int, default=2, help="Default 2 for no-play dump flow.")
    parser.add_argument("--logout-attempts", type=int, default=3)
    parser.add_argument("--retry-delay", type=float, default=1.0)
    return parser.parse_args()


def stdin_qr_if_available() -> str:
    if sys.stdin.isatty():
        return ""
    return sys.stdin.read().strip()


def apply_defaults(args: argparse.Namespace) -> tuple[str, str, str]:
    local = safe_local_defaults(args.game_root)
    keychip_id = args.keychip_id or local.get("keychip_id") or ""
    title_id = args.title_id or local.get("title_id") or DEFAULT_TITLE_ID
    client_id = normalize_client_id(args.client_id) if args.client_id else client_id_from_keychip_id(keychip_id)
    qr_content = args.qr_content or args.qr or stdin_qr_if_available()
    if args.wc_aime_url is None:
        args.wc_aime_url = list(DEFAULT_WC_AIME_URLS)
    if not qr_content:
        raise FlowError("missing QR content")
    if not keychip_id:
        raise FlowError("missing keychip/keyship id; pass --keychip-id, set SDGB_KEYCHIP_ID, or pass --game-root")
    if not client_id:
        raise FlowError("could not derive 11-character clientId from keychip/keyship")
    args.title_id = title_id
    return qr_content, keychip_id, client_id


def main() -> int:
    args = parse_args()
    if args.logout_attempts < 1:
        print("success=false error=--logout-attempts must be >= 1")
        return 2

    login_ctx: dict[str, Any] | None = None
    identity: dict[str, Any] | None = None
    base_url = ""
    client_id = ""
    allnet: AllnetResult | None = None
    delay_stats: list[dict[str, Any]] = []
    full_data: dict[str, Any] | None = None
    json_path: Path | None = None
    logout_verified = False

    try:
        qr_content, keychip_id, client_id = apply_defaults(args)
        print(f"client_id={client_id}")

        identity = resolve_qr(qr_content, keychip_id, args)
        print(f"qr_resolved=true user_id={identity['userId']} token_present=true")

        if args.base_url:
            if args.place_id is None or args.region_id is None or args.auth_time is None:
                raise FlowError("--base-url mode also needs --place-id, --region-id and --auth-time")
            allnet = AllnetResult(
                initialize=AllnetExchange("manual", 0, 0, 0, {}),
                delivery=None,
                auth_time_epoch=args.auth_time,
                auth_time_local_wall_epoch=args.auth_time,
                game_server_uri=args.base_url,
                place_id=args.place_id,
                region_id=args.region_id,
            )
            base_url = args.base_url.rstrip("/") + "/"
            print("allnet_skipped=true reason=base_url_provided")
        else:
            allnet = initialize_allnet(args, client_id)
            print("allnet_success=true")
            print(f"place_id={allnet.place_id} region_id={allnet.region_id} auth_time={allnet.auth_time_epoch}")
            base_url, _ = get_game_setting(args, allnet.game_server_uri, allnet.place_id, client_id)
            print(f"base_url={base_url}")

        login_ctx = login(args, base_url, client_id, identity, allnet)
        full_data = fetch_full_data(args, base_url, login_ctx, delay_stats)
        json_path = write_outputs(args, login_ctx["userId"], full_data)
        music = full_data.get("GetUserMusicApi", {})
        music_count = len(music.get("userMusicList") or []) if isinstance(music, dict) else 0
        print("fetch_full_success=true")
        print(f"full_json_path={json_path}")
        print(f"user_music_count={music_count}")
    except Exception as exc:  # noqa: BLE001 - final block still attempts logout
        print(f"flow_error={exc}")
    finally:
        if login_ctx and identity and allnet and base_url:
            logout_verified = logout_with_retry(args, base_url, client_id, identity, allnet, login_ctx, delay_stats)

    success = bool(full_data is not None and json_path and logout_verified)
    print(f"flow_success={bool_text(success)}")
    if args.print_json and full_data is not None:
        print(json.dumps(full_data, ensure_ascii=False, indent=2, sort_keys=True))
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
