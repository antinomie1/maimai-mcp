# maimai-mcp

本地可用的舞萌 DX **查询库 / CLI / MCP 服务**：查曲、查分、进度与绘图，供命令行或任意 Agent / 宿主调用。

- 仓库：https://github.com/antinomie1/maimai-mcp  
- 绘图与业务参考：[Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX)  

## 功能概览

| 类别 | 能力 |
|------|------|
| 曲库 | 搜歌、别名、谱面信息、随机曲、分数线 |
| 成绩 | 水鱼 / 落雪 B50、单曲成绩、等级/定数分数列表 |
| 进度 | 牌子完成表、定数表、等级进度、上分推荐 |
| 其它 | 排名、今日运势、曲目全服统计（ginfo） |
| 用户设定 | 按 QQ 存主题 / 默认查分器 / 落雪 token（`user.db`） |
| 身份缓存 | 经 **NapCat** HTTP API 拉取好友/群成员 → `identity_cache.json`，昵称反查 QQ |

默认主题 **circle**；需要 `prism_plus` 时再切换。  
**水鱼**为主路径；**落雪**可选（需 Token / OAuth 绑定）。

> **重要：** MCP **不会**自动读取聊天上下文里的 QQ。Agent **每次**查分须在 `params` 中显式传 `qq`（或 `username`）。  
> **禁止把群号填进 `qq`**（例如会话 `GroupMessage:用户QQ_群号` 中，第二段是群号）。

## 目录结构

```text
maimai_mcp/           主包（业务 + CLI + MCP）
  core/               客户端、领域逻辑、绘图、QQ 身份缓存（NapCat 拉取）
  features/           功能拆分（query / draw）
  tools/              MCP 工具注册
skills/               Agent skill（maimai-mcp 调用规则）
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
| `QQ_IDENTITY_CACHE_DIR` | 否 | `./qq-identity-cache` | 身份缓存目录（`identity_cache.json`） |
| `NAPCAT_BASE_URL` | 刷新身份缓存时需要 | — | NapCat HTTP 地址，如 `http://127.0.0.1:3000`（也可用 `ONEBOT_BASE_URL`） |
| `NAPCAT_ACCESS_TOKEN` | 否 | — | NapCat HTTP 鉴权 Token（也可用 `ONEBOT_ACCESS_TOKEN`） |
| `QQ_IDENTITY_GROUP_DELAY_MS` | 否 | `250` | 拉群成员时的组间间隔（毫秒） |

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
# 需要身份缓存（昵称反查 / 防群号当 QQ）时：
# NAPCAT_BASE_URL=http://127.0.0.1:3000
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

参数一般包在 `params` 对象中。查分类工具 **每次**传 `qq` / `username`（无 session 粘性补全）。

| 工具 | 说明 |
|------|------|
| **昵称身份缓存（高级）** | |
| `maimai_refresh_identity` | 经 **NapCat** 拉取好友/群/成员并写入身份缓存 |
| `maimai_identity_status` | 身份缓存路径与统计 |
| `maimai_resolve_qq` | 昵称/群名片反查 QQ（依赖缓存） |
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

### 玩家身份（Agent 必读）

1. **仅当用户意图是舞萌查分/查歌/进度等时** 才调用 `maimai_*`；闲聊、发消息失败、`stop` 等 **不要** 调 MCP。  
2. **每次**在 `params` 中传玩家 `qq`（发送者或被 @）或 `username`；**禁止**把群号填进 `qq`。  
3. 主题 / 默认查分器按该 `qq` 存在 `user.db`，带对 qq 即自动生效；**仅用户明确要求**时再改设定。  
4. 若工具提示像是群号：改用正确玩家 id，**不要**对调字段碰运气，也 **不要** 对错误 id 改主题/数据源。  
5. 会话 `...:<用户QQ>_<群号>`：后半段是群号。宿主日志 `昵称/数字` **勿默认当玩家 QQ**，以 OneBot `user_id` 为准。

完整调用规则见 skill：**[`skills/maimai-mcp/SKILL.md`](skills/maimai-mcp/SKILL.md)**。

### QQ 身份缓存（NapCat 主动拉取）

依赖已启动的 **[NapCat](https://github.com/NapNeko/NapCatQQ)**（或其它兼容 OneBot 11 HTTP 的实现），用于：

- 拉取机器人**好友列表**、**所在群**、**各群成员**（QQ / 昵称 / 群名片）
- 写入本地 `identity_cache.json`
- 供 `maimai_resolve_qq` 按昵称反查，并减少「把群号当成玩家 QQ」的误用

#### 1. NapCat 侧

1. 安装并登录 [NapCat](https://github.com/NapNeko/NapCatQQ)。  
2. 开启 **HTTP 服务**（端口自行配置，下文示例 `3000`）。  
3. 若启用了 Token 鉴权，记下 access token。  
4. 确认本机可访问：`http://127.0.0.1:3000`（或 Docker 内网地址，如 `http://napcat:3000`）。

#### 2. maimai-mcp 配置

```env
# 推荐直接用 NapCat 变量名
NAPCAT_BASE_URL=http://127.0.0.1:3000
# NAPCAT_ACCESS_TOKEN=你的token

# 等价写法（二选一即可）
# ONEBOT_BASE_URL=http://127.0.0.1:3000
# ONEBOT_ACCESS_TOKEN=

# 可选：缓存目录（默认仓库下 qq-identity-cache/）
# QQ_IDENTITY_CACHE_DIR=C:\path\to\qq-identity-cache
# QQ_IDENTITY_GROUP_DELAY_MS=250
```

MCP 客户端 `env` 示例：

```json
"NAPCAT_BASE_URL": "http://127.0.0.1:3000",
"NAPCAT_ACCESS_TOKEN": ""
```

#### 3. 刷新与使用

```text
maimai_refresh_identity
  → NapCat: get_friend_list / get_group_list / get_group_member_list
  → 写入 identity_cache.json

maimai_identity_status    # 查看统计
maimai_resolve_qq         # 昵称 / 群名片 → 玩家 QQ（给工具参数用）
maimai_get_qq_identity    # 按 QQ 读缓存昵称
```

`maimai_refresh_identity` 可选参数：`timeout_ms`、`group_delay_ms`、`max_groups`（测试用）、`base_url`（临时覆盖地址）。

群很多时拉取会较久，属正常现象。对用户回复仍 **不要念出 QQ/群号**（见 [maimai-mcp skill](skills/maimai-mcp/SKILL.md)）。

### 曲库与孤儿成绩

落雪全量成绩中可能含本地曲库没有的谱面。转换时会 **跳过并打日志**，避免整次查询 `KeyError`。请定期：

```bash
# MCP: maimai_update_catalog  what=["music","alias","tables"]
python -m maimai_mcp.cli update music
python -m maimai_mcp.cli update tables
```

## Agent / 宿主集成

1. 配置 MCP：`python -m maimai_mcp`（见上文配置示例）。  
2. 调用规则 skill：[`skills/maimai-mcp/`](skills/maimai-mcp/)（`SKILL.md` + `references/tools.md`，中文）。  
   宿主将 skill 纳入系统提示或 skill 加载路径即可。  
3. **仅在用户明确查舞萌相关内容时** 调用 `maimai_*`；勿在无关对话里乱调查分。  
4. **用户可见文本不得出现 QQ 号 / 群号**；身份数字仅作工具参数。  
5. 更稳妥：宿主在工具参数中注入玩家 `user_id` 作为 `params.qq`，避免模型猜号。

## 数据源说明

| 源 | 用途 |
|----|------|
| Diving-Fish 水鱼 | 默认成绩、曲库、排名 |
| Lxns 落雪 | 可选成绩源、曲库合并、OAuth |
| Yuzu 柚子 | 别名 |
| NapCat | 身份缓存：好友/群/群成员 HTTP 拉取 |

本地用户偏好（主题、数据源、落雪 token）存在 `static/data/user.db`。

## 致谢

- [Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX) — 绘图与业务参考  
- [Diving-Fish 查分器](https://www.diving-fish.com/maimaidx/prober/)  
- [落雪查分器](https://maimai.lxns.net/)  
- [柚子别名](https://www.yuzuchan.moe/)  
- [NapCat](https://github.com/NapNeko/NapCatQQ) — 身份缓存数据源（HTTP）  


## License

见 [LICENSE](LICENSE)（BSD-2-Clause）。静态资源版权以 maimaiDX / 官方素材声明为准。
