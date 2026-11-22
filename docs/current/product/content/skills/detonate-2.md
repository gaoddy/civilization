---
id: detonate-2
template: combatstart-detonate(amount=2)
---

# detonate-2

## 属性

| 属性 | 值 |
| --- | --- |
| id | detonate-2 |
| 关键词 | 无 |
| 模板 | combatstart-detonate |

## 效果

引爆：战斗开始时，移除一个敌方随从的全部传染层，每层造成 2 点伤害

## 设计说明

定位：细菌域的传染收益终结技，按目标身上的层数一次性结算。
口径：只设置每层伤害，不设置阈值；`threshold` 为引擎预留能力，卡池暂无使用者。
