# AstrBot / Agent：maimai-mcp 身份规则（必读片段）

把下面内容加入系统提示或 Agent 提示，避免把**群号**当成**玩家 QQ**。

## 目标解析

- **默认查谁**：用户说「我的 / 帮我查 / 13 分数列表」→ 用**发送者 QQ**（消息前缀 `昵称/QQ` 或事件 `sender_id`）。
- **@ 他人**：「他的 B50」→ 用**被 @ 的 QQ**，不是发送者。
- **群号**：只作上下文，**禁止**写入 `qq`。会话 ID 常见形态：
  - `GroupMessage:{userQQ}_{groupId}`
  - 或日志 `sender/userQQ` 与群 id 分开
  - **第一段 / 斜杠前数字是用户 QQ，群号在后**。
- 能确定 QQ 时，查分/进度/分数列表等工具**必须传 `qq`**。

## 推荐开场工具

```text
maimai_set_identity({
  "qq": <发送者QQ>,
  "group_id": <当前群号>   // 可选但推荐
})
```

之后同进程内可省略 `qq`（session 粘性）。**MCP 重启后需重新 set**。

## 禁止

- `qq` = 群号（把群 id 误填成玩家 QQ）。
- 对错误 QQ 调用 `maimai_user_set_source` / `bind_lxns`。
- 假设聊天上下文会自动注入 MCP（**不会**）。

## 工具对照

| 需求 | 工具 |
|------|------|
| 绑定本会话身份 | `maimai_set_identity` |
| 查看 session | `maimai_get_session` |
| 昵称反查 QQ（需 identity 缓存） | `maimai_resolve_qq` |
| 等级/定数分数列表 | `maimai_score_list` + `qq` |
| 牌子完成表 | `maimai_plate` + `qq` |
| B50 | `maimai_b50` + `qq` |

## 身份缓存（可选）

若本机有 `identity_cache.json`（含 `users` / `groups`），可设置：

```text
QQ_IDENTITY_CACHE_DIR=/path/to/qq-identity-cache
```

本 MCP 只读该缓存，不负责拉取好友/群成员。

## 宿主插件

`QQToolsPlugin` 的 `on_all_events` / `on_llm_request` / `on_decorating_result` 须与当前 AstrBot 钩子签名一致（多一个参数时用 `*args` 或补全形参），否则会刷 TypeError。更优：在 `on_llm_request` 注入 `sender_id` / `group_id` 或直接调 `maimai_set_identity`。
