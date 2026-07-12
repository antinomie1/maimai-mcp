"""python -m maimai_mcp.features.update_music"""

from __future__ import annotations

from .._cli import ensure_ready, run_async
from ...core.service import mai


def main() -> int:
    async def _run():
        await ensure_ready(load_music=False)
        await mai.get_music()
        await mai.get_plate_json()
        mai._loaded = True
        print("maimai 曲目/牌子数据更新完成（已写入本地缓存）")

    try:
        run_async(_run())
        return 0
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
