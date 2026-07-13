# maimai MCP 工具一览

参数一般包在 `params` 对象中。查分类工具支持 `qq` / `username`（session 可补全）。

## 会话 / 身份

| 工具 | 作用 |
|------|------|
| `maimai_set_identity` | 设置本进程 `qq`、`username`、可选 `group_id`（仅上下文） |
| `maimai_get_session` | 查看 session |
| `maimai_refresh_identity` | 经 NapCat 拉好友/群 → 身份缓存（需 `NAPCAT_BASE_URL`） |
| `maimai_identity_status` | 缓存路径与统计 |
| `maimai_resolve_qq` | 昵称 / 群名片 → 玩家 QQ |
| `maimai_get_qq_identity` | 按 QQ 读缓存昵称 |

## 用户设置

| 工具 | 作用 |
|------|------|
| `maimai_user_show` | 主题 / 数据源 |
| `maimai_user_set_theme` | `circle` / `prism_plus` |
| `maimai_user_set_source` | 水鱼 / 落雪 |
| `maimai_user_bind_lxns` | 落雪 OAuth |

仅在用户要改设置时调用，不要当作「身份错了」的补救步骤。

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

## 曲库

| 工具 | 作用 |
|------|------|
| `maimai_search` / `maimai_lookup_song` | 搜歌 / 搜+出图 |
| `maimai_chart` / `maimai_ginfo` | 谱面信息 / 全服统计 |
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
| `maimai_fortune` | 今日运势 | 娱乐用 |

用户只要单一能力时优先原子工具（例如只要上分 → `maimai_rise`）。
