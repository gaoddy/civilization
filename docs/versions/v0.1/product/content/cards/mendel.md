---
id: mendel
card_type: 随从
name: 孟德尔
tribe: 学者
subtype: 生物
level: 4
era: 现代
attack: 0
health: 12
skills: [battlecry-friendly-plant-growth-once]
is_scholar: true
is_hero: true
hero_skill: hero-heredity
hero_skill_cost: 1
hero_skill_type: 主动
hero_skill_uses_per_turn: 1
hero_skill_trigger: 主动使用
---

# 孟德尔

## 属性

| 属性 | 值 |
| --- | --- |
| id | mendel |
| 名称 | 孟德尔 |
| 类型 | 随从 |
| 流派 | 学者 |
| 子类 | 生物 |
| 星级 | 4 |
| 时代 | 现代 |
| 攻击力 | 0 |
| 生命值 | 12 |
| 关键词 | 战吼 |
| 技能 | battlecry-friendly-plant-growth-once |
| 学者 | 是 |
| 英雄资格 | 是 |
| 英雄技能 | hero-heredity |
| 英雄技能费用 | 1 |
| 英雄技能类型 | 主动 |
| 每回合次数 | 1 |
| 触发时机 | 主动使用 |

## 效果

战吼：使一个友方植物随从获得一次成长

## 英雄技能

遗传：使一个友方植物随从获得一次成长

## 设计说明

- 定位：植物体系专属，给植物一次成长。
- 平衡边界：若场上没有植物，技能不可用（返回 `NO_VALID_TARGET`）；不提供「发现植物」的兜底。

## 出处

孟德尔在 1856—1863 年以豌豆杂交实验总结出分离定律与自由组合定律。

对应：素材原始笔记，无外部出处。