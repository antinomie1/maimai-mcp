"""Diving-Fish rating ranking query."""

from __future__ import annotations

import time

from ...core.clients.divingfish.client import DivingFishAPI
from ...core.errors import handle_errors
from ...core.user import resolve_user


@handle_errors
async def query_ranking(
    name: str = "",
    page: int = 1,
    *,
    my_qq: int | None = None,
) -> dict:
    api = DivingFishAPI()
    rank_data = await api.rating_ranking()
    current_time = time.strftime("%Y-%m-%d %H:%M:%S")

    if my_qq is not None:
        user = await resolve_user(my_qq, auto_create=False)
        info = await DivingFishAPI(qqid=user.qqid).query_user_b50()
        for num, rank in enumerate(rank_data):
            if rank.username == info.username:
                return {
                    "mode": "self",
                    "text": f"您的Rating为「{rank.ra}」，排名第「{num + 1}」名",
                    "rank": num + 1,
                    "ra": rank.ra,
                }
        return {"mode": "self", "text": "未在查分器排行榜中找到您的记录。"}

    if name:
        found = next(
            (
                (idx + 1, r.username, r.ra)
                for idx, r in enumerate(rank_data)
                if r.username.lower() == name.lower()
            ),
            None,
        )
        if found:
            rank_index, nickname, ra = found
            text = (
                f"截止至「{current_time}」玩家「{nickname}」\n"
                f"在查分器已注册用户 RA 排行第「{rank_index}」位"
            )
            return {"mode": "name", "text": text, "rank": rank_index, "ra": ra}
        return {
            "mode": "name",
            "text": f"未在查分器排行榜前「{len(rank_data)}」名中找到玩家「{name}」",
        }

    per_page = 50
    total_pages = max(1, (len(rank_data) + per_page - 1) // per_page)
    page = max(1, min(page, total_pages))
    start = (page - 1) * per_page
    end = min(start + per_page, len(rank_data))
    header = f"截止至「{current_time}」，查分器已注册用户 RA 排行：\n"
    lines = [
        f"No.{i:02d}.「{r.ra}」 {r.username}"
        for i, r in enumerate(rank_data[start:end], start=start + 1)
    ]
    footer = f"\n第「{page} / {total_pages}」页，共「{len(rank_data)}」名玩家"
    return {
        "mode": "list",
        "text": header + "\n".join(lines) + footer,
        "page": page,
        "total_pages": total_pages,
    }
