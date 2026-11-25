# v0.3

> 状态：规划中

本文是 v0.3 的版本入口，描述版本主题、相对 v0.2 的能力增量与文档导航；不重复 v0.1、v0.2 的产品基线。

## 主题

在 v0.2 之上推进赛季：赛季周期、积分软重置与卡牌轮换；场景、大决战、商业化、好友与扩展项待定。

## 相对 v0.2 的增量

| 内容 | 范围 |
| --- | --- |
| 赛季 | 赛季周期、积分软重置与卡牌轮换 |

## 待定项

以下需求的实现版本在[需求总表](../../requirements/README.md)中为「待定」，不计入当前版本：

- v0.3 卡池增量 `R-CONT-DATA-012`
- 场景默认开启 `R-SYS-RULE-019`、大决战模式 `R-SYS-RULE-020`、英雄坐镇 `R-SYS-RULE-021`、陨落与遗嘱 `R-SYS-RULE-022`
- 好友榜与好友小队 `R-SOC-ACCT-032`、商业化 `R-SOC-SHOP-033`
- v0.3 扩展项产品文档 `R-DOC-DOCS-045`

## 文档导航

- 产品增量：[product/README.md](product/README.md)
- 未确认事项：[open-questions.md](open-questions.md)
- 跨版本路线：[版本路线与风险](../../version-roadmap.md)

## 数据与校验

本版本尚未新增卡牌与成就数据；卡池增量待定，确定后按版本开发流程补齐，并由 `python3 scripts/validate_docs.py` 校验。
