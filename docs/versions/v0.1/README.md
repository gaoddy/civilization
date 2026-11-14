# v0.1

本文是 v0.1 的版本入口，描述版本主题、目标、能力清单、状态与文档导航；不复制产品与技术基线内容。

> 状态：规划中

## 主题

《文明战旗》首版：把炉石酒馆战棋的对局骨架、文明系列的时代演进与科学史的人物和概念装进同一副牌，8 人联网对战、20–30 分钟一局。

## 目标

1. 新玩家第一局能在 3 分钟内完成一次购买与出战；
2. 老玩家能连续打 20 局而不觉得每局都一样；
3. 8 人完整对局在 20–30 分钟内结束；
4. 天梯前 100 局不出现单一流派胜率超过 60%。

## 能力清单

| 内容 | 范围 |
| --- | --- |
| 对局 | 8 名真人、40 点生命、招募 / 战斗 / 结算三段状态机；不设 AI 补位 |
| 时代 | 7 个：史前、原始、古典、封建、工业、现代、未来 |
| 流派 | 6 个，每局启用 5 个（学者固定启用） |
| 卡牌 | 176 张随从 + 36 张效果牌 |
| 星级 | 1–7 |
| 英雄 | 13 位，各带 1 个英雄技能 |
| 技能 | 118 个，卡牌与英雄共享引用 |
| 关键词 | 13 个：通用 10 个 + 机械、植物、生物专属 3 个 |
| 模式 | 会战（默认）、比赛 |
| 成就 | 6 个 |
| 客户端 | 桌面端 + 移动端，共用同一套规则引擎与协议 |
| 新手引导 | 不做教学局，只做界面提示与高亮 |
| 扩展层 | 场景、陨落、英雄坐镇、大决战默认关闭 |

## 阅读顺序

按依赖链阅读：总览与规则 → 数值 → 内容 → 系统规则 → 技术约束。

1. [设计总纲](product/overview/design-overview.md) 与[概念与定位](product/overview/concept-and-positioning.md)
2. [核心规则](product/rules/core-rules.md)、[关键词与机制](product/rules/keywords-and-mechanics.md)、[时代与场景](product/rules/eras-and-scenarios.md)
3. [数值与经济](product/systems/economy-and-numbers.md)
4. [卡牌规范与数据](product/content/card-system.md) 与[卡牌索引](product/content/cards/README.md)
5. [英雄与技能](product/content/hero-system.md) 与[英雄索引](product/content/heroes/README.md)
6. [平衡测算](product/content/balance-analysis.md)（从卡表回算的校验输出）
7. [模式与成就](product/systems/modes-and-achievements.md)
8. [联网与技术架构](technical/architecture/network-architecture.md)

## 文档导航

- 产品基线：[product/README.md](product/README.md)
- 技术基线：[technical/README.md](technical/README.md)
- 未确认事项：[open-questions.md](open-questions.md)
- 跨版本路线：[版本路线与风险](../../version-roadmap.md)、[内容素材库](../../content-backlog.md)

## 数据与校验

卡牌、英雄与技能实体文档是唯一数据源；卡牌与英雄字段规范见[卡牌规范与数据](product/content/card-system.md)与[英雄与技能](product/content/hero-system.md)，技能字段规范见[技能规范与数据](product/content/skill-system.md)；每次修改后运行 `python3 scripts/validate_docs.py`。
