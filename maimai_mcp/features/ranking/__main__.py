"""python -m maimai_mcp.features.ranking [--name xxx] [--page 1] [--my --qq 123]"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from ...result import FeatureResult
from .draw import draw_ranking_list
from .query import query_ranking


def main() -> int:
    parser = argparse.ArgumentParser(description="查分器排名")
    add_common_args(parser)
    parser.add_argument("--name", default="", help="按用户名查排名")
    parser.add_argument("--page", type=int, default=1)
    parser.add_argument("--my", action="store_true", help="查询自己的排名")
    args = parser.parse_args()

    async def _run():
        await ensure_ready(load_music=False)
        data = await query_ranking(
            name=args.name,
            page=args.page,
            my_qq=args.qq if args.my else None,
        )
        if data["mode"] != "list" or args.format == "json":
            return FeatureResult(text=data.get("text"), data=data)
        if args.format == "text":
            return FeatureResult(text=data["text"], data=data)
        return draw_ranking_list(data["text"], out=args.out)

    try:
        return print_result(run_async(_run()), "json" if args.format == "json" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
