"""Runtime bootstrap: DB, music data, optional assets preload."""

from __future__ import annotations

from .config import dfconfig, log, lxnsconfig, maiconfig
from .core.clients.divingfish.client import DivingFishAPI
from .core.database.qq import create_database
from .core.image.assets import AssetsImage
from .core.service import mai
from .resources import plate_table_dir, rating_table_dir

_bootstrapped = False
_warned_config = False


async def bootstrap(
    *,
    load_music: bool = True,
    preload_assets: bool | None = None,
    quiet: bool = False,
    warn_tables: bool = False,
) -> None:
    """Initialize database and (optionally) music catalog.

    Params:
        load_music: load song/alias/plate catalog
        preload_assets: override SAVE_IN_MEMORY for theme sprites
        quiet: suppress repeated config/table warnings
        warn_tables: force empty rating/plate table warnings (for update_tables)
    """
    global _bootstrapped, _warned_config
    if _bootstrapped and (not load_music or mai.loaded):
        # Prior boot may have skipped asset preload; finish it if needed.
        do_preload = (
            maiconfig.save_in_memory if preload_assets is None else preload_assets
        )
        if do_preload and not AssetsImage._images_loaded:
            AssetsImage._load_image()
        return

    await create_database()

    if dfconfig.divingfish_prober_proxy:
        log.info("使用代理服务器访问「水鱼」查分器")
        DivingFishAPI.set_proxy()
    if maiconfig.maimaidx_alias_proxy:
        log.info("使用代理服务器访问「柚子」别名服务器")

    if load_music and not mai.loaded:
        if not await mai.load_from_cache():
            log.info("正在获取 maimai 曲目数据")
            await mai.get_music()
            log.info("正在获取 maimai 曲目别名数据")
            await mai.get_music_alias()
            log.info("正在获取 maimai 牌子数据")
            await mai.get_plate_json()
            mai._loaded = True
            log.success("maimai 数据获取完成")

    if not quiet and not _warned_config:
        if dfconfig.divingfish_token is None:
            log.opt(colors=True).warning(
                "<r>未配置水鱼查分器开发者Token，查分模块可能仅能使用 b50</r>"
            )
        if lxnsconfig.lxns_dev_token is None:
            log.opt(colors=True).warning(
                "<r>未配置落雪查分器开发者Token，无法使用落雪数据源</r>"
            )
        _warned_config = True

    do_preload = (
        maiconfig.save_in_memory if preload_assets is None else preload_assets
    )
    if do_preload:
        AssetsImage._load_image()
        if not quiet:
            log.success("已将图片保存在内存中")

    # Empty table dirs: only when asked (update tables) or non-quiet first boot
    if warn_tables or (not quiet and not _bootstrapped):
        if not list(rating_table_dir.iterdir()):
            log.opt(colors=True).warning(
                "<y>定数表文件夹为空</y>，请运行 maimai update tables 生成。"
            )
        if not list(plate_table_dir.iterdir()):
            log.opt(colors=True).warning(
                "<y>牌子完成表文件夹为空</y>，请运行 maimai update tables 生成。"
            )

    _bootstrapped = True
