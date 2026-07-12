"""python -m maimai_mcp.features.score_line --diff 紫 --id 799 --line 100"""

from __future__ import annotations

import argparse

from .._cli import ensure_ready, print_result, run_async
from ...result import FeatureResult
from .query import HELP, query_score_line


def main() -> int:
    parser = argparse.ArgumentParser(description="分数线计算")
    parser.add_argument("--diff", help="绿黄红紫白")
    parser.add_argument("--id", type=int, dest="song_id")
    parser.add_argument("--line", type=float)
    parser.add_argument("--help-score", action="store_true")
    args = parser.parse_args()

    if args.help_score or not (args.diff and args.song_id and args.line is not None):
        if args.help_score:
            print(HELP)
            return 0
        parser.error("需要 --diff --id --line，或使用 --help-score")

    async def _run():
        await ensure_ready()
        text = await query_score_line(args.diff, args.song_id, args.line)
        return FeatureResult(text=text)

    try:
        return print_result(run_async(_run()), "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
