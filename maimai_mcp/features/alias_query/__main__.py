"""python -m maimai_mcp.features.alias_query --name xxx
python -m maimai_mcp.features.alias_query --add --id 11451 --alias 别名
"""

from __future__ import annotations

import argparse

from .._cli import ensure_ready, print_result, run_async
from ...result import FeatureResult
from .query import add_local_alias, query_aliases


def main() -> int:
    parser = argparse.ArgumentParser(description="别名查询 / 本地别名")
    parser.add_argument("--name", default=None, help="曲名/别名/ID")
    parser.add_argument("--by-id", action="store_true")
    parser.add_argument("--add", action="store_true", help="添加本地别名")
    parser.add_argument("--id", type=int, dest="song_id")
    parser.add_argument("--alias", default=None)
    args = parser.parse_args()

    async def _run():
        await ensure_ready()
        if args.add:
            if not args.song_id or not args.alias:
                raise RuntimeError("--add 需要 --id 与 --alias")
            text = await add_local_alias(args.song_id, args.alias)
        else:
            if not args.name:
                raise RuntimeError("需要 --name")
            text = await query_aliases(args.name, by_id=args.by_id)
        return FeatureResult(text=text)

    try:
        return print_result(run_async(_run()), "text")
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
