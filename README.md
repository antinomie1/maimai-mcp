# maimai-mcp

本地可用的舞萌 DX **查询库 / CLI / MCP 服务**：查曲、查分、进度与绘图，供命令行或 Agent（如 AstrBot）调用。

- 仓库：https://github.com/antinomie1/maimai-mcp  
- 绘图与业务参考：[Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX)  

## 功能概览

| 类别 | 能力 |
|------|------|
| 曲库 | 搜歌、别名、谱面信息、随机曲、分数线 |
| 成绩 | 水鱼 / 落雪 B50、单曲成绩、等级/定数分数列表 |
| 进度 | 牌子完成表、定数表、等级进度、上分推荐 |
| 其它 | 排名、今日运势、曲目全服统计（ginfo） |
| 会话 | `maimai_set_identity`（玩家 QQ + 群号分离）、session 粘性 |
| 身份缓存 | 可选只读 `identity_cache.json`，昵称反查 QQ |

默认主题 **circle**；需要 `prism_plus` 时再切换。  
**水鱼**为主路径；**落雪**可选（需 Token / OAuth 绑定）。

> **重要：** MCP **不会**自动读取聊天上下文里的 QQ。Agent 必须显式传 `qq`，或先 `maimai_set_identity`。  
> **禁止把群号填进 `qq`**（例如会话 `GroupMessage:用户QQ_群号` 中，第二段是群号）。

## 目录结构

```text
maimai_mcp/           主包（业务 + CLI + MCP）
  core/               客户端、领域逻辑、绘图、QQ 身份缓存只读
  features/           功能拆分（query / draw）
  tools/              MCP 工具注册
docs/                 AstrBot / Agent 提示词等
scripts/              Inspector、冒烟脚本
static/               曲库 JSON、字体、封面等资源（自备）
output/               默认出图目录
mcp.example.json      MCP 客户端配置示例
```

## 安装

```bash
git clone https://github.com/antinomie1/maimai-mcp.git
cd maimai-mcp
pip install -e .
# ginfo 饼图需要：
# playwright install chromium

cp maimai_mcp/.env.example maimai_mcp/.env
# 编辑 maimai_mcp/.env，至少填写 MAIMAIDX_PATH
```

要求 **Python ≥ 3.10**。

### 静态资源

与 maimaiDX 相同资源包，解压后将 `MAIMAIDX_PATH` 指向其中的 `static` 目录：

- [Cloudreve](https://cloud.yuzuchan.moe/f/34s7/Resource%20CN1.55.7z)
- [OneDrive](https://yuzuai-my.sharepoint.com/:u:/g/personal/yuzu_yuzuchan_moe/IQBGKHie6MAaTZy3rME7Q-ruAVKgXDCKROqz5e25KtMeeVY?e=53eC6a)

请遵守上游美术相关声明。首次使用表类指令前建议生成表图：

```bash
python -m maimai_mcp.cli update tables
```

## 环境变量

配置从 `maimai_mcp/.env` 读取（`pydantic-settings`），也可在 MCP 客户端 `env` 中注入。  
完整注释见 [`maimai_mcp/.env.example`](maimai_mcp/.env.example)。**不要提交含 Token 的 `.env` / `mcp.json`。**

### 路径与资源

| 变量 | 必填 | 默认 | 说明 |
|------|------|------|------|
| `MAIMAIDX_PATH` | **是** | — | `static` 资源目录的**绝对路径** |
| `MAIMAIDX_ALIAS_PROXY` | 否 | `false` | `true` 时别名走国内域名 |
| `SAVE_IN_MEMORY` | 否 | `true` | 预载主题贴图（更快、更吃内存） |
| `ASSETS_ONLINE` | 否 | `true` | 封面等缺失时是否允许在线补全 |
| `BOT_NAME` | 否 | `maimai` | 展示名（运势、落雪授权提示） |
| `DEFAULT_QQ` | 否 | — | 未传 QQ 时的默认玩家 QQ |
| `DEFAULT_USERNAME` | 否 | — | 默认水鱼用户名 |
| `OUTPUT_DIR` | 否 | 仓库 `output/` | 出图默认目录 |
| `QQ_IDENTITY_CACHE_DIR` | 否 | `./qq-identity-cache` | 可选身份缓存目录（内含 `identity_cache.json`） |

### 水鱼 Diving-Fish

| 变量 | 说明 |
|------|------|
| `DIVINGFISH_TOKEN` | 开发者 Token（强烈建议；无则查分可能仅限 b50） |
| `DIVINGFISH_PROBER_PROXY` | 是否走代理访问水鱼 |

申请：[水鱼查分器](https://www.diving-fish.com/maimaidx/prober/)

### 落雪 Lxns（可选）

| 变量 | 说明 |
|------|------|
| `LXNS_DEV_TOKEN` | 开发者 Token；曲库合并 / 落雪数据源依赖 |
| `LX_CLIENT_ID` / `LX_CLIENT_SECRET` | OAuth 绑定 |
| `REDIRECT_URI` | 本机常用 `urn:ietf:wg:oauth:2.0:oob` |

### 最小示例

```env
MAIMAIDX_PATH=C:\path\to\maimai-mcp\static
DIVINGFISH_TOKEN=你的水鱼Token
OUTPUT_DIR=C:\path\to\maimai-mcp\output
```

## CLI

```bash
# 未 pip install -e 时：set PYTHONPATH=.
python -m maimai_mcp.cli b50 --username <水鱼用户名> --out out/b50.png
python -m maimai_mcp.cli b50 --qq <QQ>
python -m maimai_mcp.cli chart 834
python -m maimai_mcp.cli search --mode 定数 14.0 --format json
python -m maimai_mcp.cli plate --ver 檄 --plan 极 --mode progress --qq <QQ>
python -m maimai_mcp.cli user theme prism_plus --qq <QQ>
python -m maimai_mcp.cli update music
python -m maimai_mcp.cli update alias
python -m maimai_mcp.cli update tables
```

入口脚本（install 后）：`maimai`、`maimai-mcp`。

## MCP

### 启动

```bash
python -m maimai_mcp
```

### 客户端配置

复制并修改 [`mcp.example.json`](mcp.example.json)（路径改成你的机器）。密钥优先写在 `maimai_mcp/.env`：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "python",
      "args": ["-m", "maimai_mcp"],
      "cwd": "/path/to/maimai-mcp",
      "env": {
        "PYTHONPATH": "/path/to/maimai-mcp",
        "MAIMAIDX_PATH": "/path/to/maimai-mcp/static",
        "OUTPUT_DIR": "/path/to/maimai-mcp/output"
      }
    }
  }
}
```

本地私有配置可用 `mcp.json`（已 gitignore）。

### 联调

```bash
./scripts/run_inspector.ps1   # 或 run_inspector.sh
python scripts/smoke_mcp_tools.py --username <水鱼用户名>
```

### 主要 Tools

参数一般包在 `params` 对象中。查分类工具支持 `qq` / `username`（session 可补全）。

| 工具 | 说明 |
|------|------|
| **会话 / 身份** | |
| `maimai_set_identity` | 设置本进程 `qq`、`username`、**`group_id`（仅上下文，不作查分 QQ）** |
| `maimai_get_session` | 查看 session（含 `group_id`） |
| `maimai_resolve_qq` | 昵称/群名片反查 QQ（需身份缓存） |
| `maimai_get_qq_identity` | 按 QQ 读缓存中的昵称信息 |
| **用户设置** | |
| `maimai_user_show` | 主题 / 数据源 |
| `maimai_user_set_theme` | `circle` / `prism_plus` |
| `maimai_user_set_source` | 水鱼 / 落雪 |
| `maimai_user_bind_lxns` | 落雪 OAuth |
| **成绩 / 进度** | |
| `maimai_b50` | Best50 / AP50 |
| `maimai_minfo` | 单曲成绩 |
| `maimai_score_list` | 等级或定数分数列表 |
| `maimai_plate` / `maimai_plate_status` | 牌子完成表 / 进度 |
| `maimai_rating_table` | 定数表 |
| `maimai_level_progress` | 等级进度 |
| `maimai_rise` | 上分推荐 |
| `maimai_ranking` | 水鱼公开 rating 榜 |
| **曲库** | |
| `maimai_search` / `maimai_lookup_song` | 搜歌 / 搜+出图 |
| `maimai_chart` / `maimai_ginfo` | 谱面信息 / 全服统计 |
| `maimai_random` / `maimai_mai_what` | 随机 / 带上分偏向 |
| `maimai_score_line` | 分数线容错 |
| `maimai_alias_query` / `maimai_alias_local_add` | 别名 |
| `maimai_update_catalog` | 刷新曲库 / 别名 / 表图 |
| **组合** | |
| `maimai_player_overview` | B50 + 可选上分 |
| `maimai_push_plan` | 上分计划串联 |
| `maimai_fortune` | 今日运势 |

### 会话与身份（Agent 必读）

1. **玩家 QQ** 与 **群号** 分离：  
   - `qq` = 发送者或被查对象  
   - `group_id` = 当前群（可选，**绝不**当查分 QQ）
2. 推荐对话开始时：

   ```json
   {
     "params": {
       "qq": "<发送者QQ>",
       "group_id": "<当前群号>"
     }
   }
   ```

   之后同 MCP 进程内可省略 `qq`（粘性 session）。**重启 MCP 后需重新 set。**
3. 若 `qq == group_id`，或可选缓存里该 id 仅是「群」不是「用户」，工具会 **校验失败** 并提示。
4. 聊天前缀 `昵称/<用户QQ>`、会话 `...:<用户QQ>_<群号>` 中：  
   **前者是用户 QQ，后者后半段是群号。**

完整提示词片段见：**[docs/ASTRBOT_AGENT_PROMPT.md](docs/ASTRBOT_AGENT_PROMPT.md)**（请粘贴到 AstrBot / Agent 系统提示）。

### 可选：QQ 身份缓存

本仓库 **不**主动拉取好友/群成员。若本机已有 `identity_cache.json`（结构含 `users` / `groups`）：

```bash
# 指向含 identity_cache.json 的目录
export QQ_IDENTITY_CACHE_DIR=/path/to/qq-identity-cache
```

即可使用 `maimai_resolve_qq` / `maimai_get_qq_identity`，并加强「群号误当 QQ」检测。

### 曲库与孤儿成绩

落雪全量成绩中可能含本地曲库没有的谱面。转换时会 **跳过并打日志**，避免整次查询 `KeyError`。请定期：

```bash
# MCP: maimai_update_catalog  what=["music","alias","tables"]
python -m maimai_mcp.cli update music
python -m maimai_mcp.cli update tables
```

## AstrBot 部署提示

1. 配置 MCP 指向本仓库 `python -m maimai_mcp`。  
2. 系统提示加入 [docs/ASTRBOT_AGENT_PROMPT.md](docs/ASTRBOT_AGENT_PROMPT.md)。  
3. 宿主插件钩子（如 `QQToolsPlugin`）须与当前 AstrBot 签名一致，否则会出现  
   `takes N positional arguments but M were given`。  
4. 更稳妥：在 `on_llm_request` 注入 `sender_id` / `group_id`，或由插件调用 `maimai_set_identity`，不要让模型从自然语言猜号。

## 数据源说明

| 源 | 用途 |
|----|------|
| Diving-Fish 水鱼 | 默认成绩、曲库、排名 |
| Lxns 落雪 | 可选成绩源、曲库合并、OAuth |
| Yuzu 柚子 | 别名 |

本地用户偏好（主题、数据源、落雪 token）存在 `static/data/user.db`。

## 致谢

- [Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX) — 绘图与业务参考  
- [Diving-Fish 查分器](https://www.diving-fish.com/maimaidx/prober/)  
- [落雪查分器](https://maimai.lxns.net/)  
- [柚子别名](https://www.yuzuchan.moe/)  

## License

见 [LICENSE](LICENSE)（BSD-2-Clause）。静态资源版权以 maimaiDX / 官方素材声明为准。
