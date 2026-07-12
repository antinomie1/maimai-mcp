"""python -m maimai_mcp.features.level_progress --level 14 --plan ap --qq 123"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_level_progress
from .query import query_level_progress


def main() -> int:
    parser = argparse.ArgumentParser(description="等级进度")
    add_common_args(parser)
    parser.add_argument("--level", required=True)
    parser.add_argument("--plan", required=True, help="如 sss+ / ap / fdx+")
    parser.add_argument("--category", default=None, help="已完成/未完成/未游玩")
    parser.add_argument("--page", type=int, default=1)
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        user, level, plan, cat, page, c, u, n = await query_level_progress(
            args.level, args.plan, qq=args.qq, category=args.category, page=args.page
        )
        return draw_level_progress(user, level, plan, cat, page, c, u, n, out=args.out)

    try:
        return print_result(run_async(_run()), "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
