"""python -m maimai_mcp.features.rise_score --qq 123 [--level 14+] [--score 10]"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_rise_score
from .query import query_rise_score


def main() -> int:
    parser = argparse.ArgumentParser(description="上分推荐")
    add_common_args(parser)
    parser.add_argument("--level", default=None)
    parser.add_argument("--score", type=int, default=None, help="目标加分")
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        user, sd, sd_low, dx, dx_low = await query_rise_score(
            qq=args.qq, level=args.level, score=args.score
        )
        if args.format == "json" and not args.out:
            from ...result import FeatureResult

            return FeatureResult(
                data={
                    "sd": [r.model_dump() if hasattr(r, "model_dump") else r for r in sd],
                    "dx": [r.model_dump() if hasattr(r, "model_dump") else r for r in dx],
                }
            )
        return draw_rise_score(user, sd, sd_low, dx, dx_low, out=args.out)

    try:
        return print_result(run_async(_run()), "json" if args.format == "json" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
