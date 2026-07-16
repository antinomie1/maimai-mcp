# maimai MCP 工具一览

## 参数约定

- 参数一律包在 **`params`** 中。禁止空 `{}`。
- 查分 / 出图：**每次**传玩家 `qq`（默认=**发送者**）或 `username`。
- **点名数据源**（水鱼/落雪等任意说法）时：成绩查询工具 **必须** 传 **`params.source`**（`divingfish` 或 `lxns`）。仅本次，不写库。
- **@机器人只是唤醒**，`params.qq` 禁止填机器人自身 QQ；**群号禁止填进 `qq`**。
- 出图优先 `format: "image"`；成功返回 `image_path` 后必须用宿主发图。
- 一次只调一个工具。

---

## 成绩 / 进度（支持 `source`）

| 工具 | 作用 |
|------|------|
| `maimai_b50` | Best50 / AP50；点名源时加 `source`。成功会旁路写入 Rating 缓存（群榜可复用） |
| `maimai_minfo` | 单曲成绩。成功会旁路写入该曲缓存（群榜可复用） |
| `maimai_score_list` | 等级或定数分数列表 |
| `maimai_plate` / `maimai_plate_status` | 牌子完成 / 进度 |
| `maimai_rating_table` | 定数表（个人进度时依赖源） |
| `maimai_level_progress` | 等级进度 |
| `maimai_rise` | 上分推荐 |
| `maimai_fortune` | 今日运势（娱乐；`source` 可有可无） |

### 公开榜（与「本群」无关）

| 工具 | 作用 |
|------|------|
| `maimai_ranking` | 水鱼**全国/查分器公开** rating 榜（按用户名或 `my=true`） |

用户说「群里谁高」时**不要**用本工具，改用下方群榜。

---

## 曲库 / 谱面

| 工具 | 作用 |
|------|------|
| `maimai_search` | 搜歌（一般不需 source） |
| `maimai_lookup_song` | 搜+出图；with_minfo 时要 `source` |
| `maimai_chart` | 谱面信息图（含个人上下文时 `source`） |
| `maimai_random` / `maimai_mai_what` | 随机 / 带上分偏向（rise 时 `source`） |
| `maimai_score_line` | 分数线容错 |
| `maimai_alias_query` / `maimai_alias_local_add` | 别名 |
| `maimai_update_catalog` | 刷新曲库 / 别名 / 表图 |

---

## 组合工具（workflow）

| 工具 | 内部串联 | 说明 |
|------|----------|------|
| `maimai_player_overview` | b50 + 可选 rise | 传 `source` 会下传到子查询 |
| `maimai_push_plan` | b50 + rise + chart | 同上 |
| `maimai_lookup_song` | search + chart（可选 minfo） | 同上 |
| `maimai_plate_status` | 牌子流程 | 同上 |

---

## 用户设置（per-QQ）

| 工具 | 作用 |
|------|------|
| `maimai_user_show` | 主题 / 默认数据源 / Import-Token 绑定状态 |
| `maimai_user_set_theme` | `circle` / `prism_plus` |
| `maimai_user_set_source` | **永久**默认源：水鱼 / 落雪（仅用户明确要求） |
| `maimai_user_bind_lxns` | 落雪 OAuth |
| `maimai_user_bind_import_token` | 水鱼 Import-Token |

---

## 成绩上传

| 工具 | 作用 |
|------|------|
| `maimai_update_records` | 官服扫码上传；**`source` 必填** `divingfish` \| `lxns` \| `both` |

---

## 群榜（查榜时按需拉分 · 限速复用缓存）

**设计：**  
- 名册 ← 身份缓存  
- 成绩 ← **本次查榜时**对缺失/过期成员拉查分器；新鲜 `player-cache` 复用  
- 个人 `b50`/`minfo` 仍会旁路写缓存（可选加速，不是进榜前提）  
- **禁止** Agent 自己循环 `b50`/`minfo` 拼榜  

### 工具

| 工具 | 作用 | 主要参数 |
|------|------|----------|
| `maimai_group_rating_rank` | 本群 Rating 榜 | `group_id`；limit/名次窗口；`force_refresh`；`max_concurrency`；`query_delay_ms` |
| `maimai_group_song_rank` | 本群单曲榜 | `group_id` + `song`；`level_index`；`sort_by`；限速参数同上 |
| `maimai_group_member_rank` | 某人在群内名次 | `qq`/`target`；可选 `song`（内部会建整榜） |
| `maimai_player_cache_status` | 成绩缓存统计 | 无 |

### 参数要点

| 参数 | 说明 |
|------|------|
| `group_id` | **QQ 群号**，不是玩家 QQ |
| `song` | 曲 id / 标题 / 别名 |
| `level_index` | `0`–`4`；省略则用结果里最高难度 |
| `force_refresh` | `true` 忽略缓存重拉（慎用） |
| `max_concurrency` | 并发，默认 3 |
| `query_delay_ms` | 成员启动间隔，默认 250 |
| `max_members` | 最多处理人数（测试/限流） |

### Agent 流程

1. 用户要群榜 → **只**调一次 `maimai_group_*`。  
2. 不要先 `b50` 循环再排序。  
3. 名单没有 → 运维向再 `maimai_refresh_identity`（不查成绩）。  

### 反模式

| 不要 | 要 |
|------|-----|
| 群成员循环 `maimai_b50` | `maimai_group_rating_rank` |
| `maimai_ranking` 当群榜 | `maimai_group_rating_rank` |
| `qq` = 群号 | `group_id` = 群号 |

---

## 【高级·默认跳过】昵称 / 身份缓存

常规查分请直接传 `qq`。以下用于昵称反查与群名册。

| 工具 | 作用 |
|------|------|
| `maimai_refresh_identity` | 经 OneBot/NapCat 拉取好友与群成员（写身份缓存，**不查成绩**） |
| `maimai_identity_status` | 身份缓存路径与统计 |
| `maimai_resolve_qq` | 昵称 / 群名片 → QQ |
| `maimai_get_qq_identity` | 按 QQ 读缓存中的昵称等信息 |

依赖环境：`NAPCAT_BASE_URL`（或 `ONEBOT_BASE_URL`）、可选 `QQ_IDENTITY_CACHE_DIR`。

**不要**把「刷新身份」当成「刷新全群成绩」。
