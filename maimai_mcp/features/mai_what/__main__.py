"""python -m maimai_mcp.features.mai_what [--rise] [--qq 123]"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from ..chart_info.draw import draw_chart_info
from ..chart_info.query import query_chart_info
from .query import query_mai_what


def main() -> int:
    parser = argparse.ArgumentParser(description="mai 什么 / 上分推荐曲")
    add_common_args(parser)
    parser.add_argument("--rise", action="store_true", help="偏向上分谱面")
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        song = await query_mai_what(qq=args.qq, rise=args.rise)
        song2, ctx, _ = await query_chart_info(str(song.song_id), args.qq)
        return draw_chart_info(song2, ctx, out=args.out)

    try:
        return print_result(run_async(_run()), "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
