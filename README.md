# maimai DX MCP Server

> 基于 Rust 构建的高性能、轻量化、模块化 **舞萌 DX (maimai DX)** Model Context Protocol (MCP) 服务端。

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/antinomie1/maimai-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/antinomie1/maimai-mcp/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/antinomie1/maimai-mcp?color=brightgreen)](https://github.com/antinomie1/maimai-mcp/releases)
[![License](https://img.shields.io/badge/License-BSD_2--Clause-blue.svg)](LICENSE)

- 📖 **[配置与部署教程 (docs/CONFIG_GUIDE.md)](docs/CONFIG_GUIDE.md)**
- 📦 **[下载预编译二进制 (GitHub Releases)](https://github.com/antinomie1/maimai-mcp/releases)**
- 绘图与业务参考：[Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX)

---

## 🌟 核心特性

- ⚡ **极致性能与轻量**：采用纯 Rust 实现，冷启动速度毫秒级，空闲内存占用低于 15MB，无繁重外部 C 库依赖。
- 🎨 **像素级纯 Rust 绘图引擎**：
  - 本项目的绘图布局、美术资产目录格式与坐标渲染逻辑基于开源项目 [Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX) 的绝对坐标体系进行纯 Rust 迁移与优化。
  - 基于 `image` + `ab_glyph` 与 `rayon` 多线程流水线，严格按绝对坐标 1:1 高保真渲染 B50、单曲结算卡片、牌子进度表与运势图。
- 🤖 **弱智能模型与小参数模型友好**：
  - **宽容的参数归一化**：自动兼容中文难度（“紫/白/黄/红/绿”）、浮点定数转换及曲名别名模糊匹配。
  - **防崩溃中文兜底**：查无数据或未绑定时返回结构化人类可读指引，避免小模型幻觉或复读调用栈。
- 🚀 **开箱即用**：
  - **双轨图文输出**：默认同时返回标准 MCP Image Content（Base64）与本地文件持久化路径，无缝对接各类 Bot 消息发送。
  - **零门槛查分**：配置全局开发者凭据后，仅需上下文中的 QQ 号即可直出一键查分图。

---

## 🛠️ MCP 工具清单 (Tool Registry)

所有工具均支持可选参数：`qq`（用户 QQ 号）、`username`（用户名）、`source`（`"divingfish"` 或 `"lxns"`）、`output_format`（`"image"`、`"text"`、`"both"`）。

| 工具名称 (Tool Name) | 功能描述 | 主要返回内容 |
| :--- | :--- | :--- |
| `mai_b50` | 查询玩家最佳 50 首成绩 (B35 + B15) | B50 预览图、Rating 及单曲明细 |
| `mai_play_score` | 查询玩家单曲/别名成绩结算卡片 | 结算图、达成率、DX分、评价等级 |
| `mai_chart_info` | 查询谱面详情、定数与 Note 物量 | 谱面预览图、各难度定数与设计师 |
| `mai_rise_score` | 计算提分建议与目标 Rating 差距 | 推荐刷分曲目与目标达成率 |
| `mai_search_song` | 模糊搜索曲目与曲师版本信息 | 匹配曲目列表与难度定数 |
| `mai_alias_query` | 查询歌曲全部已知社群别名与标签 | 别名列表与投票信息 |
| `mai_random_song` | 按难度/定数区间随机挑选曲目 | 抽取曲目卡片与参数 |
| `mai_what` | 类似“今天吃什么”的随机特色推歌 | 推荐曲目与特色文案 |
| `mai_level_progress` | 查询指定等级（如 13+）的完成度汇总 | 通关/FC/AP 等级段位汇总图 |
| `mai_plate_table` | 查询各代牌子（霸者/舞/将/极/神）进度 | 牌子完成度进度表及未通关清单 |
| `mai_bind_player` | 绑定 QQ 到水鱼/落雪账号，或切换默认源 | 绑定确认状态与信息 |
| `mai_user_settings` | 查看与修改个人查分偏好 | 当前绑定配置、默认源与展示偏好 |
| `mai_fortune` | 抽取今日舞萌运势与宜忌推荐 | 运势图片、运势值与推荐曲目 |
| `mai_group_rank` | 统计群内已绑定成员的综合 Rating 排行 | 群排行榜图表与名次 |

---

## ⚙️ 环境配置说明

服务端支持通过环境变量或 `.env` 进行配置：

```bash
# 水鱼查分网开发者 Token（配置后仅凭 QQ 号即可直接查分）
DIVINGFISH_DEVELOPER_TOKEN=your_divingfish_developer_token

# 落雪查分网开发者 Token（可选）
LXNS_DEVELOPER_TOKEN=your_lxns_token

# 静态资源根目录（默认 ./static 或 ../maimaiDX/static）
MAIMAI_STATIC_DIR=./static

# 本地数据与用户配置存储目录（默认 ./data）
MAIMAI_DATA_DIR=./data

# 生成图片临时保存目录（默认 ./output）
MAIMAI_OUTPUT_DIR=./output
```

---

## 📁 静态资源准备与目录结构

### 1. 目录结构
请将 `static` 静态资源文件夹放置在项目根目录（或通过 `MAIMAI_STATIC_DIR` 指定绝对路径）：

```
static/
├── font/              # 字体文件 (ResourceHanRoundedCN-Bold.ttf 等)
├── data/              # 歌曲/别名/牌子数据缓存 (music_data.json 等)
└── mai/
    ├── pic/           # UI 边框、评级底图、段位图标
    ├── cover/         # 曲目封面 (以 song_id 命名)
    ├── plate/         # 姓名框与牌子背景
    ├── shougou/       # 称号底图
    └── plate_version/ # 各版本牌子标识
```

### 2. 静态资源

下载并解压资源包：
- [Cloudreve](https://cloud.yuzuchan.moe/f/34s7/Resource%20CN1.55.7z)
- [OneDrive](https://yuzuai-my.sharepoint.com/:u:/g/personal/yuzu_yuzuchan_moe/IQBGKHie6MAaTZy3rME7Q-ruAVKgXDCKROqz5e25KtMeeVY?e=53eC6a)

记下其中 `static` 目录的绝对路径（即配置中的 `MAIMAI_STATIC_DIR`）。
请遵守上游美术与字体相关声明。

---

## 🚀 安装与运行
 
### 1. 方式 A：直接下载预编译二进制 (推荐)
从 [GitHub Releases](https://github.com/antinomie1/maimai-mcp/releases) 下载适合您系统架构的压缩包（Windows / Linux），解压后直接使用。

### 2. 方式 B：从源码编译
```bash
cargo build --release
```

### 3. 本地测试 MCP 服务
可以通过标准 stdio 运行：
```bash
./target/release/maimai-mcp
```

### 3. 在 AstrBot / Claude Desktop 中接入
在 `mcpServers` 配置中添加：
```json
{
  "mcpServers": {
    "maimai": {
      "command": "/path/to/maimai-mcp",
      "env": {
        "MAIMAI_STATIC_DIR": "/path/to/static",
        "DIVINGFISH_CLIENT_ID": "your_divingfish_client_id",
        "DIVINGFISH_CLIENT_SECRET": "your_divingfish_client_secret"
      }
    }
  }
}
```

---

## 🙏 致谢与开源声明 (Credits & Attribution)

本项目在开发过程中深度参考并复用了以下优秀开源项目的设计与资源规范：

- **[Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX)** (MIT License)：
  - 本项目的绘图渲染逻辑、UI 元素布局、绝对像素坐标体系以及静态资源目录结构均源自该项目的精妙设计。
  - 特此鸣谢 maimaiDX 项目的所有贡献者！
- **查分与数据源生态**：
  - 感谢 [DivingFish (水鱼查分器)](https://www.diving-fish.com/maimaidx/prober/) 提供的查分开放 API 与 OAuth 平台。
  - 感谢 [落雪咖啡屋 (LXNS)](https://maimai.lxns.net/) 提供的开放数据接口与 OAuth 平台。
  - 感谢 [Yuzuchan](https://api.yuzuchan.moe/) 提供的曲目别名数据库。

### 📜 许可证与资源权利声明

- 本代码仓库采用 [BSD 2-Clause License](LICENSE) 授权开源；
- 上游参考项目协议见 [LICENSE-UPSTREAM](LICENSE-UPSTREAM)；
- **资源版权声明**：`static` 等美术 / 字体资源**不随本代码仓库分发**，其版权以资源包与官方声明为准，**不在本开源许可证授权范围内**。请使用者自行确保素材使用的合法合规性，并遵守各字体与素材的原作者声明。

---

## 📄 架构与设计决策

- [领域模型术语表 (CONTEXT.md)](CONTEXT.md)
- [架构决策记录 (docs/adr/)](docs/adr/)
- [开发实施指南 (AGENTS.md)](AGENTS.md)
