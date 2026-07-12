"""python -m maimai_mcp.features.global_chart --song 紫11451  or --song 11451 --diff 3"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_global_chart
from .query import query_global_chart


def main() -> int:
    parser = argparse.ArgumentParser(description="全服谱面统计 ginfo")
    add_common_args(parser)
    parser.add_argument("--song", required=True, help="可选前缀绿黄红紫白 + 曲目")
    parser.add_argument(
        "--diff",
        type=int,
        default=None,
        help="难度索引 0-4（若不写且 song 无颜色前缀，默认 Master=3）",
    )
    args = parser.parse_args()

    song_key = args.song.strip()
    level_index = args.diff
    if level_index is None:
        if song_key and song_key[0] in "绿黄红紫白":
            level_index = "绿黄红紫白".index(song_key[0])
            song_key = song_key[1:].strip()
        else:
            level_index = 3

    async def _run():
        await ensure_ready()
        song, li, text = await query_global_chart(song_key, level_index)
        if args.format == "json" and not args.out:
            from ...result import FeatureResult

            return FeatureResult(text=text, data={"song_id": song.song_id, "level_index": li})
        return await draw_global_chart(song, li, text, out=args.out)

    try:
        return print_result(run_async(_run()), "json" if args.format == "json" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
