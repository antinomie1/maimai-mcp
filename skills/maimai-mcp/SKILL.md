---
name: maimai-mcp
description: >
  通过 maimai-mcp 的 MCP 工具做舞萌 DX 查分/查歌：B50、完成表、牌子、分数列表、
  定数表、上分、谱面、曲库、运势。用户提到 maimai、舞萌、b50、查分、完成表、
  牌子、分列表、定数表、上分、搜歌、谱面、rise、运势，或需要选择 maimai_* 工具 /
  区分「@机器人」与玩家 qq 时使用。即使用户只说「帮我查一下」但上下文是舞萌，也用本 skill。
  闲聊、发消息失败、stop、修 bot 等无关话题不要使用。
---

# maimai-mcp 调用

调用任意 `maimai_*` 前先遵循本 skill。把机器人 QQ 当玩家或出图后不发图，是最常见错误。

## 何时调用

**仅当**用户要查舞萌成绩、歌曲、进度、曲库或运势时，才调用 `maimai_*`。

| 用户意图 | 是否调用 |
|----------|----------|
| b50 / 查分 / 完成表 / 牌子 / 分列表 / 定数表 / 上分 / 搜歌 / 谱面 / 运势 / 今日舞萌 | 是 |
| 上传成绩 / 导分 / 官服扫码 / Import-Token 绑定 | 是（仅用户明确要求） |
| stop、闲聊、编码、修 bot、发消息超时 | 否 |
| 「为什么不发图」且上一轮已有 `image_path` | 否（直接补发该图片，不要重查） |

禁止：每条消息改设定；用查分「自救」发消息失败；校验失败后对调 `qq` 与群号。

## 调用格式（必读）

MCP **不会**读聊天上下文。参数必须包在 **`params` 对象**里；**一次只调一个** `maimai_*`。

硬性禁止：

- 空参数 `{}` 或漏掉 `params`
- 并行多个 `maimai_*`
- 用机器人自身 QQ 当 `params.qq`
- 出图成功后只回文字、不发图
- **对用户回复里写内部调用细节**（工具名、params、JSON、路径、错误码等）

```text
工具: maimai_b50
参数:
  params:
    qq: <发送者 user_id>
    format: image
```

## 数据源（全部成绩查询类 · 必读）

服务端**不做**用户原话文本识别。是否带 `params.source` **由你根据用户意图填写**。

### 哪些工具支持 `params.source`（一次查询覆盖，不写 user.db）

凡带玩家身份、会拉**个人成绩**的查询：

| 工具 | 说明 |
|------|------|
| `maimai_b50` | B50 / AP50 |
| `maimai_minfo` | 单曲成绩 |
| `maimai_score_list` | 等级/定数分数列表 |
| `maimai_plate` / `maimai_plate_status` | 牌子完成/进度 |
| `maimai_rating_table` | 定数表（含个人进度时） |
| `maimai_level_progress` | 等级进度 |
| `maimai_rise` | 上分推荐 |
| `maimai_player_overview` | b50+rise |
| `maimai_push_plan` | 上分计划 |
| `maimai_lookup_song` | 搜歌+谱面（含 with_minfo 时） |
| `maimai_chart` | 谱面信息（含个人推分上下文时） |
| `maimai_mai_what` | 随机/上分偏向（rise 时） |

**不依赖个人成绩源**（一般不用 `source`）：纯搜歌 `maimai_search`、别名、分数线、运势种子、水鱼公开榜 `maimai_ranking`、曲库更新等。

### 规则

| 用户意图 | 动作 |
|----------|------|
| **任意说法**表达「这次用水鱼 / Diving-Fish / df / 不要落雪」 | 查分工具 **必须** `params.source: divingfish`（或 `水鱼`） |
| **任意说法**表达「这次用落雪 / Lxns」 | **必须** `params.source: lxns`（或 `落雪`） |
| 只说 b50 / 查分 / 牌子，**未**点名查分器 | **不要**传 `source`，用 QQ 本地默认 |
| 「切换数据源 / 默认改成水鱼或落雪 / 以后都用…」 | 才用 `maimai_user_set_source` |
| 一句话里点名了源 + 查询 | **只**查分工具带 `source`，**禁止**先 set_source 再查 |

```text
# 「水鱼数据源 b50」「用水鱼看下分列表」「落雪的牌子进度」…
工具: maimai_b50   # 或 score_list / plate / rise …
参数:
  params:
    qq: <发送者>
    format: image
    source: divingfish   # 或 lxns
```

**禁止**：用户已点名数据源，却省略 `source`（会落到 user.db 默认，等于无视用户）。  
**禁止**：把「水鱼 b50」做成 `maimai_user_set_source` 再 b50。

### 上传成绩（另一套，source 必填）

`maimai_update_records`：`params.source` **必填** `divingfish` | `lxns` | `both`。

## 查谁（唯一路径）

群聊里 **@机器人只是唤醒**，不是查分对象。

| 用户说法 | `params.qq` |
|----------|-------------|
| 我的 / 帮我查 / 查我 / 未指明对象 / 只 @ 了机器人 | **发送者** `user_id` |
| 明确 @ **另一个玩家** 查他的 | 被 @ 的**玩家** id |
| 只给水鱼用户名 | 用 `username`，可不写 qq |

```text
默认 = 发送者 user_id
@机器人 / 会话里的 bot 号 = 忽略，不当 qq
群号 = 绝不进 qq
```

错误示例：用户 `@机器人 b50` → 传 `qq=机器人号`。正确：传发送者号。

## 出图必须发出去

查分 / 运势 / 谱面 / 表类：默认或显式 **`format: image`**。  
**勿**为「拿结构」传 `format: json`（会跳过绘图）。

成功且有 **`image_path`** 时：同一轮必须用宿主发图；用户问「为什么不发图」→ 补发上一张，不要重查。

## 工具选型

完整表见 [references/tools.md](references/tools.md)。

| 需求 | 工具 |
|------|------|
| B50 | `maimai_b50` |
| 运势 | `maimai_fortune` |
| 分列表 | `maimai_score_list` |
| 牌子 | `maimai_plate` / `maimai_plate_status` |
| 单曲成绩 | `maimai_minfo` |
| 搜歌+谱面 | `maimai_lookup_song` |
| 上分 | `maimai_rise` / `maimai_push_plan` |
| 定数表 / 等级进度 | `maimai_rating_table` / `maimai_level_progress` |
| 永久切换默认源 | `maimai_user_set_source` |
| 官服上传 | `maimai_update_records`（`source` 必填） |

## 对用户回复（必读）

**禁止向用户暴露内部调用细节。** 工具名、参数、`params`、JSON、返回字段、路径、耗时、错误码等只用于你侧决策，不要写进对用户的回复。

**不要出现：**

- 工具名：`maimai_b50`、`maimai_*`、`send_message_to_user` 等  
- 参数与结构：`params`、`source`、`format`、`image_path`、`ok`、`code`、完整/片段工具 JSON  
- 内部路径与标识：本地文件路径、`user.db`、`site-packages`、HTTP 状态码、stacktrace  
- QQ、群号、机器人号、含 QQ 的路径  
- 「我调用了…」「参数是…」「工具返回…」类过程说明  

**可以：** 曲名、难度、分数、rating、FC/AP、牌子名、「你」、**图片本身**；失败时用人话简短说明（如「查不到成绩，请确认查分器绑定」）。

出图成功时：发图 + 极短附和即可，不要复述内部文案里的路径或「数据源：…」技术字段（用户若主动问用水鱼还是落雪，可用「水鱼/落雪」口语回答）。

| 情况 | 对人话 |
|------|--------|
| 未绑定 / 查无 | 查不到成绩，请确认查分器绑定与隐私设置 |
| 落雪未授权 | 需要先完成落雪授权绑定 |

## 附录【高级·默认跳过】

昵称反查：`maimai_resolve_qq`（需 `maimai_refresh_identity` + NapCat）。
