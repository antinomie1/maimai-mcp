"""python -m maimai_mcp.features.best50 --qq 123 | --username name [--ap] [--out path]"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_best50
from .query import query_best50


def main() -> int:
    parser = argparse.ArgumentParser(description="Best50 / AP50")
    add_common_args(parser)
    parser.add_argument("--ap", action="store_true", help="AP50（仅落雪）")
    args = parser.parse_args()

    async def _run():
        await ensure_ready(load_music=False)
        user, player, best50, by_name = await query_best50(
            args.qq, username=args.username, all_perfect=args.ap
        )
        if args.format == "json" and not args.out:
            from ...result import FeatureResult

            return FeatureResult.success(
                data={
                    "qq": user.qqid,
                    "username": args.username,
                    "player": player,
                    "best50": best50,
                }
            )
        return await draw_best50(
            user, player, best50, is_username=by_name, out=args.out
        )

    try:
        result = run_async(_run())
        return print_result(result, args.format if args.format != "image" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
