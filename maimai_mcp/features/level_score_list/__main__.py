"""python -m maimai_mcp.features.level_score_list --rating 14.0 --qq 123"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_level_score_list
from .query import query_level_score_list


def main() -> int:
    parser = argparse.ArgumentParser(description="分数列表")
    add_common_args(parser)
    parser.add_argument("--rating", required=True, help="等级如 14 或定数 14.0")
    parser.add_argument("--page", type=int, default=1)
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        user, rating, page, results = await query_level_score_list(
            args.rating, qq=args.qq, page=args.page
        )
        if args.format == "json" and not args.out:
            from ...result import FeatureResult

            return FeatureResult(
                data=[
                    r.model_dump() if hasattr(r, "model_dump") else r for r in results
                ]
            )
        return draw_level_score_list(user, rating, page, results, out=args.out)

    try:
        return print_result(run_async(_run()), "json" if args.format == "json" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
