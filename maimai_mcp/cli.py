"""Unified CLI: ``python -m maimai_mcp <command> ...``"""

from __future__ import annotations

import argparse
import asyncio
import sys
from typing import Sequence

from .core.errors import as_result
from .features._cli import add_common_args
from .result import FeatureResult


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="maimai",
        description="maimaiDX 去插件化工具集（统一入口）",
    )
    parser.add_argument("--quiet", action="store_true", help="减少启动警告")
    sub = parser.add_subparsers(dest="command", required=True)

    # --- update ---
    p = sub.add_parser("update", help="更新曲库 / 别名 / 定数完成表资源")
    p.add_argument(
        "what",
        nargs="*",
        choices=("music", "alias", "tables", "all"),
        default=["all"],
        help="更新内容（可多选，默认 all）",
    )
    p.add_argument("--rating-only", action="store_true", help="tables 时只更定数表")
    p.add_argument("--plate-only", action="store_true", help="tables 时只更完成表")

    # --- user ---
    p = sub.add_parser("user", help="用户设置 / 落雪绑定 / 水鱼 Import-Token")
    us = p.add_subparsers(dest="user_cmd", required=True)
    us.add_parser("show", help="显示当前用户设置").add_argument("--qq", type=int)
    sp = us.add_parser("source", help="切换数据源")
    sp.add_argument("value", help="0/1 或 水鱼/落雪/divingfish/lxns")
    sp.add_argument("--qq", type=int)
    tp = us.add_parser("theme", help="切换主题")
    tp.add_argument("value", help="0/1 或 prism_plus/circle")
    tp.add_argument("--qq", type=int)
    bp = us.add_parser("bind", help="落雪授权")
    bp.add_argument("--qq", type=int)
    bp.add_argument("--code", default=None, help="授权码；省略则打印授权 URL")
    bip = us.add_parser("bind-import", help="绑定水鱼成绩导入 Import-Token")
    bip.add_argument("--qq", type=int, required=True)
    bip.add_argument("--token", required=True, help="Diving-Fish Import-Token")

    # --- records (official dump → convert → upload) ---
    p = sub.add_parser("records", help="官服成绩导出并上传水鱼")
    rs = p.add_subparsers(dest="records_cmd", required=True)
    ru = rs.add_parser("update", help="扫码 dump → convert → 上传水鱼/落雪")
    ru.add_argument("--qq", type=int, required=True)
    ru.add_argument("--qr-content", required=True, help="二维码解析串（SGWC...）")
    ru.add_argument("--keyship", default=None)
    ru.add_argument("--logoutid", type=int, choices=(1, 2), default=None)
    ru.add_argument("--title-ver", default=None)
    ru.add_argument("--timeout", type=float, default=240.0)
    ru.add_argument("--output-dir", default=None)
    ru.add_argument(
        "--source",
        "--target",
        dest="source",
        required=True,
        choices=("divingfish", "lxns", "both"),
        help="必填数据源：divingfish(水鱼) / lxns(落雪) / both(两者)",
    )
    ru.add_argument(
        "--refresh-music",
        action="store_true",
        help="强制刷新水鱼 music_data 缓存后再 convert（仅 divingfish）",
    )
    rc = rs.add_parser("convert", help="仅将官服 raw JSON 转为水鱼载荷")
    rc.add_argument("input", help="含 GetUserMusicApi 的 raw JSON")
    rc.add_argument("-o", "--output", default=None)
    rc.add_argument("--report", default=None)
    rc.add_argument("--pretty", action="store_true")
    rc.add_argument(
        "--music-data",
        default=None,
        help="水鱼 music_data.json 路径（默认 MAIMAIDX_PATH/data/music_data.json）",
    )

    # Common flags live on the root parser (--quiet / via parse_intermixed).
    # Subcommands use with_quiet=False so root --quiet is never overwritten.
    def _common(p: argparse.ArgumentParser) -> None:
        add_common_args(p, with_quiet=False)

    # --- catalog / song ---
    p = sub.add_parser("chart", help="谱面信息图")
    _common(p)
    p.add_argument("song", help="曲目 ID / 名 / 别名")

    p = sub.add_parser("search", help="查歌")
    _common(p)
    p.add_argument("--mode", choices=("定数", "bpm", "曲师", "谱师", "标题"), default="标题")
    p.add_argument("query", help="查询参数")
    p.add_argument("--page", type=int, default=1)

    p = sub.add_parser("score-line", help="分数线计算")
    p.add_argument("--diff", required=True, help="绿黄红紫白")
    p.add_argument("--id", type=int, required=True, dest="song_id")
    p.add_argument("--line", type=float, required=True)

    p = sub.add_parser("alias", help="别名查询 / 本地添加")
    p.add_argument("--name", default=None)
    p.add_argument("--by-id", action="store_true")
    p.add_argument("--add", action="store_true")
    p.add_argument("--id", type=int, dest="song_id")
    p.add_argument("--alias", default=None)

    p = sub.add_parser("random", help="随机谱面")
    _common(p)
    p.add_argument("--level", required=True)
    p.add_argument("--type", dest="chart_type", choices=("dx", "sd", "标准"), default=None)
    p.add_argument("--color", default=None)

    p = sub.add_parser("mai-what", help="随机 / 上分推荐曲")
    _common(p)
    p.add_argument("--rise", action="store_true")

    p = sub.add_parser("fortune", help="今日运势")
    _common(p)

    # --- player ---
    p = sub.add_parser("b50", help="Best50 / AP50（支持 --qq 或 --username）")
    _common(p)
    p.add_argument("--ap", action="store_true", help="AP50（仅落雪 + --qq）")
    p.add_argument(
        "--source",
        default=None,
        help="本次查询数据源覆盖：水鱼/divingfish 或 落雪/lxns（不改本地默认设定）",
    )

    p = sub.add_parser("minfo", help="单曲成绩")
    _common(p)
    p.add_argument("song", help="曲目 ID / 名 / 别名")

    p = sub.add_parser("rise", help="上分推荐")
    _common(p)
    p.add_argument("--level", default=None)
    p.add_argument("--score", type=int, default=None)

    p = sub.add_parser("ranking", help="水鱼排名")
    _common(p)
    p.add_argument("--name", default="")
    p.add_argument("--page", type=int, default=1)
    p.add_argument("--my", action="store_true")

    p = sub.add_parser("rating-table", help="定数表 / 完成表")
    _common(p)
    p.add_argument("--level", required=True)
    p.add_argument("--progress", action="store_true")
    p.add_argument("--plan", action="store_true")

    p = sub.add_parser("plate", help="牌子完成表 / 进度")
    _common(p)
    p.add_argument("--ver", required=True)
    p.add_argument("--plan", required=True)
    p.add_argument("--mode", choices=("table", "progress"), default="table")
    p.add_argument("--page", type=int, default=1)

    p = sub.add_parser("level-progress", help="等级进度")
    _common(p)
    p.add_argument("--level", required=True)
    p.add_argument("--plan", required=True)
    p.add_argument("--category", default=None)
    p.add_argument("--page", type=int, default=1)

    p = sub.add_parser("score-list", help="分数列表")
    _common(p)
    p.add_argument("--rating", required=True)
    p.add_argument("--page", type=int, default=1)

    return parser


def _is_quiet(args: argparse.Namespace) -> bool:
    return bool(getattr(args, "quiet", False))


async def _dispatch(args: argparse.Namespace) -> FeatureResult:
    quiet = _is_quiet(args)

    from .bootstrap import bootstrap

    cmd = args.command

    # ---- update ----
    if cmd == "update":
        await bootstrap(load_music=False, quiet=quiet)
        from .core.service import mai

        what = args.what or ["all"]
        if "all" in what:
            what = ["music", "alias", "tables"]
        msgs: list[str] = []
        if "music" in what:
            await mai.get_music()
            await mai.get_plate_json()
            msgs.append("曲目/牌子已更新")
        if "alias" in what:
            if not mai.loaded and not await mai.load_from_cache():
                await mai.get_music()
            await mai.get_music_alias()
            msgs.append("别名已更新")
        if "tables" in what:
            await bootstrap(load_music=True, quiet=quiet, warn_tables=True)
            from .core.image.update_table import UpdateTable

            update = UpdateTable()
            if args.rating_only or not args.plate_only:
                await update.update_rating_table()
                await update.update_level_15_rating_table()
                msgs.append("定数表已生成")
            if args.plate_only or not args.rating_only:
                await update.update_plate_table()
                await update.update_wu_plate_table()
                msgs.append("完成表已生成")
        mai._loaded = True
        return FeatureResult.success(text="；".join(msgs) or "无操作")

    # ---- user ----
    if cmd == "user":
        await bootstrap(load_music=False, quiet=quiet)
        from .core.database.qq import update_user
        from .core.domain import bind_lxns
        from .core.merge.models import ServiceName, Theme
        from .core.user import resolve_user
        from .config import lxnsconfig
        from textwrap import dedent

        if args.user_cmd == "show":
            from .tools.user import _user_settings_payload, _user_settings_text

            user = await resolve_user(args.qq)
            return FeatureResult.success(
                text=_user_settings_text(user),
                data=_user_settings_payload(user),
            )
        if args.user_cmd == "source":
            source = ServiceName.get_by_index(args.value)
            if source is None:
                return FeatureResult.failure(
                    f"未找到数据源：\n{ServiceName.get_help()}", code="validation_error"
                )
            user = await resolve_user(args.qq)
            user = await update_user(user.qqid, service=source)
            return FeatureResult.success(text=f"数据源已切换为：「{source.value}」")
        if args.user_cmd == "theme":
            theme = Theme.get_by_index(args.value)
            if theme is None:
                return FeatureResult.failure(
                    f"未找到主题：\n{Theme.get_help()}", code="validation_error"
                )
            user = await resolve_user(args.qq)
            await update_user(user.qqid, theme=theme)
            return FeatureResult.success(text=f"主题已切换为：「{theme.value}」")
        if args.user_cmd == "bind":
            if not args.code:
                url = (
                    "https://maimai.lxns.net/oauth/authorize"
                    "?response_type=code"
                    f"&client_id={lxnsconfig.lx_client_id}"
                    f"&redirect_uri={lxnsconfig.redirect_uri}"
                    "&scope=read_player+read_user_profile+write_player"
                )
                return FeatureResult.success(
                    text=dedent(
                        f"""
                        请打开链接授权，然后执行：
                          maimai user bind --code XXXX-XXXX-XXXX
                        {url}
                        """
                    ).strip()
                )
            user = await resolve_user(args.qq)
            msg = await bind_lxns(user, args.code)
            return FeatureResult.success(text=msg)
        if args.user_cmd == "bind-import":
            from .core.official.workflow import WorkflowError, bind_import_token

            try:
                result = await bind_import_token(args.qq, args.token)
            except WorkflowError as e:
                return FeatureResult.failure(str(e), code="workflow_error")
            return FeatureResult.success(
                text=result["text"],
                data={
                    "qq": result["qq"],
                    "import_token_bound": True,
                    "import_token_preview": result["import_token_preview"],
                },
            )

    # ---- records (official → Diving-Fish) ----
    if cmd == "records":
        await bootstrap(load_music=False, quiet=quiet)
        if args.records_cmd == "convert":
            from pathlib import Path

            from .core.official.convert import convert_file

            try:
                payload, report = convert_file(
                    Path(args.input),
                    output=Path(args.output) if args.output else None,
                    report=Path(args.report) if args.report else None,
                    pretty=args.pretty,
                    music_data=Path(args.music_data) if args.music_data else None,
                )
            except Exception as e:  # noqa: BLE001
                return FeatureResult.failure(str(e), code="convert_error")
            return FeatureResult.success(
                text=f"转换完成：{report.get('converted')} 条，跳过 {report.get('skipped')} 条",
                data=report if args.output else {"converted": len(payload), "report": report},
            )
        if args.records_cmd == "update":
            from pathlib import Path

            from .core.official.workflow import WorkflowError, update_records_workflow

            try:
                result = await update_records_workflow(
                    qq=args.qq,
                    qr_content=args.qr_content,
                    keyship=args.keyship,
                    logoutid=args.logoutid,
                    title_ver=args.title_ver,
                    timeout=args.timeout,
                    output_dir=Path(args.output_dir) if args.output_dir else None,
                    refresh_music=bool(args.refresh_music),
                    source=args.source,
                )
            except WorkflowError as e:
                return FeatureResult.failure(str(e), code="workflow_error")
            return FeatureResult.success(text=result["text"], data=result)

    # ---- features needing catalog ----
    await bootstrap(load_music=True, quiet=quiet)

    if cmd == "chart":
        from .features.chart_info.draw import draw_chart_info
        from .features.chart_info.query import query_chart_info

        song, ctx, _ = await query_chart_info(
            args.song, args.qq, username=getattr(args, "username", None)
        )
        if args.format == "json" and not args.out:
            theme = ctx.get("theme")
            return FeatureResult.success(
                data={
                    "song_id": song.song_id,
                    "song_name": song.song_name,
                    "calc": ctx.get("calc"),
                    "theme": getattr(theme, "value", theme),
                }
            )
        return draw_chart_info(song, ctx, out=args.out)

    if cmd == "search":
        from .features.search_song.draw import draw_search_result
        from .features.search_song.query import query_search

        mode = None if args.mode == "标题" else args.mode
        songs, page = await query_search(args.query, mode=mode, page=args.page)
        if args.format == "json" and not args.out:
            return FeatureResult.success(
                data=[{"song_id": s.song_id, "song_name": s.song_name} for s in songs]
            )
        return await draw_search_result(
            songs,
            page,
            qq=args.qq,
            username=getattr(args, "username", None),
            out=args.out,
        )

    if cmd == "score-line":
        from .features.score_line.query import query_score_line

        text = await query_score_line(args.diff, args.song_id, args.line)
        return FeatureResult.success(text=text)

    if cmd == "alias":
        from .features.alias_query.query import add_local_alias, query_aliases

        if args.add:
            text = await add_local_alias(args.song_id, args.alias)
        else:
            text = await query_aliases(args.name, by_id=args.by_id)
        return FeatureResult.success(text=text)

    if cmd == "random":
        from .features.chart_info.draw import draw_chart_info
        from .features.chart_info.query import query_chart_info
        from .features.random_song.query import query_random_song

        song = await query_random_song(
            level=args.level, chart_type=args.chart_type, color=args.color
        )
        song2, ctx, _ = await query_chart_info(
            str(song.song_id), args.qq, username=getattr(args, "username", None)
        )
        return draw_chart_info(song2, ctx, out=args.out)

    if cmd == "mai-what":
        from .features.chart_info.draw import draw_chart_info
        from .features.chart_info.query import query_chart_info
        from .features.mai_what.query import query_mai_what

        uname = getattr(args, "username", None)
        song = await query_mai_what(qq=args.qq, username=uname, rise=args.rise)
        song2, ctx, _ = await query_chart_info(
            str(song.song_id), args.qq, username=uname
        )
        return draw_chart_info(song2, ctx, out=args.out)

    if cmd == "fortune":
        from .features.fortune.draw import draw_fortune_jacket
        from .features.fortune.query import query_fortune

        text, song = await query_fortune(
            args.qq, username=getattr(args, "username", None)
        )
        if args.format == "json":
            return FeatureResult.success(
                text=text, data={"song_id": song.song_id, "song_name": song.song_name}
            )
        return draw_fortune_jacket(song, text, out=args.out)

    if cmd == "b50":
        from .features.best50.draw import draw_best50
        from .features.best50.query import query_best50

        user, player, best50, by_name = await query_best50(
            args.qq,
            username=getattr(args, "username", None),
            all_perfect=args.ap,
            source=getattr(args, "source", None),
        )
        if args.format == "json" and not args.out:
            return FeatureResult.success(
                data={
                    "qq": user.qqid,
                    "username": getattr(args, "username", None),
                    "service": user.service.value,
                    "player": player,
                    "best50": best50,
                }
            )
        return await draw_best50(
            user, player, best50, is_username=by_name, out=args.out
        )

    if cmd == "minfo":
        from .features.play_score.draw import draw_play_score
        from .features.play_score.query import query_play_score

        user, song, play_result = await query_play_score(
            args.song, args.qq, username=getattr(args, "username", None)
        )
        if args.format == "json" and not args.out:
            return FeatureResult.success(
                data={"song_id": song.song_id, "play_result": play_result}
            )
        return draw_play_score(user, song, play_result, out=args.out)

    if cmd == "rise":
        from .features.rise_score.draw import draw_rise_score
        from .features.rise_score.query import query_rise_score

        user, sd, sd_low, dx, dx_low = await query_rise_score(
            qq=args.qq,
            username=getattr(args, "username", None),
            level=args.level,
            score=args.score,
        )
        if args.format == "json" and not args.out:
            return FeatureResult.success(data={"sd": sd, "dx": dx})
        return draw_rise_score(user, sd, sd_low, dx, dx_low, out=args.out)

    if cmd == "ranking":
        from .features.ranking.draw import draw_ranking_list
        from .features.ranking.query import query_ranking

        rank_name = args.name
        if args.my and getattr(args, "username", None):
            rank_name = args.username
        data = await query_ranking(
            name=rank_name,
            page=args.page,
            my_qq=args.qq if args.my and not rank_name else None,
        )
        if data["mode"] != "list" or args.format in ("json", "text"):
            return FeatureResult.success(text=data.get("text"), data=data)
        return draw_ranking_list(data["text"], out=args.out)

    if cmd == "rating-table":
        from .features.rating_table.draw import (
            draw_rating_table_progress,
            draw_rating_table_text,
        )
        from .features.rating_table.query import query_rating_table

        rating, user, play_result, with_p = await query_rating_table(
            args.level,
            qq=args.qq,
            username=getattr(args, "username", None),
            with_progress=args.progress or args.plan,
        )
        if with_p and user and play_result is not None:
            return draw_rating_table_progress(
                rating, user.service, play_result, plan=args.plan, out=args.out
            )
        return draw_rating_table_text(rating, out=args.out)

    if cmd == "plate":
        from .features.plate_table.draw import draw_plate_progress, draw_plate_table
        from .features.plate_table.query import query_plate

        user, play_result, ver, version_name, plan = await query_plate(
            args.ver,
            args.plan,
            args.qq,
            username=getattr(args, "username", None),
        )
        kwargs = dict(
            service=user.service,
            play_result=play_result,
            plan=plan,
            version=ver,
            version_name=version_name,
            page=args.page,
            out=args.out,
        )
        if args.mode == "progress":
            return draw_plate_progress(**kwargs)
        return draw_plate_table(**kwargs)

    if cmd == "level-progress":
        from .features.level_progress.draw import draw_level_progress
        from .features.level_progress.query import query_level_progress

        user, level, plan, cat, page, c, u, n = await query_level_progress(
            args.level,
            args.plan,
            qq=args.qq,
            username=getattr(args, "username", None),
            category=args.category,
            page=args.page,
        )
        return draw_level_progress(
            user, level, plan, cat, page, c, u, n, out=args.out
        )

    if cmd == "score-list":
        from .features.level_score_list.draw import draw_level_score_list
        from .features.level_score_list.query import query_level_score_list

        user, rating, page, results = await query_level_score_list(
            args.rating,
            qq=args.qq,
            username=getattr(args, "username", None),
            page=args.page,
        )
        if args.format == "json" and not args.out:
            return FeatureResult.success(data=results)
        return draw_level_score_list(user, rating, page, results, out=args.out)

    return FeatureResult.failure(f"未知命令：{cmd}", code="unknown_command")


def main(argv: Sequence[str] | None = None) -> int:
    parser = _build_parser()
    argv_list = list(sys.argv[1:] if argv is None else argv)
    # Allow ``--quiet`` / ``-q`` anywhere (subparsers reject parent-only flags after cmd)
    quiet = False
    cleaned: list[str] = []
    for a in argv_list:
        if a in ("--quiet", "-q"):
            quiet = True
        else:
            cleaned.append(a)
    args = parser.parse_args(cleaned)
    args.quiet = quiet or bool(getattr(args, "quiet", False))

    async def _run() -> FeatureResult:
        return await as_result(_dispatch(args))

    result = asyncio.run(_run())
    fmt = getattr(args, "format", "text") or "text"
    if fmt == "image":
        fmt = "text"
    return result.print(fmt)


if __name__ == "__main__":
    sys.exit(main())
