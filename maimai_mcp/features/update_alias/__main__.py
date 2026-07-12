"""python -m maimai_mcp.features.update_alias"""

from __future__ import annotations

from .._cli import ensure_ready, run_async
from ...core.service import mai


def main() -> int:
    async def _run():
        await ensure_ready(load_music=False)
        # alias merge may need song titles; ensure music is available
        if not mai.loaded and not await mai.load_from_cache():
            await mai.get_music()
        await mai.get_music_alias()
        mai._loaded = True
        print("别名库更新完成（已写入本地缓存）")

    try:
        run_async(_run())
        return 0
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
