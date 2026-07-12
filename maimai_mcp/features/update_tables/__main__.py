"""python -m maimai_mcp.features.update_tables [--rating] [--plate]"""

from __future__ import annotations

import argparse

from .._cli import ensure_ready, run_async
from ...core.image.update_table import UpdateTable


def main() -> int:
    parser = argparse.ArgumentParser(description="更新定数表/完成表图片资源")
    parser.add_argument("--rating", action="store_true", help="只更新定数表")
    parser.add_argument("--plate", action="store_true", help="只更新牌子完成表")
    args = parser.parse_args()
    do_all = not args.rating and not args.plate

    async def _run():
        await ensure_ready()
        update = UpdateTable()
        if do_all or args.rating:
            print("正在更新定数表...")
            await update.update_rating_table()
            await update.update_level_15_rating_table()
            print("定数表更新完成")
        if do_all or args.plate:
            print("正在更新完成表...")
            await update.update_plate_table()
            await update.update_wu_plate_table()
            print("完成表更新完成")

    try:
        run_async(_run())
        return 0
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
