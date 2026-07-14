# maimai-mcp

[![PyPI version](https://badge.fury.io/py/maimai-mcp.svg)](https://badge.fury.io/py/maimai-mcp)
[![Python versions](https://img.shields.io/pypi/pyversions/maimai-mcp.svg)](https://pypi.org/project/maimai-mcp/)

本地可用的舞萌 DX 查询库 / CLI / MCP 服务：查曲、查分、进度与绘图，供命令行或任意 Agent / 宿主调用。

- 仓库：https://github.com/antinomie1/maimai-mcp  
- 绘图与业务参考：[Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX)

Python ≥ 3.10。成绩默认走[水鱼查分器](https://www.diving-fish.com/maimaidx/prober/)，也可切换到[落雪](https://maimai.lxns.net/)（需 Token / OAuth 绑定）。出图默认主题为 **circle**，可按 QQ 改为 **prism_plus**。

CLI 命令（`maimai …`）与 MCP 工具（`maimai_*`）能力一一对应；Agent 侧参数须包在 `params` 中，查分时每次显式传入玩家 `qq` 或 `username`（MCP 不会自动读聊天上下文）。

---

## 部署（pip，推荐）

### 1. 安装包

```bash
pip install maimai-mcp
```

安装完成后提供两个入口：

- `maimai-mcp`：stdio MCP 服务  
- `maimai`：命令行  

也可用模块方式启动：`python -m maimai_mcp` / `python -m maimai_mcp.cli`。

### 2. 下载静态资源

封面、底图、字体等资源**不随 PyPI 包分发**。请下载并解压资源包，记下其中 **`static` 目录的绝对路径**：

- [Cloudreve](https://cloud.yuzuchan.moe/f/34s7/Resource%20CN1.55.7z)
- [OneDrive](https://yuzuai-my.sharepoint.com/:u:/g/personal/yuzu_yuzuchan_moe/IQBGKHie6MAaTZy3rME7Q-ruAVKgXDCKROqz5e25KtMeeVY?e=53eC6a)

请遵守上游美术与字体相关声明。

### 3. 配置环境变量

| 变量 | 必填 | 说明 |
|------|------|------|
| `MAIMAIDX_PATH` | **是** | 上一步 `static` 的绝对路径 |
| `DIVINGFISH_TOKEN` | 建议 | [水鱼开发者 Token](https://www.diving-fish.com/maimaidx/prober/)，提高接口配额与稳定性 |
| `OUTPUT_DIR` | 否 | 出图保存目录；不设则使用包内默认位置 |

完整变量说明见 [`maimai_mcp/.env.example`](maimai_mcp/.env.example)。  
通过 pip 安装时，请把变量写在 **MCP 客户端的 `env` 块**或系统环境中，不要改 site-packages 里的文件。

### 4. 接入 MCP 客户端

在客户端（Cursor、Claude Desktop、Grok 等）的 MCP 配置中加入类似：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "maimai-mcp",
      "env": {
        "MAIMAIDX_PATH": "/path/to/static",
        "OUTPUT_DIR": "/path/to/output",
        "DIVINGFISH_TOKEN": ""
      }
    }
  }
}
```

- 路径请换成真实位置；Windows 示例：`"C:\\\\path\\\\to\\\\static"`。  
- 若 PATH 中找不到 `maimai-mcp`，可改为 `"command": "python", "args": ["-m", "maimai_mcp"]`。  
- 更完整的字段示例见 [`mcp.example.json`](mcp.example.json)。  
- 面向 Agent 的调用约定与避坑说明见 skill：[`skills/maimai-mcp/`](skills/maimai-mcp/)。

启动成功后应能列出 `maimai_*` 工具；需要成绩时请传入玩家 `qq` 或水鱼 `username`。

### 5. 初始化与自检

表类功能（定数表、牌子表等）首次使用前建议先刷新资源，再用一两项查询确认环境正常：

```bash
maimai update tables
maimai b50 --qq <QQ>
maimai chart 834
```

---

## 部署（源码）

适合二次开发或希望跟踪仓库最新改动的场景。

### 1. 克隆并安装

```bash
git clone https://github.com/antinomie1/maimai-mcp.git
cd maimai-mcp
pip install -e .
```

### 2. 下载静态资源

与 [pip 部署第 2 步](#2-下载静态资源) 相同。可将资源放在仓库旁，或直接使用资源包内的 `static` 路径。

### 3. 写配置

```bash
cp maimai_mcp/.env.example maimai_mcp/.env
# 编辑 .env：至少设置 MAIMAIDX_PATH，建议填写 DIVINGFISH_TOKEN
```

也可以不写 `.env`，仅在 MCP 客户端的 `env` 中注入变量（字段含义见 [`.env.example`](maimai_mcp/.env.example)）。

### 4. 接入 MCP 客户端

源码布局下推荐用 `python -m maimai_mcp`，并指定仓库为工作目录：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "python",
      "args": ["-m", "maimai_mcp"],
      "cwd": "/path/to/maimai-mcp",
      "env": {
        "MAIMAIDX_PATH": "/path/to/static",
        "OUTPUT_DIR": "/path/to/output",
        "DIVINGFISH_TOKEN": ""
      }
    }
  }
}
```

Agent skill 仍在 [`skills/maimai-mcp/`](skills/maimai-mcp/)。本地联调可参考 `scripts/run_inspector.ps1` 与 `scripts/smoke_mcp_tools.py`。

### 5. 初始化与自检

```bash
maimai update tables
maimai b50 --qq <QQ>
# 或：python -m maimai_mcp.cli b50 --qq <QQ>
```

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
| `maimai_user_show` | 查看指定 QQ 的主题与查分源（`service`） |
| `maimai_user_set_theme` | 设置出图主题：`circle`（默认）或 `prism_plus` |
| `maimai_user_set_source` | 设置默认查分源：水鱼或落雪 |
| `maimai_user_bind_lxns` | 获取落雪 OAuth 授权链接，或提交 `code` 完成绑定 |

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
- 参考自 [maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX) 的部分：**MIT**（见 LICENSE 附录）
- `static` 等美术 / 字体资源**不随包分发**，版权以资源包与官方声明为准，不在本许可范围内

## 致谢

- [Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX) — 绘图与业务参考  
- [Diving-Fish 查分器](https://www.diving-fish.com/maimaidx/prober/)  
- [落雪查分器](https://maimai.lxns.net/)  
- [柚子别名](https://www.yuzuchan.moe/)
