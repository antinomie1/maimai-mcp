"""python -m maimai_mcp.features.random_song --level 13+ [--type dx] [--color 紫] [--qq 123]"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from ..chart_info.draw import draw_chart_info
from ..chart_info.query import query_chart_info
from .query import query_random_song


def main() -> int:
    parser = argparse.ArgumentParser(description="随机谱面")
    add_common_args(parser)
    parser.add_argument("--level", required=True)
    parser.add_argument("--type", dest="chart_type", default=None, choices=("dx", "sd", "标准"))
    parser.add_argument("--color", default=None, help="绿黄红紫白")
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        song = await query_random_song(
            level=args.level, chart_type=args.chart_type, color=args.color
        )
        song2, ctx, _ = await query_chart_info(str(song.song_id), args.qq)
        return draw_chart_info(song2, ctx, out=args.out)

    try:
        return print_result(run_async(_run()), "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
