"""python -m maimai_mcp.features.search_song --mode 定数 --query 14.0"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_search_result
from .query import query_search


def main() -> int:
    parser = argparse.ArgumentParser(description="查歌")
    add_common_args(parser)
    parser.add_argument(
        "--mode",
        choices=("定数", "bpm", "曲师", "谱师", "标题"),
        default="标题",
    )
    parser.add_argument("--query", required=True, help="查询参数")
    parser.add_argument("--page", type=int, default=1)
    args = parser.parse_args()
    mode = None if args.mode == "标题" else args.mode

    async def _run():
        await ensure_ready()
        songs, page = await query_search(args.query, mode=mode, page=args.page)
        if args.format == "json" and not args.out:
            from ...result import FeatureResult

            return FeatureResult(
                data=[
                    {"song_id": s.song_id, "song_name": s.song_name} for s in songs
                ]
            )
        return await draw_search_result(songs, page, qq=args.qq, out=args.out)

    try:
        return print_result(run_async(_run()), "json" if args.format == "json" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
