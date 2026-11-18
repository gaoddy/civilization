---
id: ergot
card_type: minion
name: 麦角
playstyle: fungus
taxon: clavicipitaceae
subtype: fungus
level: 5
era: feudal
attack: 6
health: 12
skills: [decompose-ally-kill-pre-kill, deathrattle-summon-2-spores-2-2]
is_scholar: false
---

# 麦角

## 属性

| 属性 | 值 |
| --- | --- |
| id | ergot |
| 名称 | 麦角 |
| 类型 | 随从 |
| 玩法 | 真菌界 |
| 科学分类 | 真菌界 › 子囊菌门 › 粪壳菌纲 › 肉座菌目 › 麦角科 |
| 关联分类 | 无 |
| 子类 | 真菌 |
| 星级 | 5 |
| 时代 | 封建 |
| 攻击力 | 6 |
| 生命值 | 12 |
| 关键词 | 分解；亡语 |
| 技能 | decompose-ally-kill-pre-kill；deathrattle-summon-2-spores-2-2 |
| 学者 | 否 |
| 英雄资格 | 否 |

## 效果

分解：友方随从击杀敌方随从后，获得其被击杀前的生命值；分裂：死亡时召唤两个 2/2 的孢子

## 设计说明

定位：真菌界的高级分解单位，友方击杀后按被击杀前生命值成长。
实现口径：OnAllyKill 只统计友方击杀敌方的事件，获得被击杀者死亡前生命值。
使用建议：与高攻友方同场，通过队友击杀快速堆叠生命。

## 出处

麦角菌寄生禾本科植物，其生物碱可引发麦角中毒。
