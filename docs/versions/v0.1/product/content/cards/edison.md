---
id: edison
card_type: minion
name: 爱迪生
categories: [play:machine]
taxon: hominidae
subtype: engineering
level: 5
era: industrial
attack: 9
health: 11
skills: [invention-factory]
is_scholar: false
---

# 爱迪生

## 属性

| 属性 | 值 |
| --- | --- |
| id | edison |
| 名称 | 爱迪生 |
| 类型 | 随从 |
| 玩法 | 机械 |
| 科学分类 | 动物界 › 脊索动物门 › 哺乳纲 › 灵长目 › 人科 |
| 关联分类 | 无 |
| 子类 | 工程 |
| 星级 | 5 |
| 时代 | 工业 |
| 攻击力 | 9 |
| 生命值 | 11 |
| 关键词 | 无 |
| 技能 | invention-factory |
| 学者 | 否 |

## 效果

使你的其他机械随从各获得一次成长

## 设计说明

- 定位：机械体系的主动引擎，把「只能靠装配入场滚雪球」变成「每回合可以主动投入」。
- 平衡边界：7 个机械在场时为 6 次成长，正好等于单回合属性差上限 6；场上没有其他机械时技能不可用（返回 `NO_VALID_TARGET`）。
- 与装配的分工：装配在入场时按 N 结算，随机械数量放大；本技能不依赖入场，只按当前在场的其他机械数量结算。

## 出处

1879 年改良白炽灯并建立直流电力系统，推动电力的商业化。
