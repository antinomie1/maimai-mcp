# 1. Rust MCP 重构与整体架构选型

Date: 2026-08-18

## 状态
已通过 (Accepted)

## 上下文与问题陈述
原 Python 项目基于 nonebot / 异步网络和 PIL 实现，存在启动慢、内存开销较大、多层抽象冗长的问题。
重构目标是为 LLM（尤其是部署在 AstrBot 及 QQ 机器人上的弱/小参数模型）提供高性能、代码量小于 10,000 行、低内存占用的 Rust MCP Server。

## 决策驱动因素
* **代码简洁度**：杜绝多层冗余封装，控制在 10,000 行有效代码内。
* **极限性能与轻量**：冷启动 < 100ms，出图 < 50ms，常驻内存 < 30MB。
* **生态兼容性**：完全兼容原版静态资源目录（`static/font`, `static/data`, `static/mai/...`），兼容 AstrBot / MCP Inspector 协议。

## 考虑的选项
* **选项 1**：基于 `rmcp` 官方 SDK + 精简版 `tokio` (rt-multi-thread + io-std) + `log`/`env_logger`。
* **选项 2**：手写无依赖 JSON-RPC 2.0 stdio 循环。
* **选项 3**：保留 Python 侧并封装 subprocess / HTTP wrapper。

## 决策结果
选择 **选项 1**。
通过精简 `tokio` 特性并移除 `tracing-subscriber` 依赖树，兼顾了官方 MCP 规范兼容性与极致的编译/运行轻量化。

### 影响分析
* **正向**：开发周期短、协议严谨、依赖干净、单二进制独立分发。
* **负向**：需遵循 `rmcp` 的类型生命周期。
