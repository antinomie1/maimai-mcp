# Maimai DX MCP Server (Rust Architecture Specification)

本文档定义了将 Python 原版 `maimaiDX` 项目重构为高性能、模块化、代码精简的 Rust MCP 工具集的设计规范与实现蓝图。

---

## 1. 系统目标与设计原则

1. **通俗易懂与极简代码**：
   - 杜绝过度工程和过深的抽象层，以简单、直观的函数式和轻量结构体驱动。
   - 整体代码量严格控制在 10,000 行以内。
2. **高性能与多线程**：
   - 使用异步 I/O (`tokio`) 高并发处理查分网（水鱼 DivingFish / 落雪 LXNS / 柚子别名 Yuzuchan）的 HTTP 请求与数据同步。
   - 图片渲染阶段使用 CPU 多线程并发流水线（`rayon` 并行拼图和加载素材），大幅降低 B50 / 进度表渲染延迟。
3. **像素级绝对坐标对齐**：
   - 采用纯 Rust 图像引擎（`image` + `imageproc` + `ab_glyph`），严格按照原 PIL 脚本的绝对像素坐标 `(x, y)` 贴图与绘制文字。
   - 静态资源路径完全兼容原项目规范，支持 `MAIMAI_STATIC_DIR` 环境变量与配置文件路径覆盖。
4. **LLM 与 AstrBot 友好**：
   - 自动支持上下文提取 QQ 号，统一入参标准。
   - 接口支持输出纯文本、Base64 图片（MCP 标准 Image Content Block）及本地图片文件路径，无缝兼容 QQ 机器人消息发送。

---

## 2. 模块架构设计

```
src/
├── main.rs                 # 服务入口、MCP Server 启动与 stdio 传输监听
├── config.rs               # 环境变量与资源路径配置 (MAIMAI_STATIC_DIR 等)
├── error.rs                # 统一轻量错误处理
│
├── core/
│   ├── models/             # 领域模型定义
│   │   ├── music.rs        # 曲目、谱面、等级与别名结构
│   │   ├── score.rs        # 成绩、分段、Rating 与 DX 分计算
│   │   └── user.rs         # 用户配置、绑定信息与凭据
│   │
│   ├── database/           # 本地数据存储与缓存
│   │   ├── music_db.rs     # 曲目库与谱面缓存索引
│   │   ├── alias_db.rs     # 别名数据库与模糊匹配
│   │   └── user_store.rs   # 按 QQ 号隔离的用户配置存储 (JSON)
│   │
│   ├── clients/            # 外部查分服务客户端 (Reqwest 异步请求)
│   │   ├── divingfish.rs   # 水鱼查分网 API Client
│   │   ├── lxns.rs         # 落雪查分网 API Client
│   │   └── yuzuchan.rs     # 柚子查分/别名数据源 Client
│   │
│   └── calc/               # 游戏数值算法
│       ├── rating.rs       # 单曲 Rating、B50 算法 (35 旧 + 15 新)
│       └── progress.rs     # 牌子、完成度、段位进度统计
│
├── draw/                   # 纯 Rust 绝对坐标绘图引擎 (image + ab_glyph + rayon)
│   ├── assets.rs           # 字体与图片素材加载/缓存管理器
│   ├── canvas.rs           # 坐标贴图、文字度量与绘制辅助器
│   ├── b50.rs              # B50 成绩总览图生成
│   ├── play_score.rs       # 单曲成绩结算卡片生成
│   ├── chart_info.rs       # 谱面信息预览卡片生成
│   ├── level_progress.rs   # 定数/等级完成度表生成
│   ├── plate_table.rs      # 牌子（霸者/舞舞/将/极/神等）进度表生成
│   ├── rating_table.rs     # Rating 达成表生成
│   └── fortune.rs          # 每日舞萌运势图生成
│
└── tools/                  # 供 LLM 调用的 MCP Tool 路由定义
    ├── b50.rs              # b50 查询与出图
    ├── score.rs            # 单曲成绩、分数线、上分推荐 (play_score / rise_score)
    ├── music.rs            # 曲目搜索、谱面信息、别名检索 (search_song / chart_info / alias)
    ├── random.rs           # 随机选曲、今天吃什么 (random_song / mai_what)
    ├── progress.rs         # 完成度与牌子进度 (level_progress / plate_table / level_score_list)
    ├── user.rs             # 绑定用户、查分源切换 (bind_player / user_settings)
    ├── fortune.rs          # 今日运势 (fortune)
    └── group_rank.rs       # 群内综合排行 (group_rank)
```

---

## 3. LLM MCP 工具集定义

所有工具统一支持可选入参：
- `qq`: 用户 QQ 号（默认尝试从上下文自动获取）。
- `username`: 水鱼/落雪用户名或开发者 Token（当未传 QQ 或覆盖指定用户时使用）。
- `source`: 查分器源（`"divingfish"` 或 `"lxns"`，未传时使用用户本地配置的默认值）。
- `output_format`: 输出格式（`"image"` (默认)、`"text"` 或 `"both"`）。

| 工具名 (Tool Name) | 核心功能 | 主要返回内容 |
| :--- | :--- | :--- |
| `mai_b50` | 查询玩家最佳 50 首成绩 (B35 + B15) | B50 预览图、玩家 Rating、最佳单曲列表 |
| `mai_play_score` | 查询玩家某首歌曲/别名的游玩成绩 | 单曲成绩结算卡片、达成率、DX分、评价等级 |
| `mai_chart_info` | 查询谱面详细信息、定数、物量、配置 | 谱面详情卡片、各难度定数、设计师、Note统计 |
| `mai_rise_score` | 计算上分推荐与达到特定 Rating 所需分数 | 建议提分曲目列表与目标达成率 |
| `mai_search_song` | 根据曲名、别名、曲师、版本模糊搜索歌曲 | 匹配歌曲列表、ID、等级信息 |
| `mai_alias_query` | 查询某首歌曲的全部已知别名及来源 | 别名列表、标签与投票信息 |
| `mai_random_song` | 根据难度、定数范围、类型随机挑选曲目 | 随机抽取的曲目卡片与参数 |
| `mai_what` | 类似“今天吃什么”的主题/条件随机推歌 | 推荐曲目与特色文案 |
| `mai_level_progress`| 查询指定等级（如 13+）的完成度汇总 | 等级段位通关/FC/AP 统计图与明细 |
| `mai_plate_table` | 查询特定代数牌子（如舞、霸者、真等）的达成进度 | 牌子进度预览表图及未完成曲目清单 |
| `mai_bind_player` | 绑定用户 QQ 到水鱼/落雪账号，或切换默认源 | 绑定状态与配置确认信息 |
| `mai_user_settings`| 查看与修改个人查分偏好 | 当前绑定信息、默认查分网、展示偏好 |
| `mai_fortune` | 抽取今日舞萌运势与宜忌推荐 | 运势图片、运势值、推荐游玩曲目 |
| `mai_group_rank` | 统计群内已绑定成员的 Rating / 综合排行 | 群排行榜图表与文本名次 |

---

## 4. 静态资源与配置约定

### 4.1 环境变量配置
```bash
# 静态资源根目录（默认 ./static 或 ../maimaiDX/static）
MAIMAI_STATIC_DIR=/path/to/maimaiDX/static

# 本地数据与用户配置存储目录（默认 ./data）
MAIMAI_DATA_DIR=./data

# 输出临时图片目录（默认 ./output）
MAIMAI_OUTPUT_DIR=./output
```

### 4.2 资源文件层级
```
static/
├── font/
│   ├── ResourceHanRoundedCN-Bold.ttf
│   ├── ShangguMonoSC-Regular.otf
│   ├── Torus SemiBold.otf
│   └── FOT-NewRodin Pro EB.otf
├── data/
│   ├── music_data.json
│   ├── lxns_music_data.json
│   ├── music_chart.json
│   ├── plate_data.json
│   ├── merge_music_data.json
│   └── merge_music_alias.json
└── mai/
    ├── pic/            # 界面底图、Rating框、段位图标
    ├── cover/          # 曲目封面图片 (以 song_id 命名)
    ├── plate/          # 姓名框及牌子背景
    ├── shougou/        # 称号背景
    └── plate_version/  # 版本标识与各代牌子图标
```

---

## 5. 阶段实施计划

1. **基础设施层**：
   - 建立 `config`, `models`, `error`, `user_store` 模块。
   - 建立 `music_db`, `alias_db` 本地 JSON 索引与加载器。
2. **外部网络客户端**：
   - 实现 `divingfish`, `lxns`, `yuzuchan` API Client 及统一数据适配层。
3. **纯 Rust 绘图底层**：
   - 构建 `assets` 缓存池与 `canvas` 像素级绘制辅助工具（字型加载、图层混合、圆角裁剪）。
   - 按原项目坐标迁移 `b50`, `play_score`, `chart_info`, `fortune`, `progress` 绘图逻辑。
4. **MCP Tools 组装**：
   - 实现各业务 Tool，支持入参自适应与图片/文本多格式返回。
5. **联调与测试**：
   - 验证与 AstrBot / MCP Inspector 联调测试，完成性能压测。
