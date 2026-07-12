"""python -m maimai_mcp.features.fortune --qq 123"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_fortune_jacket
from .query import query_fortune


def main() -> int:
    parser = argparse.ArgumentParser(description="今日运势")
    add_common_args(parser)
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        text, song = await query_fortune(args.qq)
        if args.format == "json":
            from ...result import FeatureResult

            return FeatureResult(
                text=text, data={"song_id": song.song_id, "song_name": song.song_name}
            )
        return draw_fortune_jacket(song, text, out=args.out)

    try:
        return print_result(run_async(_run()), "json" if args.format == "json" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
