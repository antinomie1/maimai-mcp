"""python -m maimai_mcp.features.rating_table --level 13 [--progress] [--plan] --qq 123"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_rating_table_progress, draw_rating_table_text
from .query import query_rating_table


def main() -> int:
    parser = argparse.ArgumentParser(description="定数表 / 完成表")
    add_common_args(parser)
    parser.add_argument("--level", required=True, help="如 13 / 13+")
    parser.add_argument("--progress", action="store_true", help="带个人完成情况")
    parser.add_argument("--plan", action="store_true", help="fc/ap 计划完成表")
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        rating, user, play_result, with_p = await query_rating_table(
            args.level, qq=args.qq, with_progress=args.progress or args.plan
        )
        if with_p and user and play_result is not None:
            return draw_rating_table_progress(
                rating, user.service, play_result, plan=args.plan, out=args.out
            )
        return draw_rating_table_text(rating, out=args.out)

    try:
        return print_result(run_async(_run()), "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
