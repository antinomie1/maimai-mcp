# Maimai DX 领域模型术语表 (Domain Glossary)

本文档定义 Maimai DX MCP Server 项目中的标准领域概念与统一命名规范。

---

## 1. 核心数值与评级体系

* **Achievement Rate (达成率)**：单曲游玩得分百分比，精确到 4 位小数（如 `100.5000%`），最大值为 `101.0000%`。
* **Base Rating / Internal Level (定数 / 基础定数)**：谱面的隐藏真实难度数值，精确到一位小数（如 `13.7`, `14.2`）。
* **DX Rating (玩家评分)**：衡量玩家水平的核心数值，由 B50（前 50 首最高单曲 Rating）累加而成。
* **Single Song Rating (单曲 Rating)**：根据单曲达成率和谱面定数计算出的 Rating 值：
  - 计算公式：`定数 × 达成率系数 × 评价等级系数`
* **Best 50 / B50 (最佳50首)**：
  - **B35 (Past / 旧版本曲目最佳 35 首)**：现行版本之前发布的曲目中单曲 Rating 最高的 35 首。
  - **B15 (New / 当前版本新曲最佳 15 首)**：当前版本最新发布的曲目中单曲 Rating 最高的 15 首。
* **Combo Status (连击状态)**：
  - `FC` (Full Combo): 全连
  - `FC+` (Full Combo Plus): 全 Great 以上连击
  - `AP` (All Perfect): 全 Perfect
  - `AP+` (All Perfect Plus): 全 Critical Perfect (理论最高)
* **Sync Status (双人/多人同步状态)**：
  - `SYNC` / `FS` (Full Sync) / `FS+` / `FDX` (Full Sync DX) / `FDX+`
* **DX Score (DX 分)**：基于 Note 判定类型（Critical Perfect = 3分, Perfect = 2分, Great = 1分）累加的独立分数体系，对应 DX 星级（1星至5星）。

---

## 2. 曲目与谱面体系

* **Song ID (曲目 ID)**：游戏内部唯一标识编号（如 1 ~ 10000+，DX 谱面常加 10000 偏移）。
* **Difficulty (难度)**：
  - `0`: Basic (绿)
  - `1`: Advanced (黄)
  - `2`: Expert (红)
  - `3`: Master (紫)
  - `4`: Re:Master (白)
* **Song Type (曲目类型)**：
  - `SD` (Standard / 标准谱面)
  - `DX` (Deluxe / 舞萌DX谱面)
* **Song Alias (曲目别名)**：玩家社群中对歌曲的俗称、简称、缩写或梗名（如“潘多拉”、“茄子”、“拉脱维亚”），来源于柚子 (Yuzuchan) / 落雪 (LXNS) / 本地维护库。

---

## 3. 查分数据源与身份模型

* **DivingFish (水鱼查分器)**：国内主流 maimai 查分平台；公开接口可查 B50，其余查询需用户完成 OAuth 授权（开发者 Token 已废弃）。
* **LXNS (落雪查分器)**：开放 API 查分平台；用户数据通过 OAuth 授权访问，开发者 Token 用于曲库、别名与 AP50。
* **User Identity (用户身份)**：
  - `QQ`: 用户 QQ 号（主索引键）。
  - `Service / Source`: 默认查分源（默认为 `divingfish`，支持无缝切换至 `lxns`）。
  - `User Config`: 本地隔离存储的用户偏好配置（JSON 格式）。
