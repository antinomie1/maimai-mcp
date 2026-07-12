"""python -m maimai_mcp.features.bind_lxns [--qq 123] [--code XXXX-XXXX-XXXX]"""

from __future__ import annotations

import argparse
import re
from textwrap import dedent

from .._cli import add_common_args, ensure_ready, run_async
from ...config import lxnsconfig, maiconfig
from ...core.domain import bind_lxns
from ...core.user import resolve_user

CODE_PATTERN = re.compile(r"^[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$")


def main() -> int:
    parser = argparse.ArgumentParser(description="落雪查分器绑定")
    add_common_args(parser)
    parser.add_argument("--code", default=None, help="授权码 XXXX-XXXX-XXXX")
    args = parser.parse_args()

    if lxnsconfig.lxns_dev_token is None and (
        lxnsconfig.lx_client_id is None or lxnsconfig.redirect_uri is None
    ):
        print("[error] 尚未配置落雪查分器相关信息")
        return 1

    if not args.code:
        url = (
            "https://maimai.lxns.net/oauth/authorize"
            "?response_type=code"
            f"&client_id={lxnsconfig.lx_client_id}"
            f"&redirect_uri={lxnsconfig.redirect_uri}"
            "&scope=read_player+read_user_profile+write_player"
        )
        print(
            dedent(f"""
            请打开以下链接授权，然后将授权码用 --code 提交：
            {url}
            格式：XXXX-XXXX-XXXX
            请在落雪「账号设置 -> 隐私设置」开启允许读取成绩。
            """).strip()
        )
        return 0

    if not CODE_PATTERN.fullmatch(args.code):
        print("[error] 授权码格式错误")
        return 1

    async def _run():
        await ensure_ready(load_music=False)
        user = await resolve_user(args.qq)
        result = await bind_lxns(user, args.code)
        print(result)

    try:
        run_async(_run())
        return 0
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
