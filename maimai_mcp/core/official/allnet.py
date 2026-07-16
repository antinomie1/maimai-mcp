"""ALL.Net initialize / delivery for official title auth context."""

from __future__ import annotations

import os
import random
import re
import socket
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from typing import Any
from urllib.parse import unquote

from .aes_cbc import aes_cbc_decrypt, aes_cbc_encrypt

AES_BLOCK = 16
DEFAULT_INITIALIZE_URL = "http://at.sys-all.cn/net/initialize"
DEFAULT_DELIVERY_URL = "http://at.sys-all.cn/net/delivery/instruction"
DEFAULT_ALLNET_CONNECT_HOST = "at.sys-allnet.cn"
DEFAULT_ALLNET_USER_AGENT = "SDGB;Windows/Lite"
DEFAULT_ALLNET_AES_KEY_HEX = "2f3f6a6f2b224c265c437239283d6b47"
DEFAULT_TITLE_ID = "SDGB"
DEFAULT_TITLE_VER = "1.55.00"
DEFAULT_GAME_SERVER_PATH = "Maimai2Servlet"


class AllnetError(RuntimeError):
    pass


@dataclass(slots=True)
class AllnetResult:
    auth_time_epoch: int
    auth_time_local_wall_epoch: int
    game_server_uri: str
    place_id: int
    region_id: int


def _pkcs7_pad(data: bytes) -> bytes:
    n = AES_BLOCK - (len(data) % AES_BLOCK)
    return data + bytes([n]) * n


def _pkcs7_unpad(data: bytes) -> bytes:
    if not data:
        raise ValueError("empty AES plaintext")
    n = data[-1]
    if n < 1 or n > AES_BLOCK or data[-n:] != bytes([n]) * n:
        raise ValueError("bad PKCS7 padding")
    return data[:-n]


def parse_aes_key_hex(value: str | None) -> bytes:
    text = re.sub(r"[^0-9A-Fa-f]", "", value or "")
    if len(text) != 32:
        raise AllnetError("ALL.Net AES key must be 16 bytes / 32 hex characters")
    return bytes.fromhex(text)


def encrypt_allnet_body(plaintext: bytes, key: bytes) -> bytes:
    iv = os.urandom(AES_BLOCK)
    return iv + aes_cbc_encrypt(key, iv, _pkcs7_pad(plaintext))


def decrypt_allnet_body(wire_body: bytes, key: bytes) -> bytes:
    if len(wire_body) < AES_BLOCK * 2 or len(wire_body) % AES_BLOCK:
        raise AllnetError(f"invalid encrypted ALL.Net body length: {len(wire_body)}")
    iv = wire_body[:AES_BLOCK]
    return _pkcs7_unpad(aes_cbc_decrypt(key, iv, wire_body[AES_BLOCK:]))


def parse_allnet_form(text: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    stripped = text.strip("\ufeff \t\r\n\0")
    for part in stripped.replace("\r", "\n").replace("\n", "&").split("&"):
        if not part or "=" not in part:
            continue
        key, value = part.split("=", 1)
        fields[unquote(key)] = unquote(value)
    return fields


def _parse_http_response_header(
    raw_header: bytes,
) -> tuple[int, str, dict[str, str]]:
    lines = raw_header.decode("iso-8859-1", errors="replace").split("\r\n")
    status_line = lines[0] if lines else ""
    parts = status_line.split(" ", 2)
    status = int(parts[1]) if len(parts) >= 2 and parts[1].isdigit() else 0
    reason = parts[2] if len(parts) >= 3 else ""
    headers: dict[str, str] = {}
    for line in lines[1:]:
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        headers[key.strip().lower()] = value.strip()
    return status, reason, headers


def raw_http10_post(
    url: str,
    user_agent: str,
    body: bytes,
    timeout: float,
    connect_host: str | None,
) -> tuple[int, bytes]:
    from urllib.parse import urlparse

    parsed = urlparse(url)
    if parsed.scheme.lower() != "http" or not parsed.hostname:
        raise AllnetError("ALL.Net URL must be plain http:// with host")
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
        raise AllnetError(f"ALL.Net HTTP response had no header, bytes={len(response)}")
    status, _reason, _headers = _parse_http_response_header(response[:marker])
    return status, response[marker + 4 :]


def send_allnet_exchange(
    *,
    endpoint: str,
    url: str,
    user_agent: str,
    plaintext: str,
    aes_key: bytes,
    timeout: float,
    connect_host: str | None,
) -> dict[str, str]:
    wire_body = encrypt_allnet_body(plaintext.encode("ascii"), aes_key)
    status, body = raw_http10_post(url, user_agent, wire_body, timeout, connect_host)
    if status < 200 or status >= 300:
        raise AllnetError(f"{endpoint} HTTP status {status}")
    plain = decrypt_allnet_body(body, aes_key).decode("utf-8", errors="replace")
    return parse_allnet_form(plain)


def normalize_allnet_title_ver(value: str | None) -> str:
    text = (value or "").strip()
    match = re.fullmatch(r"(\d+\.\d+)\.0+", text)
    if match:
        return match.group(1)
    return text


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
    return timezone(
        sign * timedelta(hours=int(match.group(2)), minutes=int(match.group(3)))
    )


def local_wall_epoch_from_utc(value: str | None, timezone_text: str | None) -> int | None:
    true_epoch = parse_utc_time_epoch(value)
    if true_epoch is None:
        return None
    local_dt = (
        datetime.fromtimestamp(true_epoch, timezone.utc)
        .astimezone(parse_timezone_offset(timezone_text))
        .replace(tzinfo=None)
    )
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


def coerce_int(value: Any) -> int | None:
    if value in (None, ""):
        return None
    try:
        return int(str(value), 10)
    except ValueError:
        return None


def join_uri_path(base_uri: str, path: str) -> str:
    return base_uri.rstrip("/") + "/" + path.strip("/") + "/"


def initialize_allnet(
    client_id: str,
    *,
    title_id: str = DEFAULT_TITLE_ID,
    title_ver: str = DEFAULT_TITLE_VER,
    timeout: float = 30.0,
    initialize_url: str = DEFAULT_INITIALIZE_URL,
    delivery_url: str = DEFAULT_DELIVERY_URL,
    connect_host: str = DEFAULT_ALLNET_CONNECT_HOST,
    user_agent: str = DEFAULT_ALLNET_USER_AGENT,
    aes_key_hex: str = DEFAULT_ALLNET_AES_KEY_HEX,
    game_server_path: str = DEFAULT_GAME_SERVER_PATH,
    skip_delivery: bool = False,
) -> AllnetResult:
    aes_key = parse_aes_key_hex(aes_key_hex)
    token = random.randrange(0, 2**32)
    ver = normalize_allnet_title_ver(title_ver)
    init_plain = (
        f"title_id={title_id}&title_ver={ver}&client_id={client_id}&token={token}\r\n"
    )
    init = send_allnet_exchange(
        endpoint="initialize",
        url=initialize_url,
        user_agent=user_agent,
        plaintext=init_plain,
        aes_key=aes_key,
        timeout=timeout,
        connect_host=connect_host or None,
    )
    if init.get("result") != "1":
        raise AllnetError(f"ALL.Net initialize returned result={init.get('result', 'absent')}")

    if not skip_delivery:
        delivery_plain = f"title_id={title_id}&title_ver={ver}&client_id={client_id}\r\n"
        delivery = send_allnet_exchange(
            endpoint="delivery",
            url=delivery_url,
            user_agent=user_agent,
            plaintext=delivery_plain,
            aes_key=aes_key,
            timeout=timeout,
            connect_host=connect_host or None,
        )
        if delivery.get("result") != "1":
            raise AllnetError(
                f"ALL.Net delivery returned result={delivery.get('result', 'absent')}"
            )

    auth_epoch = parse_utc_time_epoch(init.get("utc_time"))
    auth_wall = local_wall_epoch_from_utc(init.get("utc_time"), init.get("client_timezone"))
    place_id = parse_allnet_place_id(init.get("place_id"))
    region_id = coerce_int(init.get("region0"))
    uri1 = init.get("uri1")
    if auth_epoch is None or auth_wall is None or place_id is None or region_id is None or not uri1:
        raise AllnetError("ALL.Net initialize response missed auth/place/region/uri1 fields")
    return AllnetResult(
        auth_time_epoch=auth_epoch,
        auth_time_local_wall_epoch=auth_wall,
        game_server_uri=join_uri_path(uri1, game_server_path),
        place_id=place_id,
        region_id=region_id,
    )


def local_wall_clock_epoch() -> int:
    return int((datetime.now() - datetime(1970, 1, 1)).total_seconds())
