# maimai MCP 工具一览

## 参数约定

- 参数一律包在 **`params`** 中。禁止空 `{}`。
- 查分 / 出图：**每次**传玩家 `qq`（默认=**发送者**）或 `username`。
- **点名数据源**（水鱼/落雪等任意说法）时：成绩查询工具 **必须** 传 **`params.source`**（`divingfish` 或 `lxns`）。仅本次，不写库。
- **@机器人只是唤醒**，`params.qq` 禁止填机器人自身 QQ。
- 出图优先 `format: "image"`；成功返回 `image_path` 后必须用宿主发图。
- 一次只调一个工具。

## 成绩 / 进度（支持 `source`）

| 工具 | 作用 |
|------|------|
| `maimai_b50` | Best50 / AP50；点名源时加 `source` |
| `maimai_minfo` | 单曲成绩 |
| `maimai_score_list` | 等级或定数分数列表 |
| `maimai_plate` / `maimai_plate_status` | 牌子完成 / 进度 |
| `maimai_rating_table` | 定数表（个人进度时依赖源） |
| `maimai_level_progress` | 等级进度 |
| `maimai_rise` | 上分推荐 |
| `maimai_fortune` | 今日运势（娱乐；`source` 可有可无） |

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

## 组合工具（workflow）

| 工具 | 内部串联 | 说明 |
|------|----------|------|
| `maimai_player_overview` | b50 + 可选 rise | 传 `source` 会下传到子查询 |
| `maimai_push_plan` | b50 + rise + chart | 同上 |
| `maimai_lookup_song` | search + chart（可选 minfo） | 同上 |
| `maimai_plate_status` | 牌子流程 | 同上 |

## 用户设置（per-QQ）

| 工具 | 作用 |
|------|------|
| `maimai_user_show` | 主题 / 默认数据源 / Import-Token 绑定状态 |
| `maimai_user_set_theme` | `circle` / `prism_plus` |
| `maimai_user_set_source` | **永久**默认源：水鱼 / 落雪（仅用户明确要求） |
| `maimai_user_bind_lxns` | 落雪 OAuth |
| `maimai_user_bind_import_token` | 水鱼 Import-Token |

## 成绩上传

| 工具 | 作用 |
|------|------|
| `maimai_update_records` | 官服扫码上传；**`source` 必填** `divingfish` \| `lxns` \| `both` |

## 【高级·默认跳过】昵称缓存

| 工具 | 作用 |
|------|------|
| `maimai_refresh_identity` | NapCat 拉好友/群 |
| `maimai_identity_status` | 缓存状态 |
| `maimai_resolve_qq` | 昵称 → QQ |
| `maimai_get_qq_identity` | 按 QQ 读缓存 |
