#!/usr/bin/env python3
"""Local smoke test without Inspector UI: list tools + call a few impls.

Usage (repo root):
  set PYTHONPATH=.
  python scripts/smoke_mcp_tools.py
  python scripts/smoke_mcp_tools.py --username <diving-fish-username>
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


async def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke-test maimai_mcp tools")
    parser.add_argument("--username", default=None, help="Diving-Fish username for b50")
    parser.add_argument("--qq", type=int, default=None)
    args = parser.parse_args()

    from maimai_mcp.server import mcp
    from maimai_mcp.schemas import B50Input, ChartInput, SearchInput
    from maimai_mcp.tools.catalog import chart_impl, search_impl
    from maimai_mcp.tools.player import b50_impl
    from maimai_mcp.tools.workflow import lookup_song_impl
    from maimai_mcp.schemas import LookupSongInput
    from maimai_mcp.context import session

    names = sorted(mcp._tool_manager._tools.keys())
    print(f"[tools] {len(names)} registered")
    for n in names:
        print(f"  - {n}")

    if args.username:
        session.set_identity(username=args.username, qq=args.qq)

    r = await search_impl(SearchInput(query="14.0", mode="定数", format="json"))
    print(f"[search 定数 14.0] ok={r.ok} count={len(r.data or [])}")

    r = await chart_impl(ChartInput(song="834", format="json"))
    print(f"[chart 834] ok={r.ok} data={r.data}")

    r = await lookup_song_impl(LookupSongInput(query="834", format="json"))
    print(f"[lookup 834] ok={r.ok}")

    if args.username or args.qq:
        r = await b50_impl(
            B50Input(username=args.username, qq=args.qq, format="json")
        )
        rating = None
        if r.ok and r.data and r.data.get("player") is not None:
            p = r.data["player"]
            rating = getattr(p, "rating", None) or (
                p.get("rating") if isinstance(p, dict) else None
            )
        print(f"[b50] ok={r.ok} rating={rating} err={r.error}")
    else:
        print("[b50] skip (pass --username or --qq)")

    print(json.dumps({"ok": True, "tool_count": len(names)}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
