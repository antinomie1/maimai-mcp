"""python -m maimai_mcp.features.play_score --song 11451 --qq 123"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_play_score
from .query import query_play_score


def main() -> int:
    parser = argparse.ArgumentParser(description="单曲成绩 minfo")
    add_common_args(parser)
    parser.add_argument("--song", required=True, help="曲目 ID / 曲名 / 别名")
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        user, song, play_result = await query_play_score(args.song, args.qq)
        if args.format == "json" and not args.out:
            from ...result import FeatureResult

            return FeatureResult(
                data={
                    "song_id": song.song_id,
                    "song_name": song.song_name,
                    "play_result": [
                        r.model_dump() if hasattr(r, "model_dump") else r
                        for r in play_result
                    ],
                }
            )
        return draw_play_score(user, song, play_result, out=args.out)

    try:
        return print_result(run_async(_run()), "json" if args.format == "json" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
