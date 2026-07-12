"""python -m maimai_mcp.features.chart_info --id 834 [--qq 123]"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, print_result, run_async
from .draw import draw_chart_info
from .query import query_chart_info


def main() -> int:
    parser = argparse.ArgumentParser(description="谱面信息 id / 查曲")
    add_common_args(parser)
    parser.add_argument("--id", dest="song_id", default=None, help="曲目 ID")
    parser.add_argument("--song", default=None, help="曲名或别名")
    args = parser.parse_args()
    key = args.song_id or args.song
    if not key:
        parser.error("需要 --id 或 --song")

    async def _run():
        await ensure_ready()
        song, ctx, _user = await query_chart_info(key, args.qq)
        if args.format == "json" and not args.out:
            from ...result import FeatureResult

            return FeatureResult(
                data={
                    "song_id": song.song_id,
                    "song_name": song.song_name,
                    "calc": ctx.get("calc"),
                    "banquet": ctx.get("banquet"),
                }
            )
        return draw_chart_info(song, ctx, out=args.out)

    try:
        return print_result(run_async(_run()), "json" if args.format == "json" else "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
