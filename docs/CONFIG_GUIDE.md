# 配置与部署指南

## 1. 安装

- 预编译二进制：[GitHub Releases](https://github.com/antinomie1/maimai-mcp/releases)（Windows / Linux）。
- 源码编译（Rust 1.88+，2024 Edition）：`cargo build --release`，产物为 `target/release/maimai-mcp`。

## 2. 静态资源

下载资源包并解压，记下其中 `static` 目录的绝对路径（即 `MAIMAI_STATIC_DIR`）：

- [Cloudreve](https://cloud.yuzuchan.moe/f/34s7/Resource%20CN1.55.7z) / [OneDrive](https://yuzuai-my.sharepoint.com/:u:/g/personal/yuzu_yuzuchan_moe/IQBGKHie6MAaTZy3rME7Q-ruAVKgXDCKROqz5e25KtMeeVY?e=53eC6a)
- 增量包（`鏡` / `廻` 牌与新曲绘）：[Resource 3.1.5 UPDATE](https://cloud.yuzuchan.moe/f/x3f3/Resource%203.1.5%20UPDATE.7z)，覆盖解压到同一 `static`。

`static/data`（曲目、别名、牌子数据）与 `static/mai/rating_table`、`static/mai/plate_table`（定数表、完成表底图）
无需手动准备：服务启动时自动联网获取数据并生成表格，之后每天 04:00（UTC+8）自动更新；网络失败时回退到本地暂存文件。

## 3. 查分器凭据

水鱼已废弃开发者 Token，本项目只使用 OAuth。

| 查分器 | 配置 | 未配置时 |
| :--- | :--- | :--- |
| 水鱼 | 在 [开发者控制台](https://auth.diving-fish.com/console) 申请应用，填 `DIVINGFISH_CLIENT_ID` / `DIVINGFISH_CLIENT_SECRET` | 仅可使用 `mai_b50`（公开接口） |
| 落雪 | 申请 OAuth 应用（权限勾选前三项），填 `LX_CLIENT_ID` / `LX_CLIENT_SECRET` / `REDIRECT_URI` | 无法绑定落雪 |
| 落雪开发者 Token | `LXNS_DEV_TOKEN` | 不合并落雪曲目/别名数据，AP50 不可用 |

绑定流程：
- 水鱼：调用 `mai_bind_divingfish(qq)` 获取授权链接 → 用户同意授权后得到确认码 → 再调用 `mai_bind_divingfish(qq, code)`（20 分钟内）。
- 落雪：调用 `mai_bind_lxns(qq)` 获取授权链接 → 调用 `mai_bind_lxns(qq, code)`，`code` 可为授权码或完整回调链接。

## 4. 环境变量

| 变量 | 说明 | 默认 |
| :--- | :--- | :--- |
| `MAIMAI_STATIC_DIR` | 静态资源目录（兼容 `MAIMAIDX_PATH`） | `./static` |
| `MAIMAI_DATA_DIR` | 用户配置目录 | `./data` |
| `MAIMAI_OUTPUT_DIR` | 生成图片目录 | `./output` |
| `MAIMAI_BOT_NAME` | 图片署名中的 BOT 名 | `Sakura` |
| `MAIMAI_UPDATE_HOUR` | 每日更新时刻（UTC+8，`off` 关闭） | `4` |
| `ASSETS_ONLINE` | 落雪头像/姓名框从资源服务器获取 | `true` |
| `MAIMAIDX_ALIAS_PROXY` | 使用柚子别名服务器境内中转 | `false` |
| `DIVINGFISH_PROBER_PROXY` | 使用水鱼查分器境外中转 | `false` |
| `DIVINGFISH_CLIENT_ID` / `DIVINGFISH_CLIENT_SECRET` | 水鱼 OAuth 应用 | 无 |
| `DIVINGFISH_SCOPE` | 水鱼 OAuth 权限，空格分隔 | `prober.records.read` |
| `DIVINGFISH_AUTH_URL` | 水鱼账号服务地址 | `https://auth.diving-fish.com` |
| `LXNS_DEV_TOKEN` | 落雪开发者 Token | 无 |
| `LX_CLIENT_ID` / `LX_CLIENT_SECRET` / `REDIRECT_URI` | 落雪 OAuth 应用 | 无 |
| `RUST_LOG` | 日志等级 | `info` |

## 5. 客户端接入（AstrBot / Claude Desktop / Cursor 等）

```json
{
  "mcpServers": {
    "maimai": {
      "command": "/opt/maimai-mcp/maimai-mcp",
      "env": {
        "MAIMAI_STATIC_DIR": "/opt/maimai-mcp/static",
        "MAIMAI_DATA_DIR": "/opt/maimai-mcp/data",
        "MAIMAI_OUTPUT_DIR": "/opt/maimai-mcp/output",
        "DIVINGFISH_CLIENT_ID": "your_client_id",
        "DIVINGFISH_CLIENT_SECRET": "your_client_secret"
      }
    }
  }
}
```

Windows 路径需写成 `C:\\Tools\\maimai-mcp\\static`。

## 6. 常见问题

- **图片缺元素 / 提示资源缺失**：检查 `MAIMAI_STATIC_DIR/font`、`mai/pic` 是否完整；表格类图片需等待首次数据更新完成，或调用 `mai_update_data(target="tables")`。
- **提示尚未授权水鱼**：除 B50 外的水鱼查询均需用户先完成 `mai_bind_divingfish` 授权。
- **落雪查询提示未授权**：数据源为落雪的用户需先完成 `mai_bind_lxns`，或用 `mai_user_settings(source="divingfish")` 切回水鱼。
