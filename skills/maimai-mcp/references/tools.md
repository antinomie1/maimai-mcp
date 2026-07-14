# maimai MCP 工具一览

## 参数约定

- 参数一律包在 **`params`** 中，例如 `params: { qq: 123, format: "image" }`。禁止空 `{}`。
- 查分 / 出图 / 运势：**每次**传玩家 `qq`（默认=**发送者**）或 `username`。
- **@机器人只是唤醒**，`params.qq` 禁止填机器人自身 QQ。
- 出图优先 `format: "image"`；成功返回 `image_path` 后必须用宿主发图，不要只回文字。
- **勿**对出图需求传 `format: "json"`（会跳过绘图，没有 `image_path`）。
- 一次只调一个工具；单一能力优先原子工具。

## 成绩 / 进度

| 工具 | 作用 |
|------|------|
| `maimai_b50` | Best50 / AP50 |
| `maimai_minfo` | 单曲成绩 |
| `maimai_score_list` | 等级或定数分数列表 |
| `maimai_plate` / `maimai_plate_status` | 牌子完成 / 进度 |
| `maimai_rating_table` | 定数表 |
| `maimai_level_progress` | 等级进度 |
| `maimai_rise` | 上分推荐 |
| `maimai_ranking` | 水鱼公开 rating 榜 |
| `maimai_fortune` | 今日运势（娱乐；仍要 `params.qq` 作种子） |

## 曲库

| 工具 | 作用 |
|------|------|
| `maimai_search` / `maimai_lookup_song` | 搜歌 / 搜+出图 |
| `maimai_chart` | 谱面信息图 |
| `maimai_random` / `maimai_mai_what` | 随机 / 带上分偏向 |
| `maimai_score_line` | 分数线容错 |
| `maimai_alias_query` / `maimai_alias_local_add` | 别名 |
| `maimai_update_catalog` | 刷新曲库 / 别名 / 表图 |

## 组合工具（workflow）

| 工具 | 内部串联 | 说明 |
|------|----------|------|
| `maimai_player_overview` | b50 + 可选 rise JSON | 概览，非每条消息默认动作 |
| `maimai_push_plan` | b50(json) + rise + 首条推荐 chart | 上分计划，非默认开场 |
| `maimai_lookup_song` | search + chart（可选 minfo） | 曲名含糊时优先 |
| `maimai_plate_status` | 牌子流程 | 牌子进度 |

仍须在 `params` 中带 `qq` 或 `username`。只要上分 → 直接 `maimai_rise`。

## 用户设置（per-QQ，仅用户明确要求时）

设定按 **玩家 qq** 存在本地库。查分时自动用该用户的主题与数据源：  
**默认水鱼**；用户改过则以 `maimai_user_show` 看到的 `service` 为准。  
**不要**当作「身份错了」的补救；改设定也要传正确 `params.qq`。

| 工具 | 作用 |
|------|------|
| `maimai_user_show` | 查看主题 / 数据源（`service`） |
| `maimai_user_set_theme` | `circle` / `prism_plus` |
| `maimai_user_set_source` | 水鱼 / 落雪（仅用户明确要求） |
| `maimai_user_bind_lxns` | 落雪 OAuth |

## 【高级·默认跳过】昵称缓存

常规查分直接 `params.qq`，无需先反查。

| 工具 | 作用 |
|------|------|
| `maimai_refresh_identity` | NapCat 拉好友/群 → 身份缓存 |
| `maimai_identity_status` | 缓存状态 |
| `maimai_resolve_qq` | 昵称 / 群名片 → QQ |
| `maimai_get_qq_identity` | 按 QQ 读缓存昵称 |
