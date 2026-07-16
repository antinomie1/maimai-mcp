---
name: maimai-mcp
description: >
  通过 maimai-mcp 的 MCP 工具做舞萌 DX 查分/查歌：B50、完成表、牌子、分数列表、
  定数表、上分、谱面、曲库、运势、群 Rating/单曲榜。用户提到 maimai、舞萌、b50、
  查分、完成表、牌子、分列表、定数表、上分、搜歌、谱面、rise、运势、群榜、群排名、
  谁 rating 高、这首歌群里谁强，或需要选择 maimai_* 工具 / 区分「@机器人」与玩家 qq 时使用。
  即使用户只说「帮我查一下」但上下文是舞萌，也用本 skill。
  闲聊、发消息失败、stop、修 bot 等无关话题不要使用。
---

# maimai-mcp 调用

调用任意 `maimai_*` 前先遵循本 skill。把机器人 QQ 当玩家或出图后不发图，是最常见错误。

## 何时调用

**仅当**用户要查舞萌成绩、歌曲、进度、曲库、运势或**群内成绩榜**时，才调用 `maimai_*`。

| 用户意图 | 是否调用 |
|----------|----------|
| b50 / 查分 / 完成表 / 牌子 / 分列表 / 定数表 / 上分 / 搜歌 / 谱面 / 运势 / 今日舞萌 | 是 |
| 群榜 / 群排名 / 群里谁 rating 高 / 这曲群里谁强 / 我在群里第几 | 是（见下文「群榜」） |
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

**一般不传 `source`：** 纯搜歌、别名、分数线、运势种子、水鱼**全国**公开榜 `maimai_ranking`、群榜工具、曲库更新等。

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

**群榜例外：** 榜单类工具用 `params.group_id` 表示群；查「某人名次」时用 `qq` / `target`，仍禁止把群号填进 `qq`。

## 出图必须发出去

查分 / 运势 / 谱面 / 表类：默认或显式 **`format: image`**。  
**勿**为「拿结构」传 `format: json`（会跳过绘图）。

成功且有 **`image_path`** 时：同一轮必须用宿主发图；用户问「为什么不发图」→ 补发上一张，不要重查。

群榜工具**默认出文字榜**，无图时直接回复榜单文案即可。

## 工具选型

完整参数见 [references/tools.md](references/tools.md)。

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
| 水鱼**全国**公开榜 | `maimai_ranking` |
| **本群** Rating 榜 | `maimai_group_rating_rank` |
| **本群** 某曲成绩榜 | `maimai_group_song_rank` |
| 某人在**本群**第几 | `maimai_group_member_rank` |
| 永久切换默认源 | `maimai_user_set_source` |
| 官服上传 | `maimai_update_records`（`source` 必填） |

### 全国榜 vs 群榜（勿混）

| 用户说法 | 工具 | 说明 |
|----------|------|------|
| 查分器总榜 / 全国 ranking / 水鱼排行 | `maimai_ranking` | 公开 API，与群无关 |
| 群里谁 rating 高 / 群榜 / 本群排名 | `maimai_group_rating_rank` | 查榜时按需拉分 + 缓存复用 |
| 这曲群里谁强 / 群内单曲榜 | `maimai_group_song_rank` | 查榜时按需拉该曲 + 缓存复用 |
| 我/某人在群里第几 | `maimai_group_member_rank` | 可带 `song` |

## 群榜（查榜时按需拉分 · 必读）

### 硬性规则

1. **只有用户查群榜时**才去拉成员成绩；禁止在身份刷新、闲聊、其它工具里预拉全群。  
2. 查榜请**直接**调 `maimai_group_*`，**禁止** Agent 自己循环 `maimai_b50` / `maimai_minfo` 拼榜（工具内部已限速拉取）。  
3. 新鲜本地缓存会复用（默认约 24h），重复问榜不会人人重打 API。  
4. 名册来自身份缓存；`group_id` = 群号，**禁止**填进 `qq`。

### 意图 → 工具

| 用户说法 | 工具 | 关键参数 |
|----------|------|----------|
| 群 rating 榜 / 群里谁分高 | `maimai_group_rating_rank` | `group_id`（当前群） |
| 前 10 / 第 5–15 名 | 同上 | `output_limit` 或 `start_rank`+`end_rank` |
| 《xxx》群里谁强 | `maimai_group_song_rank` | `group_id` + `song` |
| 指定难度（Master 等） | 同上 | `level_index`：0–4 |
| 我/某人在群里第几 | `maimai_group_member_rank` | `group_id` + `qq` 或 `target` |
| 某人这曲在群里第几 | 同上 | 再加 `song` |
| 强制刷新 | 对应工具 | `force_refresh: true`（慎用，会重拉） |

### 正确流程

```text
# 直接查榜（工具内部按需拉缺失/过期成绩，有缓存则复用）
工具: maimai_group_rating_rank
参数:
  params:
    group_id: <当前群号>
    output_limit: 20

工具: maimai_group_song_rank
参数:
  params:
    group_id: <当前群号>
    song: <曲名或 id>
```

### 错误示范

| 错误 | 正确 |
|------|------|
| 对每个群成员调 `maimai_b50` 再排序 | 直接 `maimai_group_rating_rank` |
| 用 `maimai_ranking` 回答「群里谁高」 | 用 `maimai_group_*` |
| 把 `group_id` 填进 `qq` | `group_id` 与 `qq` 分字段 |
| 未查榜却批量预拉成绩 | 等用户要榜再调群榜工具 |

### 对人话（群榜）

| 情况 | 对人话 |
|------|--------|
| 榜上有人 | 直接念排名与 rating/达成率，可用群名片称呼 |
| 部分人跳过 | 「有人未绑定查分器或隐私关闭，未计入」 |
| 某人不在榜 | 「TA 可能没绑定查分器、隐私关闭或没打过这曲」 |
| 身份缓存无成员 | 「群成员名单还没准备好，稍后再试」（勿念工具名） |
| 拉取较慢 | 可简短说明在汇总中，勿暴露并发/缓存参数 |

## 对用户回复（必读）

**禁止向用户暴露内部调用细节。** 工具名、参数、`params`、JSON、返回字段、路径、耗时、错误码等只用于你侧决策，不要写进对用户的回复。

**不要出现：**

- 工具名：`maimai_b50`、`maimai_*`、`send_message_to_user` 等  
- 参数与结构：`params`、`source`、`format`、`image_path`、`ok`、`code`、完整/片段工具 JSON  
- 内部路径与标识：本地文件路径、`user.db`、`player-cache`、`site-packages`、HTTP 状态码、stacktrace  
- QQ、群号、机器人号、含 QQ 的路径（对用户可用群名片/「你」指代）  
- 「我调用了…」「参数是…」「工具返回…」类过程说明  

**可以：** 曲名、难度、分数、rating、FC/AP、牌子名、「你」、名次、**图片本身**；失败时用人话简短说明。

出图成功时：发图 + 极短附和即可，不要复述内部文案里的路径或「数据源：…」技术字段（用户若主动问用水鱼还是落雪，可用「水鱼/落雪」口语回答）。

| 情况 | 对人话 |
|------|--------|
| 未绑定 / 查无 | 查不到成绩，请确认查分器绑定与隐私设置 |
| 落雪未授权 | 需要先完成落雪授权绑定 |

## 附录【高级·默认跳过】

### 昵称反查

仅当用户**没给 QQ**、只用昵称/群名片指人时：`maimai_resolve_qq`（需已有身份缓存）。  
常规查分优先直接传 `qq`。

### 身份缓存维护（运维向）

`maimai_refresh_identity` / `maimai_identity_status` / `maimai_get_qq_identity`：给机器人维护名单用，**不是**查分或刷榜手段。用户日常查群榜时不要每次先全量 refresh。
