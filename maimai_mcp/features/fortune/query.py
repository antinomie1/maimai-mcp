"""Daily fortune text + recommended song."""

from __future__ import annotations

import random
import zlib

from ...config import maiconfig
from ...constants import FORTUNE
from ...core.errors import handle_errors
from ...core.merge.models import Song
from ...core.service import mai
from ...core.tool import qqhash
from ...core.user import resolve_player


def _fortune_seed(qq: int | None, username: str | None) -> int:
    if username:
        return zlib.adler32(username.lower().encode("utf-8")) & 0x7FFFFFFF
    if qq is not None:
        return qqhash(qq)
    return qqhash(0)


@handle_errors
async def query_fortune(
    qq: int | None = None, *, username: str | None = None
) -> tuple[str, Song]:
    # Prefer explicit identity; optional so username-only works without DEFAULT_QQ
    ref = await resolve_player(qq, username, optional=True)
    seed_qq = ref.user.qqid if ref and ref.user.qqid else None
    seed_user = ref.username if ref else username
    fortune_hash = _fortune_seed(seed_qq if not seed_user else None, seed_user)
    if seed_user is None and ref and ref.user.qqid:
        fortune_hash = qqhash(ref.user.qqid)

    daily_random = random.Random(fortune_hash)
    rp = fortune_hash % 100
    h = fortune_hash
    wm_value = []
    for _ in range(11):
        wm_value.append(h & 3)
        h >>= 2
    who = seed_user or (str(ref.user.qqid) if ref else "?")
    msg = f"[{who}] 今日人品值：{rp}\n"
    for i in range(11):
        if wm_value[i] == 3:
            msg += f"宜 {FORTUNE[i]}\n"
        elif wm_value[i] == 0:
            msg += f"忌 {FORTUNE[i]}\n"
    song = daily_random.choice(mai.total_list.root)
    ds = "/".join([str(d.level_value) for d in song.difficulties])
    msg += (
        f"{maiconfig.bot_name} 提醒您：打机时不要大力拍打或滑动哦\n"
        f"今日推荐歌曲：ID.{song.song_id} - {song.song_name}\n"
        f"定数：{ds}"
    )
    return msg, song
