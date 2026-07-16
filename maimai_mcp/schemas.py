"""Shared Pydantic inputs for maimai MCP tools."""

from __future__ import annotations

from typing import Literal

from pydantic import AliasChoices, BaseModel, ConfigDict, Field


class StrictModel(BaseModel):
    model_config = ConfigDict(
        str_strip_whitespace=True,
        validate_assignment=True,
        extra="forbid",
    )


class PlayerArgs(StrictModel):
    """Player identity: QQ and/or Diving-Fish username."""

    qq: int | None = Field(
        default=None,
        description=(
            "Player QQ (sender or target). NEVER put group id here. "
            "Pass every call; chat context is NOT auto-injected."
        ),
        ge=1,
    )
    username: str | None = Field(
        default=None,
        description="Diving-Fish username if no QQ. Prefer qq for Lxns.",
        max_length=64,
    )
    source: str | None = Field(
        default=None,
        description=(
            "One-shot data source for this query only (does NOT write user.db): "
            "divingfish / 水鱼 / df, or lxns / 落雪. "
            "When the user names a prober for this query, MUST set this. "
            "Omit to use the QQ saved default (maimai_user_show.service)."
        ),
        validation_alias=AliasChoices("source", "service", "数据源"),
        max_length=32,
    )


class ImageOutArgs(StrictModel):
    format: Literal["text", "json", "image"] = Field(
        default="image",
        description=(
            "Prefer image (default). format=json skips drawing and returns "
            "structured data only — do NOT use json when the user expects a chart image."
        ),
    )
    include_image_b64: bool = Field(
        default=False,
        description="If true, embed PNG as base64 (large). Default returns image_path only.",
    )
    out: str | None = Field(
        default=None,
        description="Optional output PNG path or directory.",
        max_length=512,
    )


class ResolveQqInput(StrictModel):
    query: str = Field(
        ...,
        min_length=1,
        max_length=64,
        description="QQ number, QQ nickname, group card, or waterfish name",
    )
    group_id: int | None = Field(
        default=None, ge=1, description="Prefer matching within this group"
    )
    max_results: int = Field(default=10, ge=1, le=20)


class GetQqIdentityInput(StrictModel):
    qq: int = Field(..., ge=1, description="Player QQ")
    group_id: int | None = Field(default=None, ge=1)


class RefreshIdentityInput(StrictModel):
    base_url: str | None = Field(
        default=None,
        max_length=256,
        description="Override NAPCAT_BASE_URL / ONEBOT_BASE_URL for this call",
    )
    no_cache: bool = Field(
        default=True, description="Pass no_cache to get_group_member_list"
    )
    timeout_ms: int = Field(default=10000, ge=1000, le=120000)
    group_delay_ms: int | None = Field(
        default=None, ge=0, le=10000, description="Delay between groups; default env"
    )
    max_groups: int | None = Field(
        default=None, ge=1, le=500, description="Limit groups (testing)"
    )


class GroupRatingRankInput(StrictModel):
    group_id: int = Field(
        ...,
        ge=1,
        description="QQ 群号（不是玩家 QQ）。查榜时按需拉分；新鲜缓存复用。",
    )
    sort_order: Literal["asc", "desc"] = Field(
        default="desc", description="desc=高分在前，asc=低分在前"
    )
    output_limit: int | None = Field(
        default=20,
        ge=1,
        le=500,
        description="输出条数上限；不传或配合名次窗口使用",
    )
    start_rank: int | None = Field(
        default=None, ge=1, description="名次起点（1-based），须与 end_rank 成对"
    )
    end_rank: int | None = Field(
        default=None, ge=1, description="名次终点（含），须与 start_rank 成对"
    )
    rating_min: int | None = Field(default=None, ge=0, description="Rating 下限（含）")
    rating_max: int | None = Field(default=None, ge=0, description="Rating 上限（含）")
    force_refresh: bool = Field(
        default=False, description="忽略成绩缓存，查榜时全部重拉"
    )
    max_concurrency: int | None = Field(
        default=None, ge=1, le=10, description="并发拉取人数，默认 3（或环境变量）"
    )
    query_delay_ms: int | None = Field(
        default=None,
        ge=0,
        le=5000,
        description="成员间启动间隔毫秒，默认 250，用于限速",
    )
    max_members: int | None = Field(
        default=None, ge=1, le=2000, description="最多处理多少名群成员（限流/测试）"
    )


class GroupSongRankInput(StrictModel):
    group_id: int = Field(
        ...,
        ge=1,
        description="QQ 群号。查本榜时才按成员拉该曲成绩；新鲜缓存复用。",
    )
    song: str = Field(
        ...,
        min_length=1,
        max_length=128,
        description="曲目 id / 标题 / 别名",
    )
    level_index: int | None = Field(
        default=None,
        ge=0,
        le=4,
        description="0=Basic … 4=Re:Master；默认取缓存中出现过的最高难度",
    )
    sort_by: Literal["achievements", "rating", "dxScore"] = Field(
        default="achievements", description="排序字段"
    )
    sort_order: Literal["asc", "desc"] = Field(default="desc")
    output_limit: int | None = Field(default=20, ge=1, le=500)
    start_rank: int | None = Field(default=None, ge=1)
    end_rank: int | None = Field(default=None, ge=1)
    force_refresh: bool = Field(default=False, description="忽略缓存强制重拉")
    max_concurrency: int | None = Field(default=None, ge=1, le=10)
    query_delay_ms: int | None = Field(default=None, ge=0, le=5000)
    max_members: int | None = Field(default=None, ge=1, le=2000)


class GroupMemberRankInput(StrictModel):
    group_id: int | None = Field(
        default=None,
        ge=1,
        description="QQ 群号；若该玩家只在一个缓存群中可省略",
    )
    qq: int | None = Field(default=None, ge=1, description="玩家 QQ（禁止填群号）")
    target: str | None = Field(
        default=None,
        max_length=64,
        description="未给 qq 时：昵称 / 群名片 / 水鱼名（依赖身份缓存）",
    )
    song: str | None = Field(
        default=None,
        max_length=128,
        description="有则查该曲群内名次，无则查 Rating 名次",
    )
    level_index: int | None = Field(default=None, ge=0, le=4)
    context_size: int = Field(
        default=3, ge=0, le=10, description="目标上下各展示多少人"
    )
    force_refresh: bool = Field(default=False)
    max_concurrency: int | None = Field(default=None, ge=1, le=10)
    query_delay_ms: int | None = Field(default=None, ge=0, le=5000)
    max_members: int | None = Field(default=None, ge=1, le=2000)


class B50Input(PlayerArgs, ImageOutArgs):
    all_perfect: bool = Field(
        default=False,
        description="AP50 (Lxns only; requires bound QQ, not username-only).",
    )


class SongKeyInput(PlayerArgs, ImageOutArgs):
    song: str = Field(
        ...,
        description="Song id, title, or alias (e.g. '834', 'PANDORA').",
        min_length=1,
        max_length=128,
    )


class SearchInput(PlayerArgs, ImageOutArgs):
    query: str = Field(..., min_length=1, max_length=128, description="Search text")
    mode: Literal["标题", "定数", "bpm", "曲师", "谱师"] = Field(
        default="标题",
        description="Search mode: title / level_value / bpm / artist / charter",
    )
    page: int = Field(default=1, ge=1, le=100)


class ChartInput(PlayerArgs, ImageOutArgs):
    song: str = Field(..., min_length=1, max_length=128, description="Song id/title/alias")


class ScoreLineInput(StrictModel):
    diff: Literal["绿", "黄", "红", "紫", "白"] = Field(
        ..., description="Difficulty color label"
    )
    song_id: int = Field(..., ge=1, description="Song ID")
    line: float = Field(..., gt=0, lt=101, description="Target achievement line e.g. 100")


class RiseInput(PlayerArgs, ImageOutArgs):
    level: str | None = Field(default=None, description="Optional level filter e.g. 14+")
    score: int | None = Field(default=None, ge=1, description="Target rating gain")


class PlateInput(PlayerArgs, ImageOutArgs):
    ver: str = Field(..., description="Plate version char e.g. 祝")
    plan: str = Field(..., description="Plan e.g. 将/极/神/舞舞/者")
    mode: Literal["table", "progress"] = Field(default="progress")
    page: int = Field(default=1, ge=1)


class RatingTableInput(PlayerArgs, ImageOutArgs):
    level: str = Field(..., description="Level e.g. 13 or 13+")
    progress: bool = Field(default=False, description="Include personal completion")
    plan: bool = Field(default=False, description="FC/AP plan table")


class LevelProgressInput(PlayerArgs, ImageOutArgs):
    level: str = Field(...)
    plan: str = Field(..., description="e.g. ap, sss+, fdx+")
    category: str | None = Field(default=None, description="已完成/未完成/未游玩")
    page: int = Field(default=1, ge=1)


class ScoreListInput(PlayerArgs, ImageOutArgs):
    rating: str = Field(..., description="Level 14 or constant 14.0")
    page: int = Field(default=1, ge=1)


class RankingInput(PlayerArgs):
    name: str = Field(default="", description="Prober username to find rank")
    page: int = Field(default=1, ge=1)
    my: bool = Field(default=False, description="Look up self via QQ b50 username")


class RandomInput(PlayerArgs, ImageOutArgs):
    level: str = Field(...)
    chart_type: Literal["dx", "sd", "标准"] | None = None
    color: str | None = Field(default=None, description="绿黄红紫白")


class MaiWhatInput(PlayerArgs, ImageOutArgs):
    rise: bool = Field(default=False, description="Bias toward rating push charts")


class FortuneInput(PlayerArgs, ImageOutArgs):
    pass


class AliasQueryInput(StrictModel):
    name: str = Field(..., min_length=1)
    by_id: bool = Field(default=False)


class AliasAddInput(StrictModel):
    song_id: int = Field(..., ge=1)
    alias: str = Field(..., min_length=1, max_length=64)


class UpdateInput(StrictModel):
    what: list[Literal["music", "alias", "tables", "all"]] = Field(
        default_factory=lambda: ["all"],
        description="What to refresh from network / regenerate",
    )


class ThemeInput(StrictModel):
    value: str = Field(..., description="circle / prism_plus / 0 / 1")
    qq: int | None = Field(default=None, ge=1)


class SourceInput(StrictModel):
    value: str = Field(..., description="水鱼/落雪 or 0/1")
    qq: int | None = Field(default=None, ge=1)


class BindInput(StrictModel):
    qq: int | None = Field(default=None, ge=1)
    code: str | None = Field(
        default=None,
        description="Lxns OAuth code XXXX-XXXX-XXXX; omit to get authorize URL",
    )


class BindImportTokenInput(StrictModel):
    qq: int = Field(..., ge=1, description="Player QQ (binding subject)")
    import_token: str = Field(
        ...,
        min_length=8,
        max_length=256,
        description="Diving-Fish score Import-Token (not developer token)",
        validation_alias=AliasChoices("import_token", "importToken"),
    )


class UpdateRecordsInput(StrictModel):
    qq: int = Field(
        ...,
        ge=1,
        description=(
            "Player QQ. divingfish needs Import-Token bind; "
            "lxns needs maimai_user_bind_lxns OAuth."
        ),
    )
    # Required: never upload without an explicit data source.
    source: Literal["divingfish", "lxns", "both"] = Field(
        ...,
        description=(
            "REQUIRED data source for upload. "
            "divingfish / 水鱼: Import-Token; "
            "lxns / 落雪: OAuth write_player; "
            "both: dump once, upload to both."
        ),
        validation_alias=AliasChoices(
            "source",
            "sources",
            "target",
            "targets",
            "数据源",
        ),
    )
    qr_content: str = Field(
        ...,
        min_length=1,
        description="QR decode string (usually starts with SGWC)",
        validation_alias=AliasChoices("qr_content", "qrContent"),
    )
    keyship: str | None = Field(default=None, description="keychip / keyship id")
    logoutid: Literal[1, 2, 5] | None = Field(
        default=None,
        description="UserLogout type; default 5 (title-server score sync style)",
    )
    title_ver: str | None = Field(
        default=None,
        description="Title version e.g. 1.55.00",
        validation_alias=AliasChoices("title_ver", "titleVer"),
    )
    timeout: float = Field(default=240.0, ge=30, le=600)
    refresh_music: bool = Field(
        default=False,
        description="Force refresh Diving-Fish music_data cache before convert",
        validation_alias=AliasChoices("refresh_music", "refreshMusic"),
    )


class LookupSongInput(PlayerArgs, ImageOutArgs):
    query: str = Field(..., min_length=1, description="Title/alias/id to resolve")
    with_minfo: bool = Field(
        default=False,
        description="If true and identity set, also fetch personal play data",
    )


class PlayerOverviewInput(PlayerArgs, ImageOutArgs):
    with_rise: bool = Field(default=True, description="Include rise recommendations")


class PushPlanInput(PlayerArgs, ImageOutArgs):
    level: str | None = None
    score: int | None = Field(default=None, ge=1)
