"""Shared helpers for feature entrypoints and the unified CLI."""

from __future__ import annotations

import argparse
import asyncio
from typing import Any, Awaitable

from ..config import maiconfig
from ..core.errors import as_result, wrap_exception
from ..result import FeatureResult


def add_common_args(parser: argparse.ArgumentParser, *, with_quiet: bool = True) -> None:
    parser.add_argument("--qq", type=int, default=None, help="QQ 号（默认 DEFAULT_QQ）")
    parser.add_argument(
        "--username",
        "-u",
        default=None,
        help="水鱼查分器用户名（可与 --qq 二选一；默认 DEFAULT_USERNAME）",
    )
    parser.add_argument(
        "--format",
        choices=("text", "json", "image"),
        default="image",
        help="输出格式：text / json / image",
    )
    parser.add_argument("--out", type=str, default=None, help="图片输出路径或目录")
    if with_quiet:
        parser.add_argument(
            "--quiet",
            "-q",
            action="store_true",
            help="减少 bootstrap 警告输出",
        )


def print_result(result: FeatureResult, fmt: str) -> int:
    """Backward-compatible alias for FeatureResult.print."""
    # json mode always dumps full contract; image/text use human view
    out_fmt = "json" if fmt == "json" else "text"
    return result.print(out_fmt)


def run_async(coro: Awaitable[Any]) -> Any:
    return asyncio.run(coro)


async def ensure_ready(*, load_music: bool = True, quiet: bool = True) -> None:
    """Feature ``__main__`` defaults to quiet to avoid spam; CLI can pass False."""
    from ..bootstrap import bootstrap

    await bootstrap(load_music=load_music, quiet=quiet)


def default_qq(args: argparse.Namespace) -> int | None:
    return args.qq if getattr(args, "qq", None) is not None else maiconfig.default_qq


async def run_feature(coro: Awaitable[Any]) -> FeatureResult:
    """Await feature work and convert exceptions to FeatureResult."""
    return await as_result(coro)


def exit_with(result: FeatureResult, fmt: str = "text") -> int:
    return print_result(result, fmt)
