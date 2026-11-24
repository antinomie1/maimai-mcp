# Maimai DX MCP Server (Rust Architecture Specification)

本文档定义了将 Python 原版 `maimaiDX` 项目重构为高性能、模块化、代码精简的 Rust MCP 工具集的设计规范与实现蓝图。

---

## 1. 系统目标与设计原则

1. **极简代码**：
   - 杜绝过度工程和过深的抽象层，以简单、直观的函数式和轻量结构体驱动。
   - 整体代码量严格控制在 10,000 行以内。
2. **高性能与多线程**：
   - 使用异步 I/O (`tokio`) 高并发处理查分网（水鱼 DivingFish / 落雪 LXNS / 柚子别名 Yuzuchan）的 HTTP 请求与数据同步。
   - 图片渲染阶段使用 CPU 多线程并发流水线（`rayon` 并行拼图和加载素材），大幅降低 B50 / 进度表渲染延迟。
3. **像素级绝对坐标对齐**：
   - 采用纯 Rust 图像引擎（`image` + `ab_glyph` 光栅化 + `rustybuzz` 文字整形，复刻 Pillow/libraqm 排版），严格按照原 PIL 脚本的绝对像素坐标 `(x, y)` 贴图与绘制文字。
   - 静态资源路径完全兼容原项目规范，支持 `MAIMAI_STATIC_DIR` 环境变量与配置文件路径覆盖。
4. **LLM 与 AstrBot 友好**：
   - 自动支持上下文提取 QQ 号，统一入参标准。
   - 接口支持输出纯文本、Base64 图片（MCP 标准 Image Content Block）及本地图片文件路径，无缝兼容 QQ 机器人消息发送。

---

## 2. 模块架构设计

```
src/
├── main.rs                 # MCP Server 入口与工具注册 (stdio)
├── config.rs               # 环境变量配置
├── error.rs                # 错误类型（文案与原版 handler_error 一致）
├── util.rs                 # Python 兼容格式化、列宽截断、UTC+8 日期、PRNG
│
├── core/
│   ├── constants.rs        # 版本 / 评级 / 牌子映射 (constants.py)
│   ├── models.rs           # 曲目、成绩、玩家、用户模型（兼容 merge_music_data.json）
│   ├── calc.rs             # 单曲 Rating、评级、DX 星级
│   ├── music.rs            # 内存曲库、别名、牌子、等级表（原子替换）
│   ├── store.rs            # 按 QQ 号的用户配置 (JSON) 与 OAuth 待回填会话
│   ├── update.rs           # 联网更新 + 合并 + 每日调度 + 表格重绘
│   ├── service.rs          # 业务逻辑 (handler.py)：B50、成绩、上分推荐等
│   └── clients/
│       ├── divingfish.rs   # 水鱼：公开 B50、OAuth 成绩、排行、OAuth 绑定
│       ├── lxns.rs         # 落雪：OAuth（自动刷新）与开发者 API
│       └── yuzuchan.rs     # 柚子：别名、牌子数据
│
├── draw/                   # 与原版 PIL 坐标一一对应
│   ├── text.rs             # 字体、整形、锚点、描边、多行、text_to_image
│   ├── canvas.rs           # 素材缓存、预乘缩放、alpha_composite、渐变、毛玻璃
│   ├── base.rs             # 成绩卡网格 (whiledraw)
│   ├── best50.rs / chart.rs / info.rs / song_list.rs / score.rs
│   ├── rating_table.rs / plate_table.rs
│   └── update_table.rs     # 定数表 / 完成表底图生成（rayon 并行）
│
└── tools/                  # MCP 工具实现
    ├── b50.rs              # b50 / 排行 / 群排行
    ├── score.rs            # info / ginfo / 分数线 / 上分
    ├── music.rs            # 谱面 / 查歌 / 别名 / 随机 / mai什么 / 运势
    ├── progress.rs         # 定数表 / 完成表 / 进度 / 分数列表 / 牌子
    └── user.rs             # 水鱼与落雪绑定 / 设置 / 数据更新

tests/                      # 单元测试
```

---

## 3. LLM MCP 工具集定义

用户相关工具入参 `qq`；出图工具支持 `output_format`（`"image"` 默认 / `"text"`）。
工具与原版指令的对应关系见 README「MCP 工具清单」。

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
