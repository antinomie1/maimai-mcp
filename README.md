# maimai-mcp

[![PyPI](https://img.shields.io/badge/PyPI-0.1-blue)](https://pypi.org/project/maimai-mcp/0.1/)
[![Python](https://img.shields.io/badge/python-3.10-blue)](https://pypi.org/project/maimai-mcp/)
[![MCP Badge](https://lobehub.com/badge/mcp/antinomie1-maimai-mcp?style=flat)](https://lobehub.com/mcp/antinomie1-maimai-mcp)

本地可用的舞萌 DX 查询库 / CLI / MCP 服务：查曲、查分、进度与绘图，供命令行或任意 Agent / 宿主调用。

- 仓库：https://github.com/antinomie1/maimai-mcp  
- LobeHub：https://lobehub.com/mcp/antinomie1-maimai-mcp  
- 绘图与业务参考：[Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX)

Python ≥ 3.10。成绩默认走[水鱼查分器](https://www.diving-fish.com/maimaidx/prober/)，也可切换到[落雪](https://maimai.lxns.net/)（需 Token / OAuth 绑定）。出图默认主题为 **circle**，可按 QQ 改为 **prism_plus**。

CLI 命令（`maimai …`）与 MCP 工具（`maimai_*`）能力一一对应；Agent 侧参数须包在 `params` 中，查分时每次显式传入玩家 `qq` 或 `username`（MCP 不会自动读聊天上下文）。

---

## 安装与配置

需要 **Python ≥ 3.10**。出图依赖外部 **static** 资源（不随包分发）。

### 1. 安装

任选其一：

| 方式 | 命令 |
|------|------|
| uv 工具（推荐） | `uv tool install maimai-mcp` |
| uv 临时运行 | `uvx maimai-mcp`（只跑 MCP，不装全局命令） |
| pip | `pip install maimai-mcp` |

安装后可用：

- `maimai` — 命令行
- `maimai-mcp` — stdio MCP 服务

也可用 `python -m maimai_mcp.cli` / `python -m maimai_mcp`。

### 2. 静态资源

1. 下载并解压资源包：
   - [Cloudreve](https://cloud.yuzuchan.moe/f/34s7/Resource%20CN1.55.7z)
   - [OneDrive](https://yuzuai-my.sharepoint.com/:u:/g/personal/yuzu_yuzuchan_moe/IQBGKHie6MAaTZy3rME7Q-ruAVKgXDCKROqz5e25KtMeeVY?e=53eC6a)
2. 记下其中 **`static` 目录的绝对路径**（即 `STATIC_PATH`）。
3. 请遵守上游美术与字体相关声明。

### 3. 环境变量

写在 MCP 客户端的 `env`、系统环境，或 `maimai_mcp/.env`。完整列表见 [`.env.example`](maimai_mcp/.env.example)。

| 变量 | 必填 | 说明 |
|------|------|------|
| `STATIC_PATH` | **是** | 上一步 `static` 的绝对路径 |
| `DIVINGFISH_TOKEN` | 建议 | [水鱼开发者 Token](https://www.diving-fish.com/maimaidx/prober/)，**不是** Import-Token |
| `OUTPUT_DIR` | 否 | 出图目录 |

### 4. 接入 MCP

推荐配置（`uvx`，不必事先 install）：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "uvx",
      "args": ["maimai-mcp"],
      "env": {
        "STATIC_PATH": "/path/to/static",
        "OUTPUT_DIR": "/path/to/output",
        "DIVINGFISH_TOKEN": ""
      }
    }
  }
}
```

说明：

- 路径改成你的机器上的真实路径；Windows 示例：`"C:\\path\\to\\static"`
- 若已 `uv tool install` 或 `pip install`，可改为 `"command": "maimai-mcp"`，并去掉 `args`
- 更多字段见 [`mcp.example.json`](mcp.example.json)
- Agent 调用约定见 [`skills/maimai-mcp/`](skills/maimai-mcp/)

### 5. 自检

```bash
maimai update tables
maimai b50 --qq <QQ>
maimai chart 834
```

能出图、客户端能列出 `maimai_*` 工具即可。

---

## 从源码开发

仅二次开发或跟踪仓库最新代码时需要：

```bash
git clone https://github.com/antinomie1/maimai-mcp.git
cd maimai-mcp
uv sync                 # 或 pip install -e .
# 开发依赖：uv sync --extra dev
cp maimai_mcp/.env.example maimai_mcp/.env   # 至少填 STATIC_PATH
```

静态资源与环境变量见上文。MCP 可在仓库目录用 `uv run`：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "uv",
      "args": ["run", "maimai-mcp"],
      "cwd": "/path/to/maimai-mcp",
      "env": {
        "STATIC_PATH": "/path/to/static",
        "DIVINGFISH_TOKEN": ""
      }
    }
  }
}
```

自检：`uv run maimai update tables`、`uv run maimai b50 --qq <QQ>`。  
联调：`scripts/run_inspector.ps1`、`scripts/smoke_mcp_tools.py`。

---

## 工具列表

以下为 MCP 工具名与简要说明。CLI 侧为同名能力（例如 `maimai b50` 对应 `maimai_b50`）。  
查分 / 出图类工具通常需要 `params.qq` 或 `params.username`；出图默认 `format: image`，成功时返回 `image_path`。

更细的 Agent 约定见 [`skills/maimai-mcp/references/tools.md`](skills/maimai-mcp/references/tools.md)。

### 成绩与进度

| 工具 | 说明 |
|------|------|
| `maimai_b50` | 拉取并绘制 Best 50；水鱼可用 `username`，绑定服务可用 `qq`。`all_perfect=true` 为 AP50（需落雪 + 已绑定 QQ） |
| `maimai_minfo` | 单曲个人成绩（minfo），须指定玩家身份 |
| `maimai_score_list` | 按等级或定数（如 `14.0`）列出该玩家已打 / 相关分数 |
| `maimai_plate` | 牌子完成表或进度图（如 `ver=祝` `plan=将`，`mode=progress` / `table`） |
| `maimai_plate_status` | 与 `maimai_plate` 等价的薄封装，便于 Agent 发现「牌子进度」意图 |
| `maimai_rating_table` | 等级定数表；`progress=true` 时叠加个人完成情况 |
| `maimai_level_progress` | 某等级 + 目标的完成进度（如 `level=14` `plan=ap`） |
| `maimai_rise` | 上分推荐谱面，可按等级 / 分数等条件筛选 |
| `maimai_ranking` | 水鱼公开 rating 榜；可查榜、搜名，或 `my=true` 看自己位置 |
| `maimai_fortune` | 今日运势 + 曲绘（娱乐向；`qq` / `username` 仅作随机种子） |

### 曲库与谱面

| 工具 | 说明 |
|------|------|
| `maimai_search` | 按曲名、定数、BPM、曲师、谱师等搜索；多结果返回列表，唯一命中时可直接出谱面图 |
| `maimai_lookup_song` | 组合：搜索 → 出谱面图；可选 `with_minfo` 附带个人成绩（内部串联 search + chart ± minfo） |
| `maimai_chart` | 按 ID / 曲名 / 别名绘制谱面信息图；可选带玩家身份以计算推分 |
| `maimai_score_line` | 计算指定达成线（如 100%）下的 TAP GREAT 容错 |
| `maimai_random` | 按等级 / 谱面类型 / 颜色随机一首并出谱面图 |
| `maimai_mai_what` | 随机推曲；`rise=true` 时偏向有上分空间的谱面 |
| `maimai_alias_query` | 查询某曲的别名列表 |
| `maimai_alias_local_add` | 仅写入本地的别名（不上报远端投票） |
| `maimai_update_catalog` | 刷新曲库数据、别名，和/或重绘定数表、牌子表资源 |

### 组合工作流

| 工具 | 说明 |
|------|------|
| `maimai_player_overview` | 玩家概览：B50 图/数据，并可附带上分推荐 JSON（不必当作每条消息的默认动作） |
| `maimai_push_plan` | 上分计划：推荐增益谱面，并为第一条推荐曲出谱面图 |

只需要上分列表时，优先直接调用 `maimai_rise`。

### 用户设定（按 QQ 本地存储）

设定保存在本地 `user.db`。查分时会自动使用该 QQ 已保存的主题与数据源；**默认水鱼**。仅在用户明确要求切换时再改设定。

| 工具 | 说明 |
|------|------|
| `maimai_user_show` | 查看主题、默认查分源、Import-Token 绑定状态（须传 `qq`） |
| `maimai_user_set_theme` | 设置出图主题：`circle`（默认）或 `prism_plus` |
| `maimai_user_set_source` | 设置默认查分源：水鱼或落雪 |
| `maimai_user_bind_lxns` | 获取落雪 OAuth 授权链接，或提交 `code` 完成绑定 |

### 官服成绩上传

| 工具 | 说明 |
|------|------|
| `maimai_user_bind_import_token` | 绑定水鱼 **Import-Token**（个人资料页生成，不是 `DIVINGFISH_TOKEN`） |
| `maimai_update_records` | 机台扫码 → dump → convert → 上传；**`params.source` 必填** `divingfish` \| `lxns` \| `both` |

将机台扫码成绩导入 [水鱼](https://www.diving-fish.com/maimaidx/prober/) / 落雪时：

1. 在水鱼网页「编辑个人资料」生成 **Import-Token**（**不是** 开发者 `DIVINGFISH_TOKEN`）。
2. 绑定到本地 `user.db`：

```bash
maimai user bind-import --qq <QQ> --token <Import-Token>
maimai user show --qq <QQ>   # 可见 import_token 绑定状态（掩码）
```

3. 确保曲库缓存为水鱼 `/music_data`（`static/data/music_data.json`）：

```bash
maimai update music
```

4. 一键：扫码内容 → dump → convert → 上传。**必须声明数据源 `--source`**：

```bash
# 水鱼（需 bind-import）
maimai records update --qq <QQ> --qr-content 'SGWC...' --source divingfish

# 落雪（需先 maimai user bind 落雪 OAuth，scope 含 write_player）
maimai records update --qq <QQ> --qr-content 'SGWC...' --source lxns

# 水鱼 + 落雪
maimai records update --qq <QQ> --qr-content 'SGWC...' --source both
```

MCP：`maimai_update_records` 的 **`params.source` 必填**：`divingfish` | `lxns` | `both`（也可用 `target` 别名）。  
响应含 `source` / `sources` / `source_label`。

### 可选：身份缓存（需 OneBot / NapCat）

常规查分请直接传 `params.qq`，不必依赖昵称反查。配置 HTTP API 后可拉取好友 / 群成员到本地缓存。

| 工具 | 说明 |
|------|------|
| `maimai_refresh_identity` | 通过 OneBot / NapCat 刷新好友与群成员身份缓存 |
| `maimai_identity_status` | 查看身份缓存状态 |
| `maimai_resolve_qq` | 按昵称 / 群名片从缓存反查 QQ |
| `maimai_get_qq_identity` | 按 QQ 读取缓存中的昵称等信息 |

---

## License

- 本仓库代码：**[BSD-2-Clause](LICENSE)**
- 参考自 [maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX) 的部分：**MIT**（见 [LICENSE-UPSTREAM](LICENSE-UPSTREAM)）
- `static` 等美术 / 字体资源**不随包分发**，版权以资源包与官方声明为准，不在本许可范围内

## 致谢

- [Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX) — 绘图与业务参考  
- [Diving-Fish 查分器](https://www.diving-fish.com/maimaidx/prober/)  
- [落雪查分器](https://maimai.lxns.net/)  
- [柚子别名](https://www.yuzuchan.moe/)
