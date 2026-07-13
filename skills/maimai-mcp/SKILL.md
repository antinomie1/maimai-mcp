---
name: maimai-mcp
description: >
  通过 maimai-mcp 的 MCP 工具做舞萌 DX 查分/查歌：B50、完成表、牌子、分数列表、
  定数表、上分、谱面、曲库与玩家身份。用户提到 maimai、舞萌、b50、查分、完成表、
  牌子、分列表、定数表、上分、搜歌、谱面、rise，或需要选择 maimai_* 工具 /
  区分 qq 与 group_id 时使用。即使用户只说「帮我查一下」但上下文是舞萌，也用本 skill。
  闲聊、发消息失败、stop、修 bot 等无关话题不要使用。
---

# maimai-mcp 调用

调用任意 `maimai_*` 工具前先遵循本 skill。身份搞错或在无关对话里乱调工具，会浪费落雪配额并可能泄露 QQ/群号。

## 何时调用

**仅当**用户要查舞萌成绩、歌曲、进度或曲库时，才调用 `maimai_*`。

| 用户意图 | 是否调用 MCP |
|----------|--------------|
| b50 / 查分 / 完成表 / 牌子 / 分列表 / 定数表 / 上分 / 搜歌 / 谱面 | 是 |
| 「为什么不发」「stop」、闲聊、编码、修 bot、发消息超时 | 否 |

禁止：

- 每条消息都无条件 `maimai_set_identity`
- 话题无关时改主题 / 数据源 / 绑落雪
- 校验失败后把 `qq` 与 `group_id` 对调碰运气
- 用查分工具「自救」宿主/插件发消息失败

## 对用户回复的隐私

面向用户的最终回复中 **不要出现**：

- QQ 号、群号、`group_id`、会话 ID 中的数字身份
- 工具 JSON 里的 `default_qq` / `qq` / `group_id` 原文
- 含 QQ 的本地路径念读（如 `b50_123456.png`）——只说「已生成图片」或直接发图

**可以：** 曲名、难度、分数、rating、FC/AP、牌子名、「你/该玩家」、图片本身。

报错用人话，不要复述错误 id，例如：「查不到成绩，可能未绑定查分器。」

## 解析查分对象

MCP **不会**自动读聊天上下文。查分类工具必须传 `qq` / `username`，或本进程内已设置 session。

| 用户说法 | 工具参数 |
|----------|----------|
| 我的 / 帮我查 / 未指明对象 | `qq` = 发送者 `user_id`（**不是**群号） |
| @ 某人 / 他的 | `qq` = 被 @ 用户 id |
| 只给水鱼用户名 | `username` |

`group_id` 只作上下文，**绝不**当作查分 `qq`。优先用 OneBot 的 `user_id` / `group_id`，不要把日志里的 `昵称/数字` 默认当成玩家 QQ。

若工具提示「像是群号」：改用正确的玩家 id。不要对调字段，也不要对错误 id 调 `maimai_user_set_*` / 绑定落雪。

### 会话身份（仅在真正查分前）

本 MCP 进程尚无身份、且即将查分时：

```text
maimai_set_identity
  qq = <玩家 id>
  group_id = <当前群号，可选>
```

同进程后续可省略 `qq`。MCP 重启后需重新 set。

可选：昵称反查用 `maimai_resolve_qq`（依赖 `maimai_refresh_identity` + NapCat）。

## 工具选型

出图优先 `format: image`。图片交给宿主发送，文案里不写 QQ。

完整工具表与组合工具说明见 [references/tools.md](references/tools.md)。

常用对照：

| 需求 | 工具 |
|------|------|
| B50 | `maimai_b50` |
| 等级/定数分列表 | `maimai_score_list` |
| 牌子 | `maimai_plate` / `maimai_plate_status` |
| 单曲成绩 | `maimai_minfo` |
| 搜歌 + 谱面 | `maimai_lookup_song`，或 `maimai_search` + `maimai_chart` |
| 仅上分推荐 | `maimai_rise` |
| 上分 + 首选谱面图 | `maimai_push_plan` |
| 定数表 / 等级进度 | `maimai_rating_table` / `maimai_level_progress` |

## 数据源与限流

默认 **水鱼**。落雪需 Token/OAuth，易触发 `lxns_rate_limit`（429）。遇限流时告诉用户稍后再试；仅在用户明确要求时切换数据源。曲库缺谱面可提示 `maimai_update_catalog`，不要念内部 key。

## 回复风格

- 好：「你的 13 分列表好了～」+ 图片
- 坏：贴工具 JSON / 念 QQ；用户问「为什么不发」却去调 `maimai_score_list`

| 情况 | 对用户说法 |
|------|------------|
| 未绑定 / 查无 | 查不到成绩，请确认查分器绑定与隐私设置 |
| 落雪未授权 | 需要先完成落雪授权绑定 |
| 请求过多 | 落雪查分过于频繁，请稍后再试 |
| 群号当 QQ | 查分身份有误，请用玩家本人标识重试 |
