"""Group rating / song ranking: fetch on query, reuse fresh local cache."""

from __future__ import annotations

import asyncio
import os
from typing import Any, Literal

from ...core.clients.divingfish.client import DivingFishAPI
from ...core.clients.exceptions import MusicNotPlayError
from ...core.domain.chart import resolve_song
from ...core.errors import ValidationError, handle_errors
from ...core.merge.play_result import df_to_playresult
from ...core.merge.player import df_to_player
from ...core.player_cache import (
    chart_to_cache_dict,
    get_rating,
    get_rating_if_fresh,
    get_song_entry,
    get_song_entry_if_fresh,
    write_rating,
    write_song_scores,
)
from ...core.qq_identity_store import (
    get_identity,
    infer_group_id,
    list_group_members,
    list_user_group_ids,
    resolve_identities,
    upsert_waterfish_profile,
)

SortOrder = Literal["asc", "desc"]
SongSortBy = Literal["achievements", "rating", "dxScore"]


def _env_int(name: str, default: int, *, low: int, high: int) -> int:
    raw = os.environ.get(name)
    if raw is None or not str(raw).strip() or not str(raw).strip().isdigit():
        return default
    return max(low, min(high, int(str(raw).strip())))


def _default_concurrency() -> int:
    return _env_int("GROUP_RANK_MAX_CONCURRENCY", 3, low=1, high=10)


def _default_delay_ms() -> int:
    return _env_int("GROUP_RANK_QUERY_DELAY_MS", 250, low=0, high=5000)


def _display_name(member: dict[str, Any], qq: str, group_id: str | None) -> str:
    if member.get("displayName"):
        return str(member["displayName"])
    ident = get_identity(qq, group_id)
    if ident:
        pref = ident.get("preferredGroup")
        if isinstance(pref, dict):
            for key in ("groupNickname", "card", "nickname"):
                if pref.get(key):
                    return str(pref[key])
        for key in ("qqNickname", "friendNickname", "waterfishNickname"):
            if ident.get(key):
                return str(ident[key])
    return qq


def _apply_window(
    rows: list[dict[str, Any]],
    *,
    output_limit: int | None,
    start_rank: int | None,
    end_rank: int | None,
) -> list[dict[str, Any]]:
    if start_rank is not None and end_rank is not None:
        start = max(0, start_rank - 1)
        end = min(len(rows), end_rank)
        sliced = rows[start:end]
        return [{**row, "rank": start + i + 1} for i, row in enumerate(sliced)]
    if output_limit is not None:
        sliced = rows[:output_limit]
        return [{**row, "rank": i + 1} for i, row in enumerate(sliced)]
    return [{**row, "rank": i + 1} for i, row in enumerate(rows)]


def _validate_rank_window(
    start_rank: int | None, end_rank: int | None
) -> None:
    if (start_rank is None) != (end_rank is None):
        raise ValidationError("start_rank 与 end_rank 须同时提供。")
    if start_rank is not None and end_rank is not None and start_rank > end_rank:
        raise ValidationError("start_rank 不能大于 end_rank。")


def _resolve_group_and_qq(
    *,
    group_id: int | str | None,
    qq: int | str | None,
    target: str | None,
) -> tuple[str, str | None]:
    gid = str(group_id).strip() if group_id is not None else None
    q = str(qq).strip() if qq is not None else None

    if not q and target:
        resolved = resolve_identities(target, group_id=gid, max_results=10)
        matches = resolved.get("matches") or []
        if not matches:
            raise ValidationError(
                f"身份缓存中未找到「{target}」。可先 maimai_refresh_identity，或直接传 qq。"
            )
        if resolved.get("ambiguous") or (
            len(matches) > 1
            and matches[0].get("matchScore") == matches[1].get("matchScore")
        ):
            candidates = [
                {
                    "qq": m.get("qq"),
                    "qqNickname": m.get("qqNickname"),
                    "groups": m.get("groups"),
                }
                for m in matches[:8]
            ]
            raise ValidationError(
                f"「{target}」匹配到多个 QQ，请改用明确 qq。候选：{candidates}"
            )
        q = str(matches[0].get("qq") or "")
        if not q:
            raise ValidationError(f"「{target}」没有可用 QQ。")

    if not gid and q:
        inferred = infer_group_id(q)
        if inferred:
            gid = inferred
        else:
            groups = list_user_group_ids(q)
            if not groups:
                raise ValidationError(
                    f"QQ {q} 不在任何群身份缓存中。请先 maimai_refresh_identity，或传入 group_id。"
                )
            raise ValidationError(
                f"QQ {q} 属于多个群 {groups}，请指定 group_id。"
            )

    if not gid:
        raise ValidationError("必须提供 group_id（或可从唯一群归属推断的 qq/target）。")
    return gid, q or None


async def _fetch_rating_remote(qq: str) -> dict[str, Any] | None:
    """Query Diving-Fish B50 for one QQ; write cache on success."""
    try:
        api = DivingFishAPI(qqid=int(qq))
        userinfo = await api.query_user_b50()
        player = df_to_player(userinfo)
        if not isinstance(player.rating, (int, float)) or player.rating <= 0:
            return None
        write_rating(
            qq,
            rating=int(player.rating),
            name=player.name,
            source="divingfish",
        )
        upsert_waterfish_profile(qq, nickname=player.name)
        return get_rating(qq)
    except Exception:
        return None


async def _fetch_song_remote(qq: str, song: Any) -> dict[str, Any] | None:
    """Query Diving-Fish single-song scores; write cache on success."""
    try:
        api = DivingFishAPI(qqid=int(qq))
        data = await api.query_user_post_dev(song_id=song.song_id)
        if not data:
            return None
        charts = df_to_playresult(data, song=song)
        if not charts:
            return None
        write_song_scores(
            qq,
            song_id=song.song_id,
            song_name=song.song_name,
            charts=[chart_to_cache_dict(c) for c in charts],
            source="divingfish",
        )
        return get_song_entry(qq, song.song_id)
    except MusicNotPlayError:
        return None
    except Exception:
        return None


async def _map_limited(
    items: list[str],
    *,
    concurrency: int,
    delay_ms: int,
    worker,
) -> dict[str, Any]:
    """Run async worker(qq) with concurrency + inter-start delay."""
    sem = asyncio.Semaphore(max(1, concurrency))
    results: dict[str, Any] = {}
    lock = asyncio.Lock()

    async def run_one(index: int, qq: str) -> None:
        if delay_ms > 0 and index > 0:
            await asyncio.sleep((delay_ms * index) / 1000.0)
        async with sem:
            value = await worker(qq)
        async with lock:
            results[qq] = value

    await asyncio.gather(*(run_one(i, qq) for i, qq in enumerate(items)))
    return results


async def _ensure_ratings(
    qqs: list[str],
    *,
    force_refresh: bool,
    concurrency: int,
    delay_ms: int,
) -> tuple[dict[str, dict[str, Any]], int, int]:
    """Return rating docs by qq; fetch only missing/stale. (docs, cache_hits, fetched)."""
    docs: dict[str, dict[str, Any]] = {}
    need: list[str] = []
    for qq in qqs:
        hit = get_rating_if_fresh(qq, force=force_refresh)
        if hit:
            docs[qq] = hit
        else:
            need.append(qq)

    if not need:
        return docs, len(docs), 0

    fetched = await _map_limited(
        need,
        concurrency=concurrency,
        delay_ms=delay_ms,
        worker=_fetch_rating_remote,
    )
    remote_ok = 0
    for qq, doc in fetched.items():
        if isinstance(doc, dict):
            docs[qq] = doc
            remote_ok += 1
    return docs, len(qqs) - len(need), remote_ok


async def _ensure_songs(
    qqs: list[str],
    song: Any,
    *,
    force_refresh: bool,
    concurrency: int,
    delay_ms: int,
) -> tuple[dict[str, dict[str, Any]], int, int]:
    docs: dict[str, dict[str, Any]] = {}
    need: list[str] = []
    song_id = int(song.song_id)
    for qq in qqs:
        hit = get_song_entry_if_fresh(qq, song_id, force=force_refresh)
        if hit:
            docs[qq] = hit
        else:
            need.append(qq)

    if not need:
        return docs, len(docs), 0

    async def worker(qq: str) -> dict[str, Any] | None:
        return await _fetch_song_remote(qq, song)

    fetched = await _map_limited(
        need,
        concurrency=concurrency,
        delay_ms=delay_ms,
        worker=worker,
    )
    remote_ok = 0
    for qq, doc in fetched.items():
        if isinstance(doc, dict):
            docs[qq] = doc
            remote_ok += 1
    return docs, len(qqs) - len(need), remote_ok


@handle_errors
async def query_group_rating_rank(
    group_id: int | str,
    *,
    sort_order: SortOrder = "desc",
    output_limit: int | None = 20,
    start_rank: int | None = None,
    end_rank: int | None = None,
    rating_min: int | None = None,
    rating_max: int | None = None,
    force_refresh: bool = False,
    max_concurrency: int | None = None,
    query_delay_ms: int | None = None,
    max_members: int | None = None,
) -> dict[str, Any]:
    """Rank group members by rating; fetch missing scores only when this is called."""
    gid = str(group_id).strip()
    if not gid:
        raise ValidationError("必须提供 group_id。")
    _validate_rank_window(start_rank, end_rank)
    if sort_order not in ("asc", "desc"):
        raise ValidationError("sort_order 须为 asc 或 desc。")

    members = list_group_members(gid)
    if not members:
        raise ValidationError(
            f"群 {gid} 在身份缓存中没有成员。请先 maimai_refresh_identity。"
        )
    if max_members is not None:
        if max_members < 1:
            raise ValidationError("max_members 须为正整数。")
        members = members[:max_members]

    concurrency = (
        max(1, min(10, max_concurrency))
        if max_concurrency is not None
        else _default_concurrency()
    )
    delay_ms = (
        max(0, min(5000, query_delay_ms))
        if query_delay_ms is not None
        else _default_delay_ms()
    )

    qqs = [str(m["userId"]) for m in members]
    rating_by_qq, cache_hits, fetched = await _ensure_ratings(
        qqs,
        force_refresh=force_refresh,
        concurrency=concurrency,
        delay_ms=delay_ms,
    )

    rows: list[dict[str, Any]] = []
    skipped = 0
    member_by_qq = {str(m["userId"]): m for m in members}
    for qq in qqs:
        rating_doc = rating_by_qq.get(qq)
        if not rating_doc:
            skipped += 1
            continue
        value = rating_doc.get("value")
        if not isinstance(value, (int, float)) or value <= 0:
            skipped += 1
            continue
        ivalue = int(value)
        if rating_min is not None and ivalue < rating_min:
            continue
        if rating_max is not None and ivalue > rating_max:
            continue
        m = member_by_qq.get(qq, {"userId": qq})
        rows.append(
            {
                "qq": qq,
                "displayName": _display_name(m, qq, gid),
                "rating": ivalue,
                "playerName": rating_doc.get("name"),
                "fetchedAt": rating_doc.get("fetchedAt"),
                "source": rating_doc.get("source"),
            }
        )

    reverse = sort_order == "desc"
    rows.sort(
        key=lambda r: (r["rating"], r.get("playerName") or "", r["qq"]),
        reverse=reverse,
    )
    ranked_total = len(rows)
    windowed = _apply_window(
        rows,
        output_limit=output_limit if start_rank is None else None,
        start_rank=start_rank,
        end_rank=end_rank,
    )

    order_label = "从高到低" if reverse else "从低到高"
    lines = [
        f"群 {gid} Rating 榜（{order_label}）",
        (
            f"群成员 {len(members)} 人，上榜 {ranked_total} 人，"
            f"跳过/无成绩 {skipped} 人；缓存命中 {cache_hits}，本次拉取 {fetched}"
        ),
        "说明：查榜时按需拉分；新鲜缓存会复用，不在后台预拉全群。",
        "",
    ]
    if not windowed:
        lines.append("当前没有可展示的成绩（可能未绑定查分器或隐私关闭）。")
    else:
        for row in windowed:
            name = row["displayName"]
            pname = f" / {row['playerName']}" if row.get("playerName") else ""
            lines.append(
                f"#{row['rank']}  {row['rating']}  {name}{pname}  ({row['qq']})"
            )

    return {
        "groupId": gid,
        "sortOrder": sort_order,
        "memberCount": len(members),
        "rankedCount": ranked_total,
        "skippedCount": skipped,
        "cacheHits": cache_hits,
        "fetchedCount": fetched,
        "forceRefresh": force_refresh,
        "rows": windowed,
        "text": "\n".join(lines),
    }


@handle_errors
async def query_group_song_rank(
    group_id: int | str,
    song_key: str,
    *,
    level_index: int | None = None,
    sort_by: SongSortBy = "achievements",
    sort_order: SortOrder = "desc",
    output_limit: int | None = 20,
    start_rank: int | None = None,
    end_rank: int | None = None,
    force_refresh: bool = False,
    max_concurrency: int | None = None,
    query_delay_ms: int | None = None,
    max_members: int | None = None,
) -> dict[str, Any]:
    """Rank group on one song; fetch per-member song scores only when called."""
    gid = str(group_id).strip()
    if not gid:
        raise ValidationError("必须提供 group_id。")
    _validate_rank_window(start_rank, end_rank)
    if sort_by not in ("achievements", "rating", "dxScore"):
        raise ValidationError("sort_by 须为 achievements / rating / dxScore。")
    if sort_order not in ("asc", "desc"):
        raise ValidationError("sort_order 须为 asc 或 desc。")
    if level_index is not None and not (0 <= level_index <= 4):
        raise ValidationError("level_index 须为 0–4。")

    song = resolve_song(song_key)
    if song is None:
        raise ValidationError(f"未找到曲目：{song_key}")
    if isinstance(song, list):
        raise ValidationError(
            "找到多个曲目，请用 ID：\n"
            + "\n".join(f"{s.song_id}：{s.song_name}" for s in song if s)
        )
    song_id = int(song.song_id)
    song_name = song.song_name

    members = list_group_members(gid)
    if not members:
        raise ValidationError(
            f"群 {gid} 在身份缓存中没有成员。请先 maimai_refresh_identity。"
        )
    if max_members is not None:
        if max_members < 1:
            raise ValidationError("max_members 须为正整数。")
        members = members[:max_members]

    concurrency = (
        max(1, min(10, max_concurrency))
        if max_concurrency is not None
        else _default_concurrency()
    )
    delay_ms = (
        max(0, min(5000, query_delay_ms))
        if query_delay_ms is not None
        else _default_delay_ms()
    )

    qqs = [str(m["userId"]) for m in members]
    song_by_qq, cache_hits, fetched = await _ensure_songs(
        qqs,
        song,
        force_refresh=force_refresh,
        concurrency=concurrency,
        delay_ms=delay_ms,
    )

    field_map = {
        "achievements": "achievements",
        "rating": "rating",
        "dxScore": "dxScore",
    }
    sort_field = field_map[sort_by]

    effective_level = level_index
    if effective_level is None:
        seen_levels: set[int] = set()
        for entry in song_by_qq.values():
            for ch in entry.get("charts") or []:
                if isinstance(ch, dict) and isinstance(ch.get("levelIndex"), int):
                    seen_levels.add(ch["levelIndex"])
        if seen_levels:
            effective_level = max(seen_levels)

    rows: list[dict[str, Any]] = []
    skipped = 0
    member_by_qq = {str(m["userId"]): m for m in members}
    for qq in qqs:
        entry = song_by_qq.get(qq)
        if not entry:
            skipped += 1
            continue
        charts = [c for c in (entry.get("charts") or []) if isinstance(c, dict)]
        if effective_level is not None:
            charts = [c for c in charts if c.get("levelIndex") == effective_level]
        if not charts:
            skipped += 1
            continue
        best = max(
            charts,
            key=lambda c: (
                c.get("rating") if isinstance(c.get("rating"), (int, float)) else -1,
                c.get("achievements")
                if isinstance(c.get("achievements"), (int, float))
                else -1,
            ),
        )
        primary = best.get(sort_field)
        if not isinstance(primary, (int, float)):
            skipped += 1
            continue
        m = member_by_qq.get(qq, {"userId": qq})
        rows.append(
            {
                "qq": qq,
                "displayName": _display_name(m, qq, gid),
                "chart": best,
                "achievements": best.get("achievements"),
                "rating": best.get("rating"),
                "dxScore": best.get("dxScore"),
                "levelIndex": best.get("levelIndex"),
                "levelLabel": best.get("levelLabel"),
                "fc": best.get("fc"),
                "fs": best.get("fs"),
                "rate": best.get("rate"),
                "fetchedAt": entry.get("fetchedAt"),
            }
        )

    reverse = sort_order == "desc"

    def _sort_key(r: dict[str, Any]) -> tuple:
        primary = r.get(sort_field)
        p = primary if isinstance(primary, (int, float)) else float("-inf")
        ach = (
            r.get("achievements")
            if isinstance(r.get("achievements"), (int, float))
            else float("-inf")
        )
        return (p, ach, r["qq"])

    rows.sort(key=_sort_key, reverse=reverse)
    ranked_total = len(rows)
    windowed = _apply_window(
        rows,
        output_limit=output_limit if start_rank is None else None,
        start_rank=start_rank,
        end_rank=end_rank,
    )

    level_names = {0: "Basic", 1: "Advanced", 2: "Expert", 3: "Master", 4: "Re:Master"}
    level_label = (
        level_names.get(effective_level, str(effective_level))
        if effective_level is not None
        else "未限定"
    )
    lines = [
        f"群 {gid} 单曲榜：{song_name}（{song_id}） {level_label}",
        f"排序：{sort_by} {'↓' if reverse else '↑'}",
        (
            f"群成员 {len(members)} 人，上榜 {ranked_total} 人，"
            f"无该曲成绩 {skipped} 人；缓存命中 {cache_hits}，本次拉取 {fetched}"
        ),
        "说明：查本榜时才按成员拉该曲成绩；新鲜缓存复用。",
        "",
    ]
    if not windowed:
        lines.append("当前没有可展示的本曲成绩。")
    else:
        for row in windowed:
            ach = row.get("achievements")
            ra = row.get("rating")
            dx = row.get("dxScore")
            rate = row.get("rate") or ""
            fc = row.get("fc") or ""
            lines.append(
                f"#{row['rank']}  {ach}%  ra={ra}  dx={dx}  {rate} {fc}  "
                f"{row['displayName']}  ({row['qq']})"
            )

    return {
        "groupId": gid,
        "songId": song_id,
        "songName": song_name,
        "levelIndex": effective_level,
        "sortBy": sort_by,
        "sortOrder": sort_order,
        "memberCount": len(members),
        "rankedCount": ranked_total,
        "skippedCount": skipped,
        "cacheHits": cache_hits,
        "fetchedCount": fetched,
        "forceRefresh": force_refresh,
        "rows": windowed,
        "text": "\n".join(lines),
    }


@handle_errors
async def query_group_member_rank(
    *,
    group_id: int | str | None = None,
    qq: int | str | None = None,
    target: str | None = None,
    song: str | None = None,
    level_index: int | None = None,
    context_size: int = 3,
    force_refresh: bool = False,
    max_concurrency: int | None = None,
    query_delay_ms: int | None = None,
    max_members: int | None = None,
) -> dict[str, Any]:
    """Locate one member's rank; builds the board on demand (same as full board)."""
    gid, resolved_qq = _resolve_group_and_qq(
        group_id=group_id, qq=qq, target=target
    )
    if not resolved_qq:
        raise ValidationError("必须提供 qq 或 target。")
    ctx = max(0, min(int(context_size), 10))

    if song:
        board = await query_group_song_rank(
            gid,
            song,
            level_index=level_index,
            sort_by="achievements",
            sort_order="desc",
            output_limit=None,
            force_refresh=force_refresh,
            max_concurrency=max_concurrency,
            query_delay_ms=query_delay_ms,
            max_members=max_members,
        )
        key_field = "achievements"
    else:
        board = await query_group_rating_rank(
            gid,
            sort_order="desc",
            output_limit=None,
            force_refresh=force_refresh,
            max_concurrency=max_concurrency,
            query_delay_ms=query_delay_ms,
            max_members=max_members,
        )
        key_field = "rating"

    rows = board.get("rows") or []
    index = next(
        (i for i, r in enumerate(rows) if str(r.get("qq")) == resolved_qq),
        None,
    )
    member = next(
        (m for m in list_group_members(gid) if m["userId"] == resolved_qq),
        {"userId": resolved_qq, "displayName": resolved_qq},
    )
    display = _display_name(member, resolved_qq, gid)

    if index is None:
        kind = f"单曲 {board.get('songName') or song}" if song else "Rating"
        text = (
            f"{display}（{resolved_qq}）在群 {gid} 的{kind}榜中未找到成绩"
            "（未绑定查分器、隐私关闭或未游玩该曲）。"
        )
        return {
            "groupId": gid,
            "qq": resolved_qq,
            "displayName": display,
            "found": False,
            "songId": board.get("songId"),
            "cacheHits": board.get("cacheHits"),
            "fetchedCount": board.get("fetchedCount"),
            "text": text,
        }

    rank = index + 1
    total = len(rows)
    lo = max(0, index - ctx)
    hi = min(total, index + ctx + 1)
    context = rows[lo:hi]
    self_row = rows[index]
    metric = self_row.get(key_field)
    lines = [
        f"{display}（{resolved_qq}）在群 {gid} "
        + (f"《{board.get('songName')}》" if song else "Rating")
        + f" 排名：第 {rank} / {total}",
        f"成绩：{metric}",
        (
            f"缓存命中 {board.get('cacheHits')}，本次拉取 {board.get('fetchedCount')}"
        ),
        "",
        "附近名次：",
    ]
    for row in context:
        mark = " <<" if str(row.get("qq")) == resolved_qq else ""
        val = row.get(key_field)
        lines.append(
            f"#{row.get('rank')}  {val}  {row.get('displayName')}  ({row.get('qq')}){mark}"
        )

    return {
        "groupId": gid,
        "qq": resolved_qq,
        "displayName": display,
        "found": True,
        "rank": rank,
        "totalRanked": total,
        "target": self_row,
        "context": context,
        "songId": board.get("songId"),
        "songName": board.get("songName"),
        "cacheHits": board.get("cacheHits"),
        "fetchedCount": board.get("fetchedCount"),
        "text": "\n".join(lines),
    }
