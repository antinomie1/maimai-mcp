"""QQ identity cache: local JSON + optional OneBot HTTP refresh (friends/groups)."""

from __future__ import annotations

import json
import os
import threading
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import httpx

from ..config import identityconfig, log


def cache_dir() -> Path:
    configured = identityconfig.qq_identity_cache_dir or os.environ.get(
        "QQ_IDENTITY_CACHE_DIR"
    )
    if configured:
        return Path(configured).expanduser().resolve()
    return (Path.cwd() / "qq-identity-cache").resolve()


def cache_path() -> Path:
    return cache_dir() / "identity_cache.json"


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def empty_cache() -> dict[str, Any]:
    return {
        "version": 1,
        "fetchedAt": None,
        "updatedAt": now_iso(),
        "source": "onebot",
        "groups": {},
        "users": {},
        "stats": {
            "friendCount": 0,
            "groupCount": 0,
            "groupMemberRows": 0,
            "uniqueUsers": 0,
        },
    }


def read_cache() -> dict[str, Any]:
    path = cache_path()
    if not path.exists():
        return empty_cache()
    try:
        parsed = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return empty_cache()
    if not isinstance(parsed, dict):
        return empty_cache()
    parsed.setdefault("groups", {})
    parsed.setdefault("users", {})
    parsed.setdefault("stats", {})
    return parsed


def build_stats(cache: dict[str, Any]) -> dict[str, int]:
    groups = cache.get("groups") if isinstance(cache.get("groups"), dict) else {}
    users = cache.get("users") if isinstance(cache.get("users"), dict) else {}
    friend_count = sum(
        1 for u in users.values() if isinstance(u, dict) and u.get("isFriend")
    )
    member_rows = 0
    for u in users.values():
        ug = u.get("groups") if isinstance(u, dict) else None
        if isinstance(ug, dict):
            member_rows += len(ug)
    return {
        "friendCount": friend_count,
        "groupCount": len(groups),
        "groupMemberRows": member_rows,
        "uniqueUsers": len(users),
    }


def write_cache(cache: dict[str, Any]) -> None:
    cache["updatedAt"] = now_iso()
    cache["stats"] = build_stats(cache)
    path = cache_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(f".{path.name}.{os.getpid()}.{threading.get_ident()}.tmp")
    tmp.write_text(
        json.dumps(cache, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    os.replace(tmp, path)


def _norm_id(value: Any) -> str | None:
    if isinstance(value, int):
        value = str(value)
    if not isinstance(value, str):
        return None
    value = value.strip()
    return value or None


def _opt_str(value: Any) -> str | None:
    if not isinstance(value, str):
        return None
    value = value.strip()
    return value or None


def _ensure_user(cache: dict[str, Any], qq: str) -> dict[str, Any]:
    users = cache.setdefault("users", {})
    user = users.get(qq)
    if not isinstance(user, dict):
        user = {"qq": qq, "groups": {}}
        users[qq] = user
    user.setdefault("qq", qq)
    user.setdefault("groups", {})
    return user


def upsert_group(
    cache: dict[str, Any], group_id: Any, group_name: Any = None
) -> dict[str, Any] | None:
    gid = _norm_id(group_id)
    if not gid:
        return None
    groups = cache.setdefault("groups", {})
    group = groups.get(gid)
    if not isinstance(group, dict):
        group = {"groupId": gid}
        groups[gid] = group
    group["groupId"] = gid
    if _opt_str(group_name):
        group["groupName"] = _opt_str(group_name)
    group["updatedAt"] = now_iso()
    return group


def upsert_friend(
    cache: dict[str, Any], qq: Any, nickname: Any = None
) -> dict[str, Any] | None:
    key = _norm_id(qq)
    if not key:
        return None
    user = _ensure_user(cache, key)
    user["isFriend"] = True
    if _opt_str(nickname):
        user["friendNickname"] = _opt_str(nickname)
        user.setdefault("qqNickname", _opt_str(nickname))
    user["friendUpdatedAt"] = now_iso()
    return user


def upsert_group_member(
    cache: dict[str, Any],
    *,
    group_id: Any,
    qq: Any,
    group_name: Any = None,
    nickname: Any = None,
    card: Any = None,
) -> dict[str, Any] | None:
    gid = _norm_id(group_id)
    key = _norm_id(qq)
    if not gid or not key:
        return None
    group = upsert_group(cache, gid, group_name)
    user = _ensure_user(cache, key)
    if _opt_str(nickname):
        user["qqNickname"] = _opt_str(nickname)
    display = _opt_str(card) or _opt_str(nickname) or key
    user.setdefault("groups", {})[gid] = {
        "groupId": gid,
        "groupName": group.get("groupName") if isinstance(group, dict) else _opt_str(group_name),
        "groupNickname": display,
        "card": _opt_str(card),
        "nickname": _opt_str(nickname),
        "updatedAt": now_iso(),
    }
    user["groupUpdatedAt"] = now_iso()
    return user


def looks_like_group_id(qq: int | str | None) -> bool:
    """True if id is listed only under cache.groups (not users)."""
    key = _norm_id(qq)
    if not key:
        return False
    cache = read_cache()
    groups = cache.get("groups") if isinstance(cache.get("groups"), dict) else {}
    users = cache.get("users") if isinstance(cache.get("users"), dict) else {}
    return key in groups and key not in users


def get_identity(qq: Any, group_id: Any = None) -> dict[str, Any] | None:
    key = _norm_id(qq)
    if not key:
        return None
    cache = read_cache()
    user = cache.get("users", {}).get(key)
    if not isinstance(user, dict):
        return None
    groups = user.get("groups") if isinstance(user.get("groups"), dict) else {}
    preferred = None
    gid = _norm_id(group_id)
    if gid and isinstance(groups.get(gid), dict):
        preferred = groups[gid]
    group_entries = [v for v in groups.values() if isinstance(v, dict)]
    return {
        "qq": key,
        "qqNickname": user.get("qqNickname"),
        "friendNickname": user.get("friendNickname"),
        "preferredGroup": preferred,
        "groups": sorted(
            group_entries,
            key=lambda i: (str(i.get("groupName") or ""), str(i.get("groupId") or "")),
        ),
        "waterfishNickname": user.get("waterfishNickname"),
        "waterfishUsername": user.get("waterfishUsername"),
        "isFriend": user.get("isFriend") is True,
        "cachePath": str(cache_path()),
    }


def list_group_members(group_id: Any) -> list[dict[str, Any]]:
    """Members of a group from identity cache (no network).

    Each item: userId, displayName, nickname, card, groupName.
    """
    gid = _norm_id(group_id)
    if not gid:
        return []
    cache = read_cache()
    users = cache.get("users") if isinstance(cache.get("users"), dict) else {}
    members: list[dict[str, Any]] = []
    for qq, user in users.items():
        if not isinstance(user, dict):
            continue
        groups = user.get("groups") if isinstance(user.get("groups"), dict) else {}
        entry = groups.get(gid)
        if not isinstance(entry, dict):
            continue
        display = (
            _opt_str(entry.get("groupNickname"))
            or _opt_str(entry.get("card"))
            or _opt_str(entry.get("nickname"))
            or _opt_str(user.get("qqNickname"))
            or qq
        )
        members.append(
            {
                "userId": str(qq),
                "displayName": display,
                "nickname": _opt_str(entry.get("nickname"))
                or _opt_str(user.get("qqNickname")),
                "card": _opt_str(entry.get("card")),
                "groupName": _opt_str(entry.get("groupName")),
            }
        )
    members.sort(key=lambda m: m["userId"])
    return members


def list_user_group_ids(qq: Any) -> list[str]:
    """Group ids a QQ belongs to in identity cache."""
    key = _norm_id(qq)
    if not key:
        return []
    cache = read_cache()
    user = cache.get("users", {}).get(key)
    if not isinstance(user, dict):
        return []
    groups = user.get("groups") if isinstance(user.get("groups"), dict) else {}
    return sorted(str(gid) for gid, entry in groups.items() if isinstance(entry, dict))


def infer_group_id(qq: Any) -> str | None:
    """If the QQ is in exactly one cached group, return that group id."""
    groups = list_user_group_ids(qq)
    if len(groups) == 1:
        return groups[0]
    return None


def upsert_waterfish_profile(
    qq: Any,
    *,
    nickname: Any = None,
    username: Any = None,
) -> None:
    """Best-effort write of Diving-Fish display name into identity cache."""
    key = _norm_id(qq)
    if not key:
        return
    nick = _opt_str(nickname)
    uname = _opt_str(username)
    if not nick and not uname:
        return
    try:
        cache = read_cache()
        user = cache.get("users", {}).get(key)
        if not isinstance(user, dict):
            # Do not create orphan users without group/friend context.
            return
        changed = False
        if nick and user.get("waterfishNickname") != nick:
            user["waterfishNickname"] = nick
            changed = True
        if uname and user.get("waterfishUsername") != uname:
            user["waterfishUsername"] = uname
            changed = True
        if changed:
            write_cache(cache)
    except Exception:
        return


def cache_status() -> dict[str, Any]:
    cache = read_cache()
    return {
        "cacheExists": cache_path().exists(),
        "cachePath": str(cache_path()),
        "fetchedAt": cache.get("fetchedAt"),
        "updatedAt": cache.get("updatedAt"),
        "source": cache.get("source"),
        "stats": cache.get("stats") or build_stats(cache),
        "onebotBaseUrl": identityconfig.effective_onebot_base_url(),
    }


def _score_name(value: Any, q: str) -> int:
    if not isinstance(value, str) or not value.strip():
        return 0
    n = value.strip().casefold()
    if n == q:
        return 100
    if q in n:
        return 50
    return 0


def resolve_identities(
    query: Any, *, group_id: Any = None, max_results: int = 10
) -> dict[str, Any]:
    raw = str(query).strip() if query is not None else ""
    if not raw:
        return {"query": raw, "matches": [], "ambiguous": False}
    q = raw.casefold()
    cache = read_cache()
    users = cache.get("users") if isinstance(cache.get("users"), dict) else {}
    matches: list[dict[str, Any]] = []
    for qq_key, user in users.items():
        if not isinstance(user, dict):
            continue
        snap = get_identity(qq_key, group_id)
        if not snap:
            continue
        score = 0
        fields: list[str] = []
        if raw == str(snap.get("qq") or ""):
            score, fields = 200, ["qq"]
        else:
            for name, val in (
                ("qqNickname", snap.get("qqNickname")),
                ("friendNickname", snap.get("friendNickname")),
                ("waterfishNickname", snap.get("waterfishNickname")),
                ("waterfishUsername", snap.get("waterfishUsername")),
            ):
                s = _score_name(val, q)
                if s:
                    score = max(score, s)
                    fields.append(name)
            pref = snap.get("preferredGroup")
            if isinstance(pref, dict):
                for key in ("groupNickname", "card", "nickname"):
                    s = _score_name(pref.get(key), q)
                    if s:
                        score = max(score, s + 10)
                        fields.append(f"preferredGroup.{key}")
            for g in snap.get("groups") or []:
                if not isinstance(g, dict):
                    continue
                for key in ("groupNickname", "card", "nickname"):
                    s = _score_name(g.get(key), q)
                    if s:
                        score = max(score, s)
                        fields.append(f"group.{key}")
        if score <= 0:
            continue
        snap["matchScore"] = score
        snap["matchedFields"] = sorted(set(fields))
        matches.append(snap)
    matches.sort(key=lambda i: (-int(i.get("matchScore") or 0), str(i.get("qq") or "")))
    limited = matches[: max(1, max_results)]
    amb = False
    if len(limited) > 1:
        amb = int(limited[0].get("matchScore") or 0) == int(
            limited[1].get("matchScore") or 0
        )
    return {
        "query": raw,
        "groupId": _norm_id(group_id),
        "matches": limited,
        "ambiguous": amb,
        "cachePath": str(cache_path()),
        "cacheExists": cache_path().exists(),
    }


# --- OneBot HTTP pull ---


class IdentityRefreshError(Exception):
    def __init__(self, message: str, *, code: str = "identity_refresh_error") -> None:
        super().__init__(message)
        self.message = message
        self.code = code


def _onebot_headers() -> dict[str, str]:
    headers = {
        "Accept": "application/json",
        "Content-Type": "application/json",
        "User-Agent": "maimai-mcp-identity/1.0",
    }
    token = identityconfig.onebot_access_token
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


async def _onebot_post(
    client: httpx.AsyncClient, base: str, path: str, payload: dict[str, Any]
) -> Any:
    url = f"{base.rstrip('/')}/{path.lstrip('/')}"
    try:
        resp = await client.post(url, json=payload, headers=_onebot_headers())
    except httpx.TimeoutException as e:
        raise IdentityRefreshError(
            f"OneBot 请求超时：{path}", code="onebot_timeout"
        ) from e
    except httpx.HTTPError as e:
        raise IdentityRefreshError(
            f"OneBot 网络错误：{path}: {e}", code="onebot_network"
        ) from e
    if resp.status_code >= 400:
        raise IdentityRefreshError(
            f"OneBot HTTP {resp.status_code}：{path}",
            code="onebot_http",
        )
    try:
        data = resp.json()
    except Exception as e:
        raise IdentityRefreshError(
            f"OneBot 返回非 JSON：{path}", code="onebot_json"
        ) from e
    if isinstance(data, list):
        return data
    if not isinstance(data, dict):
        raise IdentityRefreshError(
            f"OneBot 响应格式错误：{path}", code="onebot_json"
        )
    # OneBot 11: { status, retcode, data }
    if data.get("retcode") not in (None, 0):
        raise IdentityRefreshError(
            f"OneBot 业务错误：{data.get('message') or data.get('wording') or data.get('retcode')}",
            code="onebot_retcode",
        )
    return data.get("data", data)


def _as_list(data: Any) -> list[Any]:
    if isinstance(data, list):
        return data
    return []


async def refresh_identity_cache(
    *,
    base_url: str | None = None,
    no_cache: bool = True,
    timeout_ms: int = 10000,
    group_delay_ms: int | None = None,
    max_groups: int | None = None,
) -> dict[str, Any]:
    """Pull friend list + groups + members from OneBot HTTP API and write cache."""
    base = (base_url or identityconfig.effective_onebot_base_url() or "").strip()
    if not base:
        raise IdentityRefreshError(
            "未配置 NapCat 地址。请设置 NAPCAT_BASE_URL（或 ONEBOT_BASE_URL）。",
            code="onebot_not_configured",
        )
    delay = (
        group_delay_ms
        if group_delay_ms is not None
        else identityconfig.qq_identity_group_delay_ms
    )
    timeout = timeout_ms / 1000.0
    cache = empty_cache()

    async with httpx.AsyncClient(timeout=timeout) as client:
        friends_raw = _as_list(
            await _onebot_post(client, base, "get_friend_list", {})
        )
        for item in friends_raw:
            if not isinstance(item, dict):
                continue
            uid = item.get("user_id", item.get("userId"))
            upsert_friend(cache, uid, item.get("nickname"))

        groups_raw = _as_list(
            await _onebot_post(client, base, "get_group_list", {})
        )
        groups: list[dict[str, Any]] = []
        for item in groups_raw:
            if not isinstance(item, dict):
                continue
            gid = item.get("group_id", item.get("groupId"))
            gname = item.get("group_name", item.get("groupName"))
            if _norm_id(gid):
                groups.append({"groupId": _norm_id(gid), "groupName": _opt_str(gname)})
                upsert_group(cache, gid, gname)

        if max_groups is not None:
            groups = groups[: max(1, max_groups)]

        for index, g in enumerate(groups, start=1):
            gid = g["groupId"]
            payload: dict[str, Any] = {
                "group_id": int(gid) if str(gid).isdigit() else gid,
                "no_cache": no_cache,
            }
            members_raw = _as_list(
                await _onebot_post(client, base, "get_group_member_list", payload)
            )
            for item in members_raw:
                if not isinstance(item, dict):
                    continue
                uid = item.get("user_id", item.get("userId"))
                upsert_group_member(
                    cache,
                    group_id=gid,
                    group_name=g.get("groupName"),
                    qq=uid,
                    nickname=item.get("nickname"),
                    card=item.get("card"),
                )
            log.info(
                f"身份缓存：已拉取群 {index}/{len(groups)} "
                f"({g.get('groupName') or gid}) 成员 {len(members_raw)}"
            )
            if delay > 0 and index < len(groups):
                time.sleep(delay / 1000.0)

    cache["fetchedAt"] = now_iso()
    cache["source"] = base
    write_cache(cache)
    status = cache_status()
    log.success(
        f"身份缓存已更新：好友 {status['stats'].get('friendCount')}，"
        f"群 {status['stats'].get('groupCount')}，"
        f"唯一用户 {status['stats'].get('uniqueUsers')}"
    )
    return status
