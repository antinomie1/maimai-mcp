# maimai DX MCP Server

> 基于 Rust 构建的高性能、轻量化、模块化 **舞萌 DX (maimai DX)** Model Context Protocol (MCP) 服务端。

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/antinomie1/maimai-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/antinomie1/maimai-mcp/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

- 📖 **[配置与部署教程 (docs/CONFIG_GUIDE.md)](docs/CONFIG_GUIDE.md)**
- 📦 **[下载预编译二进制 (GitHub Releases)](https://github.com/antinomie1/maimai-mcp/releases)**
- 绘图与业务参考：[Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX)

---

## 🌟 核心特性

- ⚡ **高性能**：纯 Rust 实现，B50 渲染约 60 ms；定数表与完成表（43 张）多线程生成约 5 s（原版约 25 s）。封面按需并行解码不常驻内存。
- 🎨 **像素级复刻**：按原版 PIL 绝对坐标绘制；文字排版复刻 Pillow + libraqm（HarfBuzz 整形、FreeType 取整、锚点、描边），与原版输出尺寸一致、平均像素误差 < 1/255。
- 🔐 **OAuth 绑定**：水鱼 OAuth（授权链接 + 确认码，校验绑定归属）与落雪 OAuth（自动刷新令牌）。
- 🔄 **数据自动更新**：启动时及每日 04:00（UTC+8）联网更新曲目（水鱼 + 落雪合并）、别名（柚子 + 落雪 + 本地）与牌子数据，并自动重绘表格；失败时回退本地暂存。
- 🤖 **LLM 友好**：入参宽容（中文难度、别名、ID 均可），错误以可读中文返回；输出 MCP 图片 + 文本，同时保存本地图片路径。

---

## 🛠️ MCP 工具清单

所有出图工具支持 `output_format`：`"image"`（默认，图片 + 文本）或 `"text"`。

| 工具 | 对应原版指令 | 功能 |
| :--- | :--- | :--- |
| `mai_b50` | b50 / ap50 | B50 / AP50 图，可按 QQ 或水鱼用户名查询 |
| `mai_play_score` | info | 单曲各难度成绩 |
| `mai_song_stats` | ginfo | 谱面全服统计（文本） |
| `mai_score_line` | 分数线 | 目标达成率允许的 GREAT 数 |
| `mai_rise_score` | 我要在13+上5分 | 上分推荐 |
| `mai_chart_info` | XXX是什么歌 / id | 谱面信息卡（带个人加分计算） |
| `mai_search_song` | 查歌 / 定数查歌 / bpm查歌 / 曲师查歌 / 谱师查歌 | 搜索，多结果出曲目列表图 |
| `mai_alias_query` | XXX有什么别名 | 别名查询 |
| `mai_random_song` | 随个[dx][紫]13+ | 随机谱面 |
| `mai_what` | mai什么 / 今日mai打什么上分 | 随机或推分推荐 |
| `mai_fortune` | 今日运势 | 人品值、宜忌与推荐曲 |
| `mai_rating_table` | 13+定数表 | 定数表 |
| `mai_level_table` | 13+完成表 / 14ap完成表 | 等级完成表 |
| `mai_level_progress` | 14 sss 进度 / 14ap未完成进度 | 等级进度与分类列表 |
| `mai_level_score_list` | 13+分数列表 / 13.7分数列表 | 分数列表 |
| `mai_plate_table` | 祝极完成表 / 牌子条件 | 牌子完成表 |
| `mai_plate_progress` | 舞将进度 | 牌子未完成谱面 |
| `mai_bind_divingfish` | 绑定水鱼 / 确认码 | 水鱼 OAuth 绑定 |
| `mai_bind_lxns` | lxbind / 授权码 | 落雪 OAuth 绑定 |
| `mai_user_settings` | 数据源 / 主题 | 查看与切换数据源、主题 |
| `mai_rating_ranking` | 查看排名 / 我的排名 | 水鱼查分器排行榜 |
| `mai_group_rank` | — | 群内 Rating 排行 |
| `mai_update_data` | 更新maimai数据 / 更新别名库 / 更新定数表 | 手动更新数据与表格 |

---

## ⚙️ 配置

完整说明见 [配置与部署指南](docs/CONFIG_GUIDE.md)。最小配置：

```bash
MAIMAI_STATIC_DIR=/path/to/static        # 资源包中的 static 目录
DIVINGFISH_CLIENT_ID=...                 # 水鱼 OAuth（未配置时仅 b50 可用）
DIVINGFISH_CLIENT_SECRET=...
LX_CLIENT_ID=... LX_CLIENT_SECRET=... REDIRECT_URI=...   # 落雪 OAuth（可选）
LXNS_DEV_TOKEN=...                       # 落雪开发者 Token（可选）
```

资源包：[Cloudreve](https://cloud.yuzuchan.moe/f/34s7/Resource%20CN1.55.7z) / [OneDrive](https://yuzuai-my.sharepoint.com/:u:/g/personal/yuzu_yuzuchan_moe/IQBGKHie6MAaTZy3rME7Q-ruAVKgXDCKROqz5e25KtMeeVY?e=53eC6a)，
增量包 [Resource 3.1.5 UPDATE](https://cloud.yuzuchan.moe/f/x3f3/Resource%203.1.5%20UPDATE.7z)。请遵守上游美术与字体声明。

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

### 4. 在 AstrBot / Claude Desktop 中接入
在 `mcpServers` 配置中添加：
```json
{
  "mcpServers": {
    "maimai": {
      "command": "/path/to/maimai-mcp",
      "env": {
        "MAIMAI_STATIC_DIR": "/path/to/static",
        "DIVINGFISH_CLIENT_ID": "your_client_id",
        "DIVINGFISH_CLIENT_SECRET": "your_client_secret"
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

- 本代码仓库采用 [MIT License](LICENSE) 授权开源；
- 上游参考项目协议见 [LICENSE-UPSTREAM](LICENSE-UPSTREAM)；
- **资源版权声明**：`static` 等美术 / 字体资源**不随本代码仓库分发**，其版权以资源包与官方声明为准，**不在本开源许可证授权范围内**。请使用者自行确保素材使用的合法合规性，并遵守各字体与素材的原作者声明。

---

## 📄 架构与设计决策

- [领域模型术语表 (CONTEXT.md)](CONTEXT.md)
- [架构决策记录 (docs/adr/)](docs/adr/)
- [开发实施指南 (AGENTS.md)](AGENTS.md)
