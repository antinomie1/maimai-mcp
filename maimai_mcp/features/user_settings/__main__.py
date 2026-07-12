"""python -m maimai_mcp.features.user_settings --qq 123 --source 水鱼|落雪 --theme prism_plus|circle"""

from __future__ import annotations

import argparse

from .._cli import add_common_args, ensure_ready, run_async
from ...core.database.qq import update_user
from ...core.merge.models import ServiceName, Theme
from ...core.user import resolve_user


def main() -> int:
    parser = argparse.ArgumentParser(description="用户数据源 / 主题")
    add_common_args(parser)
    parser.add_argument("--source", default=None, help="数据源：水鱼/落雪 或 divingfish/lxns")
    parser.add_argument("--theme", default=None, help="主题名或索引")
    parser.add_argument("--show", action="store_true", help="仅显示当前设置")
    args = parser.parse_args()

    async def _run():
        await ensure_ready(load_music=False)
        user = await resolve_user(args.qq)
        if args.show or (not args.source and not args.theme):
            print(f"QQ={user.qqid} service={user.service} theme={user.theme}")
            print("可选数据源:", ServiceName.get_help() if hasattr(ServiceName, "get_help") else list(ServiceName))
            print("可选主题:", Theme.get_help() if hasattr(Theme, "get_help") else list(Theme))
            return
        if args.source:
            source = ServiceName.get_by_index(args.source)
            if source is None:
                print(f"[error] 未找到数据源：{ServiceName.get_help()}")
                return
            user = await update_user(user.qqid, service=source)
            print(f"数据源已切换为：「{source.value}」")
        if args.theme:
            theme = Theme.get_by_index(args.theme)
            if theme is None:
                print(f"[error] 未找到主题：{Theme.get_help()}")
                return
            user = await update_user(user.qqid, theme=theme)
            print(f"主题已切换为：「{theme.value}」")

    try:
        run_async(_run())
        return 0
    except Exception as e:
        print(f"[error] {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
