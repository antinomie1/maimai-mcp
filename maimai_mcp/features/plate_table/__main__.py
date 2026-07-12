"""python -m maimai_mcp.features.plate_table --ver 祝 --plan 将 --mode table|progress --qq 123"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_plate_progress, draw_plate_table
from .query import query_plate


def main() -> int:
    parser = argparse.ArgumentParser(description="牌子完成表 / 进度")
    add_common_args(parser)
    parser.add_argument("--ver", required=True, help="版本字：真超...祝双宴镜彩")
    parser.add_argument("--plan", required=True, help="极/将/神/舞舞/者 等")
    parser.add_argument(
        "--mode", choices=("table", "progress"), default="table"
    )
    parser.add_argument("--page", type=int, default=1)
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        user, play_result, ver, version_name, plan = await query_plate(
            args.ver, args.plan, args.qq
        )
        kwargs = dict(
            service=user.service,
            play_result=play_result,
            plan=plan,
            version=ver,
            version_name=version_name,
            page=args.page,
            out=args.out,
        )
        if args.mode == "progress":
            return draw_plate_progress(**kwargs)
        return draw_plate_table(**kwargs)

    try:
        return print_result(run_async(_run()), "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
