# maimai DX MCP Server 配置与部署使用指南

本文档为 **maimai DX MCP Server** 的完整配置与部署教程，涵盖二进制下载、静态资源准备、查分网凭据配置、各大 MCP 客户端（AstrBot、Claude Desktop、Cursor 等）接入以及常见问题排查。

---

## 目录

1. [快速开始与二进制安装](#1-快速开始与二进制安装)
2. [静态美术与字体资源配置](#2-静态美术与字体资源配置)
3. [查分平台凭据配置 (水鱼 / 落雪)](#3-查分平台凭据配置-水鱼--落雪)
4. [环境变量全景参考](#4-环境变量全景参考)
5. [主流 MCP 客户端接入示例](#5-主流-mcp-客户端接入示例)
   - [5.1 AstrBot (QQ 机器人接入)](#51-astrbot-qq-机器人接入)
   - [5.2 Claude Desktop](#52-claude-desktop)
   - [5.3 Cursor / Windsurf / VSCode Cline](#53-cursor--windsurf--vscode-cline)
6. [工具清单与调用说明](#6-工具清单与调用说明)
7. [常见问题排查 (FAQ)](#7-常见问题排查-faq)

---

## 1. 快速开始与二进制安装

### 方式 A：直接下载预编译二进制（推荐）

前往 [GitHub Releases](https://github.com/antinomie1/maimai-mcp/releases) 页面下载对应系统的最新发布包：

- **Windows**: `maimai-mcp-v0.3.0-x86_64-pc-windows-msvc.zip`
- **Linux**: `maimai-mcp-v0.3.0-x86_64-unknown-linux-gnu.tar.gz`

解压后即可得到单个可执行文件 `maimai-mcp` (Linux) 或 `maimai-mcp.exe` (Windows)。

### 方式 B：从源码编译构建

如果你安装了 Rust 开发环境（Rust 1.85+，支持 2024 Edition）：

```bash
git clone https://github.com/antinomie1/maimai-mcp.git
cd maimai-mcp
cargo build --release
```

编译生成的目标二进制文件位于 `target/release/maimai-mcp`（Windows 上为 `target/release/maimai-mcp.exe`）。

---

## 2. 静态美术与字体资源配置

> **重要说明**：受版权声明限制，本仓库不随代码直接分发美术与字体静态包。首次使用前需下载并配置静态素材。

### 2.1 资源包下载

可任选以下分流下载静态资源压缩包：
- **分流 1 (Cloudreve)**: [Resource CN1.55.7z](https://cloud.yuzuchan.moe/f/34s7/Resource%20CN1.55.7z)
- **分流 2 (OneDrive)**: [Resource CN1.55](https://yuzuai-my.sharepoint.com/:u:/g/personal/yuzu_yuzuchan_moe/IQBGKHie6MAaTZy3rME7Q-ruAVKgXDCKROqz5e25KtMeeVY?e=53eC6a)

### 2.2 目录结构放置

解压后，将静态资源解压到本地某个目录（例如 `./static` 或 `/opt/maimai-mcp/static`），其层级结构应如下：

```
static/
├── font/                      # 字体文件
│   ├── ResourceHanRoundedCN-Bold.ttf
│   ├── ShangguMonoSC-Regular.otf
│   ├── Torus SemiBold.otf
│   └── FOT-NewRodin Pro EB.otf
├── data/                      # 歌曲元数据与别名索引
│   ├── music_data.json
│   ├── lxns_music_data.json
│   ├── music_chart.json
│   ├── plate_data.json
│   ├── merge_music_data.json
│   └── merge_music_alias.json
└── mai/
    ├── pic/                   # UI 底图、段位框、评级图标
    ├── cover/                 # 曲目封面 (如 11253.png 等)
    ├── plate/                 # 姓名框及名牌底图
    ├── shougou/               # 称号背景
    └── plate_version/         # 版本标识与各代牌子图标
```

---

## 3. 查分平台凭据配置 (水鱼 / 落雪)

maimai DX MCP Server 支持 **水鱼 (DivingFish)** 与 **落雪咖啡屋 (LXNS)** 双查分源。

### 3.1 水鱼查分网 (DivingFish)

- **开发者 Token 模式（推荐，全局免密查分）**：
  - 前往 [水鱼查分器](https://www.diving-fish.com/maimaidx/prober/) 申请开发者 Token；
  - 设置环境变量 `DIVINGFISH_DEVELOPER_TOKEN`；
  - 设置后，只要用户的查分数据为“公开”或导入过水鱼，无需用户绑定即可仅凭 QQ 号一键出图。
- **OAuth 2.0 模式（可选）**：
  - 配置 `DIVINGFISH_CLIENT_ID` 与 `DIVINGFISH_CLIENT_SECRET`。

### 3.2 落雪查分网 (LXNS)

- **开发者 Token 模式**：
  - 前往 [落雪开放平台](https://maimai.lxns.net/) 申请开发者 Token；
  - 设置环境变量 `LXNS_DEVELOPER_TOKEN`。
- **OAuth 2.0 模式（可选）**：
  - 配置 `LXNS_CLIENT_ID`、`LXNS_CLIENT_SECRET` 与 `LXNS_REDIRECT_URI`。

---

## 4. 环境变量全景参考

可通过系统环境变量、`.env` 文件或客户端配置中的 `env` 字段指定：

| 环境变量名 | 类型 | 说明 | 默认值 |
| :--- | :--- | :--- | :--- |
| `MAIMAI_STATIC_DIR` | 路径 | 静态资源根目录绝对路径 | `./static` |
| `MAIMAI_DATA_DIR` | 路径 | 用户数据、绑定与持久化配置存储目录 | `./data` |
| `MAIMAI_OUTPUT_DIR` | 路径 | 生成图片输出目录 | `./output` |
| `DIVINGFISH_DEVELOPER_TOKEN` | 字符串 | 水鱼查分网开发者 Token | 无 |
| `DIVINGFISH_CLIENT_ID` | 字符串 | 水鱼查分网 OAuth Client ID | 无 |
| `DIVINGFISH_CLIENT_SECRET`| 字符串 | 水鱼查分网 OAuth Client Secret | 无 |
| `LXNS_DEVELOPER_TOKEN` | 字符串 | 落雪查分网开发者 Token | 无 |
| `LXNS_CLIENT_ID` | 字符串 | 落雪查分网 OAuth Client ID | 无 |
| `LXNS_CLIENT_SECRET` | 字符串 | 落雪查分网 OAuth Client Secret | 无 |
| `LXNS_REDIRECT_URI` | 字符串 | 落雪查分网 OAuth 回调地址 | 无 |
| `RUST_LOG` | 字符串 | 日志等级 (`info`, `debug`, `warn`) | `info` |

---

## 5. 主流 MCP 客户端接入示例

### 5.1 AstrBot (QQ 机器人接入)

AstrBot 支持标准 MCP 服务。在 AstrBot 的 MCP 服务配置文件中加入以下配置：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "/opt/maimai-mcp/maimai-mcp",
      "args": [],
      "env": {
        "MAIMAI_STATIC_DIR": "/opt/maimai-mcp/static",
        "MAIMAI_DATA_DIR": "/opt/maimai-mcp/data",
        "MAIMAI_OUTPUT_DIR": "/opt/maimai-mcp/output",
        "DIVINGFISH_DEVELOPER_TOKEN": "your_divingfish_developer_token_here",
        "LXNS_DEVELOPER_TOKEN": "your_lxns_developer_token_here"
      }
    }
  }
}
```

> **提示**：大模型对话时，AstrBot 会自动将提问者的 QQ 号传递给 `mai_b50` 等工具，玩家直接在群聊中发送“查 b50”即可自动出图！

### 5.2 Claude Desktop

编辑 Claude Desktop 配置文件：
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

#### Windows 示例：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "C:\\Tools\\maimai-mcp\\maimai-mcp.exe",
      "env": {
        "MAIMAI_STATIC_DIR": "C:\\Tools\\maimai-mcp\\static",
        "MAIMAI_DATA_DIR": "C:\\Tools\\maimai-mcp\\data",
        "MAIMAI_OUTPUT_DIR": "C:\\Tools\\maimai-mcp\\output",
        "DIVINGFISH_DEVELOPER_TOKEN": "your_divingfish_token"
      }
    }
  }
}
```

#### Linux / macOS 示例：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "/usr/local/bin/maimai-mcp",
      "env": {
        "MAIMAI_STATIC_DIR": "/var/lib/maimai-mcp/static",
        "MAIMAI_DATA_DIR": "/var/lib/maimai-mcp/data",
        "MAIMAI_OUTPUT_DIR": "/var/lib/maimai-mcp/output",
        "DIVINGFISH_DEVELOPER_TOKEN": "your_divingfish_token"
      }
    }
  }
}
```

### 5.3 Cursor / Windsurf / VSCode Cline

在 MCP Server 配置界面或配置文件中添加：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "/path/to/maimai-mcp",
      "env": {
        "MAIMAI_STATIC_DIR": "/path/to/static",
        "DIVINGFISH_DEVELOPER_TOKEN": "your_divingfish_token"
      }
    }
  }
}
```

---

## 6. 工具清单与调用说明

maimai DX MCP Server 注册了以下工具供 LLM 智能调用：

| 工具名称 (Tool) | 入参示例 | 主要功能 |
| :--- | :--- | :--- |
| `mai_b50` | `{"qq": "12345678", "theme": "prism_plus"}` | 查询玩家 Best 50 (B35 + B15) 成绩卡片 |
| `mai_play_score` | `{"qq": "12345678", "song_name": "Valsqetch"}` | 查询某首曲目或别名的单曲结算与达成率 |
| `mai_chart_info` | `{"song_name": "潘多拉", "level_index": 3}` | 查询谱面详情、定数、Note物量分布 |
| `mai_rise_score` | `{"qq": "12345678", "target_rating": 15000}` | 计算上分推荐与达到目标 Rating 所需分数 |
| `mai_search_song` | `{"keyword": "Garakuta"}` | 根据曲名/别名/曲师搜索曲目及定数 |
| `mai_alias_query` | `{"song_name": "八钱"}` | 查询曲目的社群别名与投票来源 |
| `mai_random_song` | `{"level": "14+", "song_type": "dx"}` | 按难度/定数/曲目分类随机抽曲 |
| `mai_what` | `{}` | 类似“今天吃什么”的特色推歌与文案 |
| `mai_level_progress`| `{"qq": "12345678", "level": "13+"}` | 查询指定等级的通关/FC/AP完成度统计 |
| `mai_plate_table` | `{"qq": "12345678", "version": "真", "plate_type": "将"}` | 查询特定代数牌子完成度与未完成曲目表 |
| `mai_bind_player` | `{"qq": "12345678", "divingfish_username": "player_foo"}` | 绑定 QQ 到水鱼或落雪账号 |
| `mai_user_settings`| `{"qq": "12345678", "theme": "prism_plus"}` | 查看或修改个人查分偏好与主题 |
| `mai_fortune` | `{"qq": "12345678"}` | 今日舞萌运势、运势值与推荐吉曲 |
| `mai_group_rank` | `{"qq_list": ["123", "456", "789"]}` | 群内已绑定玩家综合 Rating 排行榜 |

---

## 7. 常见问题排查 (FAQ)

### Q1: 启动时报错 `Failed to load font ...` 或图片找不到？
- **原因**：`MAIMAI_STATIC_DIR` 路径不正确或未解压完整静态素材。
- **排查**：检查 `MAIMAI_STATIC_DIR/font/ResourceHanRoundedCN-Bold.ttf` 文件是否存在，并确保配置的是绝对路径。

### Q2: 查分提示 `未找到该玩家的成绩数据`？
- **排查**：
  1. 确认该玩家是否已在水鱼或落雪查分网上导入过成绩；
  2. 确认该玩家在查分网设置中是否开启了“公开成绩”；
  3. 如果未公开，服务端需要配置开发者 Token 或由玩家通过 `mai_bind_player` 绑定对应账号 Token。

### Q3: Windows 下路径含有空格或斜杠报错？
- **排查**：在 JSON 配置文件中，Windows 路径的反斜杠必须使用双反斜杠转义，例如：`"C:\\Users\\Admin\\maimai-mcp\\static"`。

### Q4: 如何在不同主题间切换？
- **解答**：支持 `prism_plus`（PRISM+ 新框体主题，默认）与 `circle`（经典主题）。可以通过工具入参 `theme` 单次指定，或使用 `mai_user_settings` 保存为用户默认偏好。
