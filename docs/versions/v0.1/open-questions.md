# 待确认事项

本文只保留 v0.1 尚未确认的事项；只登记定不了的问题，不保留讨论过程、备选方案与倾向性说明。结论确认后写入对应文档，并从本文移除。

## 产品与平衡

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 平衡推演未考虑 8 人局的牌池争抢 | 平衡 | 需要实测校准 | [平衡测算](product/content/balance-analysis.md) |
| 随从统一 3 金币是否让高星级卡缺乏稀缺感 | 经济 | 首版保留统一价，观察 6 星卡出现时机 | [数值与经济](product/systems/economy-and-numbers.md) |
| 是否允许「卖血升级」 | 单局节奏 | 规则允许，观察是否成为唯一解 | [数值与经济](product/systems/economy-and-numbers.md) |
| 学者「血量上浮 30%」在 6/7 星是否过高 | 平衡 | 观察，必要时降到 20% | [平衡测算](product/content/balance-analysis.md) |
| 6 星卡中学者占 5 张是否会让终局过度偏向学者 | 平衡 | 首版先观察，必要时降 1–2 张为 5 星 | [卡牌规范与数据](product/content/card-system.md) |
| 7 星 12 张中学者占 3 张（费米、海森堡、薛定谔）是否偏高 | 平衡 | 观察终局学者占比；判据 7 已设为 ≤ 30% | [平衡测算](product/content/balance-analysis.md) |
| 7 星池每张 5 份在 8 人局是否过少 | 平衡 | M0 模拟后校准 | [数值与经济](product/systems/economy-and-numbers.md) |
| 升满七段的实测回合是否落在 12–14 | 节奏 | M0 模拟后校准 | [数值与经济](product/systems/economy-and-numbers.md) |
| 古典段一次开放三个关键词是否让前期复杂度陡增 | 平衡 | 观察新手首局 | [关键词与机制](product/rules/keywords-and-mechanics.md) |

## 技术与版本

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 服务端工程目录落点与 workspace 拆分细度 | 实现 | 搭建服务端工程前确定 | [服务端技术基线](technical/architecture/server-stack.md) |
| 服务端部署编排与多实例切分维度 | 部署 | M2 联机内测前确定 | [服务端技术基线](technical/architecture/server-stack.md) |
| 客户端工程目录与构建导出流程未定 | 实现 | 搭建客户端工程前确定 | [客户端技术基线](technical/architecture/client-stack.md) |
| 客户端测试框架未定 | 实现 | 搭建客户端工程前确定 | [客户端技术基线](technical/architecture/client-stack.md) |
| 移动端纳入首版后的人周估算 | 排期 | 当前 22–31 人周未含移动端适配与两端验收，需在 M1 结束后重估 | [版本规划与风险](../../version-roadmap.md) |
| 七时代是否需要重估 22–31 人周 | 排期 | M0 前重估 | [版本规划与风险](../../version-roadmap.md) |

## 文档结构

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 仓库缺少根 `AGENTS.md` 的协作原则、runbook 位置声明与项目事实 | 仓库治理 | 按项目接入流程补建；缺用户规模与端到端测试入口，不能留空占位 | 仓库根目录 |
