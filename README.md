# maimai-mcp

本地可用的舞萌 DX **查询库 / CLI / MCP 服务**：查曲、查分、进度与绘图，供命令行或 Agent 调用。

- 仓库：https://github.com/antinomie1/maimai-mcp  
- 上游参考：[Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX)  
- 绘图沿用原版布局与素材接口  

## 能做什么

- 查曲、谱面信息、分数线  
- 水鱼 b50 / 用户名查分；有开发者 Token 时可查更完整成绩  
- 上分推荐、排名、今日运势等  
- 定数表 / 完成表 / 牌子与等级进度  
- 约 28 个 MCP tools；组合工具可内部串联（如搜歌 → 出图）  

默认主题 **circle**；需要 prism_plus 时再显式切换。身份支持 **QQ** 或 **水鱼用户名**。

落雪查分器是**可选**的，不配也能用水鱼主流程。

## 目录

```text
maimai_mcp/     业务 + CLI + MCP（主包）
  core/         客户端、领域逻辑、原版 image
  features/     功能拆分（query / draw）
  tools/        MCP 工具层
scripts/        Inspector、冒烟脚本
```

## 安装与配置

```bash
git clone https://github.com/antinomie1/maimai-mcp.git
cd maimai-mcp
pip install -e .
# ginfo 饼图需要：
# playwright install chromium

cp maimai_mcp/.env.example maimai_mcp/.env
```

`maimai_mcp/.env` 至少：

| 变量 | 说明 |
|------|------|
| `MAIMAIDX_PATH` | `static` 的**绝对路径**（必填） |
| `DIVINGFISH_TOKEN` | 水鱼开发者 Token（强烈建议） |
| `DEFAULT_USERNAME` / `DEFAULT_QQ` | 可选默认身份 |
| `LXNS_*` | 仅在使用落雪时填写 |

资源包需自行下载（与 maimaiDX 相同），解压后指向 `static`：

- [Cloudreve](https://cloud.yuzuchan.moe/f/34s7/Resource%20CN1.55.7z)  
- [OneDrive](https://yuzuai-my.sharepoint.com/:u:/g/personal/yuzu_yuzuchan_moe/IQBGKHie6MAaTZy3rME7Q-ruAVKgXDCKROqz5e25KtMeeVY?e=53eC6a)  

请遵守上游美术相关声明。

首次若要用表类指令，生成一次表图：

```bash
python -m maimai_mcp.cli update tables
```

## 使用

**CLI**

```bash
# 未 install -e 时：export PYTHONPATH=.
python -m maimai_mcp.cli b50 --username <水鱼用户名> --out out/b50.png
python -m maimai_mcp.cli chart 834
python -m maimai_mcp.cli search --mode 定数 14.0 --format json
python -m maimai_mcp.cli user theme prism_plus --qq <QQ>
```

**MCP**

```bash
python -m maimai_mcp
```

MCP 由客户端拉起。配置示例（路径改成你的机器）：

```json
{
  "mcpServers": {
    "maimai": {
      "command": "python",
      "args": ["-m", "maimai_mcp"],
      "cwd": "/path/to/maimai-mcp",
      "env": {
        "PYTHONPATH": "/path/to/maimai-mcp"
      }
    }
  }
}
```

Windows 示例见 `mcp.example.json`。

**联调**

```bash
./scripts/run_inspector.ps1          # 或 run_inspector.sh
python scripts/smoke_mcp_tools.py --username <水鱼用户名>
```

**刷新曲库**

```bash
python -m maimai_mcp.cli update music
python -m maimai_mcp.cli update alias
```

## 致谢

- [Yuri-YuzuChaN/maimaiDX](https://github.com/Yuri-YuzuChaN/maimaiDX) — 绘图与业务参考
- [Diving-Fish 查分器](https://www.diving-fish.com/maimaidx/prober/) — 成绩与曲库数据
- [落雪查分器](https://maimai.lxns.net/) — 可选成绩数据源
- [柚子别名](https://www.yuzuchan.moe/) — 曲目别名数据
