"""Music data service (trimmed: no guess, no alias push)."""

from ...config import log, lxnsconfig
from ...resources import local_alias_file, merge_alias_file, merge_music_file, plate_file
from ..merge import merge_alias_data, merge_music_data
from ..merge.alias_list import AliasList
from ..merge.models import Alias, SimpleSong, Song
from ..merge.music_list import MusicList
from ..tool import openfile, writefile
from .diving_fish import get_music_list
from .lxns import get_music_aliases, get_music_data
from .yuzuchan import get_music_alias_list, get_plate_data


class MaiMusic:
    total_list: MusicList
    """曲目数据"""
    total_alias_list: AliasList
    """别名数据"""
    total_plate_id_list: dict[str, list[int]]
    """牌子ID列表数据"""
    total_level_data: dict[str, dict[str, list[SimpleSong]]]
    """等级列表数据"""
    total_level_value_map: dict[str, float]
    """定数字典数据"""

    def __init__(self) -> None:
        self._loaded = False

    @property
    def loaded(self) -> bool:
        return self._loaded

    @staticmethod
    def build_level_value_map(music_list: MusicList) -> dict[str, float]:
        """Build chart constant map from a MusicList (plain dict, not defaultdict)."""
        return {
            f"{song.song_id}-{int(d.level_index)}": d.level_value
            for song in music_list.root
            for d in song.difficulties
        }

    def resolve_level_value(
        self, song_id: int, level_index: int
    ) -> float | None:
        """Look up chart constant; return None if the chart is missing from catalog."""
        key = f"{song_id}-{int(level_index)}"
        level_map = getattr(self, "total_level_value_map", None) or {}
        if key in level_map:
            return level_map[key]
        total_list = getattr(self, "total_list", None)
        if total_list is None:
            return None
        found = total_list.by_id(song_id)
        if found is None:
            return None
        li = int(level_index)
        for d in found.difficulties:
            if int(d.level_index) == li:
                return d.level_value
        return None

    def dx_score_of(self, song_id: int, level_index: int) -> int | None:
        """DX score max for a chart, or None if missing."""
        total_list = getattr(self, "total_list", None)
        if total_list is None:
            return None
        found = total_list.by_id(song_id)
        if found is None:
            return None
        li = int(level_index)
        for d in found.difficulties:
            if int(d.level_index) == li:
                return d.dx_score
        try:
            return found.difficulties[li].dx_score
        except (IndexError, TypeError):
            return None

    async def load_from_cache(self) -> bool:
        """Prefer local merge_*.json + plate_data.json when present (fast CLI startup)."""
        if not (
            merge_music_file.exists()
            and merge_alias_file.exists()
            and plate_file.exists()
        ):
            return False
        try:
            music_raw = await openfile(merge_music_file)
            alias_raw = await openfile(merge_alias_file)
            plate_raw = await openfile(plate_file)
            self.total_list = MusicList.model_validate(music_raw)
            self.total_alias_list = AliasList.model_validate(alias_raw)
            self.total_plate_id_list = plate_raw
            self.total_level_data = self.total_list.by_level_list()
            self.total_level_value_map = self.build_level_value_map(self.total_list)
            self._loaded = True
            log.success(
                f"已从本地缓存加载曲库（{len(self.total_list.root)} 首）"
            )
            return True
        except Exception as e:
            log.warning(f"本地缓存加载失败，将重新拉取：{e}")
            return False

    async def get_music(self) -> None:
        """获取所有曲目数据（网络）"""
        df_music_data, df_stats_data = await get_music_list()
        log.success("成功获取「水鱼」查分器曲目数据")
        if lxnsconfig.lxns_dev_token:
            lxns_data = await get_music_data()
            log.success("成功获取「落雪」查分器曲目数据")
        else:
            lxns_data = None
            log.opt(colors=True).warning(
                "<r>未配置落雪开发者Token，跳过获取「落雪」曲目数据源</r>"
            )

        log.info("正在合并曲目数据")
        self.total_list, self.total_level_value_map = await merge_music_data(
            diving_fish_list=df_music_data, lxns_list=lxns_data, stats_map=df_stats_data
        )
        log.success("曲目数据合并完成")
        self.total_level_data = self.total_list.by_level_list()

    async def get_music_alias(self) -> None:
        """获取所有曲目别名（网络）"""
        yuzu_data = await get_music_alias_list()
        log.success("成功获取「柚子」别名数据")
        if lxnsconfig.lxns_dev_token:
            lxns_data = await get_music_aliases()
            log.success("成功获取「落雪」别名数据")
        else:
            lxns_data = None
            log.opt(colors=True).warning(
                "<r>未配置落雪开发者Token，跳过获取「落雪」别名数据源</r>"
            )

        local_alias_data = {}
        if local_alias_file.exists():
            local_alias_data = await openfile(local_alias_file)
        if not local_alias_data:
            local_alias_data = None

        log.info("正在合并别名数据")
        self.total_alias_list = await merge_alias_data(
            yuzu_data, lxns_data, local_alias_data
        )
        log.success("别名数据合并完成")

    async def get_plate_json(self) -> None:
        """获取所有牌子数据"""
        self.total_plate_id_list = await get_plate_data()
        log.success("成功获取牌子数据")

    async def update(self) -> None:
        """强制从网络更新数据"""
        await self.get_music()
        await self.get_music_alias()
        await self.get_plate_json()
        self._loaded = True
        log.success("maimai 数据更新完毕")


mai = MaiMusic()


async def update_local_alias(song_id: int, alias_name: str) -> bool:
    try:
        song_id_key = str(song_id)
        alias = alias_name.lower()

        local_alias_data: dict[str, list[str]] = {}
        if local_alias_file.exists():
            local_alias_data = await openfile(local_alias_file)

        if song_id_key not in local_alias_data:
            local_alias_data[song_id_key] = []
        if alias not in local_alias_data[song_id_key]:
            local_alias_data[song_id_key].append(alias)

        entries = mai.total_alias_list.by_id(song_id)
        if entries:
            if alias not in entries[0].alias:
                entries[0].alias.append(alias)
        else:
            _song = mai.total_list.by_id(song_id)
            mai.total_alias_list.root.append(
                Alias(
                    song_id=song_id,
                    song_name=_song.song_name if _song else "",
                    alias=[alias],
                )
            )

        await writefile(local_alias_file, local_alias_data)
        return True
    except Exception as e:
        log.error(f"添加本地别名失败: {e}")
        return False
