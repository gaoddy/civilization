# 技能索引

本文索引 v0.1 的 259 个技能实体文档，一行一个；字段规范与校验规则见[技能系统](../skill-system.md)。文件名等于技能 `id`，是数据契约的一部分；技能 `id` 与卡牌 `id` 分属两个命名空间，允许同名。技能实例用 `template` 引用[技能模板](../skill-templates/README.md)并只写参数，卡面文本由模板渲染，特殊措辞才用 `text` 覆盖。技能按科学分类（`taxon`）、玩法（`playstyle`）与关联分类（`play` / `prey` / `host`）筛选，筛选口径见[技能系统](../skill-system.md) §2.2。英雄卡引用的技能不带英雄专属前缀；主动技能带 `cost` 与 `uses_per_turn`，触发式技能按文本前缀结算。

| 技能 | 关键词 | 文本 | 引用数 |
| --- | --- | --- | --- |
| [activate-friendly-growth-twice](activate-friendly-growth-twice.md) | 无 | 使一个友方随从获得两次成长 | 1 |
| [activate-reptile-permanent-taunt](activate-reptile-permanent-taunt.md) | 无 | 使一个友方随从获得嘲讽 | 1 |
| [alternating-current](alternating-current.md) | 无 | 使一个友方随从获得无视嘲讽直到回合结束 | 1 |
| [aura-adjacent-mammal](aura-adjacent-mammal.md) | 无 | 相邻的友方哺乳纲随从获得 +2/+2 | 1 |
| [aura-all-plus-0-plus-1](aura-all-plus-0-plus-1.md) | 无 | 你的随从获得 +0/+1 | 3 |
| [aura-all-plus-0-plus-2](aura-all-plus-0-plus-2.md) | 无 | 你的随从获得 +0/+2 | 1 |
| [aura-biology-plus-0-plus-1](aura-biology-plus-0-plus-1.md) | 无 | 你的动物随从获得 +0/+1 | 1 |
| [aura-biology-plus-1-plus-1](aura-biology-plus-1-plus-1.md) | 无 | 你的动物随从获得 +1/+1 | 1 |
| [aura-mech-optimal-attack](aura-mech-optimal-attack.md) | 无 | 你的机械随从获得无视嘲讽 | 1 |
| [aura-mech-plus-2-plus-2](aura-mech-plus-2-plus-2.md) | 无 | 你的机械随从获得 +2/+2 | 2 |
| [aura-other-biology-plus-1-attack](aura-other-biology-plus-1-attack.md) | 无 | 你的其他动物随从获得 +1 攻击力 | 1 |
| [aura-other-fish-plus-0-plus-1](aura-other-fish-plus-0-plus-1.md) | 无 | 你的其他鱼纲随从获得 +0/+1 | 1 |
| [aura-other-insect-plus-0-plus-1](aura-other-insect-plus-0-plus-1.md) | 无 | 你的其他昆虫纲随从获得 +0/+1 | 1 |
| [aura-other-mech-plus-1-plus-1](aura-other-mech-plus-1-plus-1.md) | 无 | 你的其他机械随从获得 +1/+1 | 1 |
| [aura-other-plants-plus-0-plus-1](aura-other-plants-plus-0-plus-1.md) | 无 | 你的其他植物随从获得 +0/+1 | 0 |
| [aura-other-plus-0-plus-1](aura-other-plus-0-plus-1.md) | 无 | 你的其他随从获得 +0/+1 | 1 |
| [aura-other-plus-0-plus-2](aura-other-plus-0-plus-2.md) | 无 | 你的其他随从获得 +0/+2 | 1 |
| [aura-other-plus-0-plus-3](aura-other-plus-0-plus-3.md) | 无 | 你的其他随从获得 +0/+3 | 1 |
| [aura-other-protozoa-plus-1-attack](aura-other-protozoa-plus-1-attack.md) | 无 | 你的其他原生生物界随从获得 +1 攻击力 | 5 |
| [aura-plant-plus-1-plus-1](aura-plant-plus-1-plus-1.md) | 无 | 你的植物随从获得 +1/+1 | 2 |
| [aura-plant-plus-2-plus-2](aura-plant-plus-2-plus-2.md) | 无 | 你的植物随从获得 +2/+2 | 1 |
| [aura-plant-taunt](aura-plant-taunt.md) | 无 | 你的植物随从获得嘲讽 | 1 |
| [aura-school-fish-1](aura-school-fish-1.md) | 无 | 群聚：你的其他鱼纲随从获得 +1/+0；每有一个其他鱼纲随从，再获得 +0/+1 | 6 |
| [aura-school-fish-2](aura-school-fish-2.md) | 无 | 群聚：你的其他鱼纲随从获得 +1/+1；每有一个其他鱼纲随从，再获得 +1 攻击力 | 1 |
| [aura-school-mammal](aura-school-mammal.md) | 无 | 群聚：你的其他哺乳纲随从获得 +1/+1；每有一个其他哺乳纲随从，再获得 +1 攻击力 | 2 |
| [battlecry-all-biology-growth-once](battlecry-all-biology-growth-once.md) | battlecry | 战吼：使你的所有动物随从获得一次成长 | 1 |
| [battlecry-all-friendly-growth-once](battlecry-all-friendly-growth-once.md) | battlecry | 战吼：使你的所有随从获得一次成长 | 5 |
| [battlecry-all-friendly-growth-twice](battlecry-all-friendly-growth-twice.md) | battlecry | 战吼：使你的所有随从获得两次成长 | 1 |
| [battlecry-all-friendly-plus-0-plus-1](battlecry-all-friendly-plus-0-plus-1.md) | battlecry | 战吼：使你的所有随从获得 +0/+1 | 2 |
| [battlecry-all-mech-growth-once](battlecry-all-mech-growth-once.md) | battlecry | 战吼：使你的所有机械随从获得一次成长 | 2 |
| [battlecry-all-mech-growth-once-no-keyword](battlecry-all-mech-growth-once-no-keyword.md) | 无 | 战吼：使你的所有机械随从获得一次成长 | 1 |
| [battlecry-copy-hand-minion](battlecry-copy-hand-minion.md) | battlecry | 战吼：复制一张你手牌中的随从牌 | 1 |
| [battlecry-friendly-archers-plus-2-attack](battlecry-friendly-archers-plus-2-attack.md) | battlecry | 战吼：使你的弓箭手获得 +2 攻击力 | 1 |
| [battlecry-friendly-bacteria-plus-0-plus-1](battlecry-friendly-bacteria-plus-0-plus-1.md) | battlecry | 战吼：使一个友方细菌域随从获得 +0/+1 | 1 |
| [battlecry-friendly-bird-growth-once](battlecry-friendly-bird-growth-once.md) | battlecry | 战吼：使一个友方鸟纲随从获得一次成长 | 5 |
| [battlecry-friendly-fish-growth-once](battlecry-friendly-fish-growth-once.md) | battlecry | 战吼：使一个友方鱼纲随从获得一次成长 | 2 |
| [battlecry-friendly-fungus-growth-once](battlecry-friendly-fungus-growth-once.md) | battlecry | 战吼：使一个友方真菌界随从获得一次成长 | 1 |
| [battlecry-friendly-growth-once](battlecry-friendly-growth-once.md) | battlecry | 战吼：使一个友方随从获得一次成长 | 11 |
| [battlecry-friendly-growth-plant-twice](battlecry-friendly-growth-plant-twice.md) | battlecry | 战吼：使一个友方随从获得一次成长；若其为植物，改为两次 | 1 |
| [battlecry-friendly-growth-twice](battlecry-friendly-growth-twice.md) | battlecry | 战吼：使一个友方随从获得两次成长 | 4 |
| [battlecry-friendly-human-plus-0-plus-1](battlecry-friendly-human-plus-0-plus-1.md) | battlecry | 战吼：使一个友方人科随从获得 +0/+1 | 1 |
| [battlecry-friendly-mech-growth-once](battlecry-friendly-mech-growth-once.md) | battlecry | 战吼：使一个友方机械随从获得一次成长 | 3 |
| [battlecry-friendly-plant-growth-once](battlecry-friendly-plant-growth-once.md) | battlecry | 战吼：使一个友方植物随从获得一次成长 | 1 |
| [battlecry-friendly-plant-kingdom-growth-once](battlecry-friendly-plant-kingdom-growth-once.md) | battlecry | 战吼：使一个友方植物界随从获得一次成长 | 0 |
| [battlecry-friendly-plus-0-plus-1](battlecry-friendly-plus-0-plus-1.md) | battlecry | 战吼：使一个友方随从获得 +0/+1 | 4 |
| [battlecry-friendly-plus-0-plus-2](battlecry-friendly-plus-0-plus-2.md) | battlecry | 战吼：使一个友方随从获得 +0/+2 | 4 |
| [battlecry-friendly-plus-1-attack](battlecry-friendly-plus-1-attack.md) | battlecry | 战吼：使一个友方随从获得 +1 攻击力 | 1 |
| [battlecry-friendly-plus-1-health](battlecry-friendly-plus-1-health.md) | battlecry | 战吼：使一个友方随从获得 +1 生命值 | 1 |
| [battlecry-friendly-protozoa-growth-once](battlecry-friendly-protozoa-growth-once.md) | battlecry | 战吼：使一个友方原生生物界随从获得一次成长 | 1 |
| [battlecry-friendly-virus-plus-1-attack](battlecry-friendly-virus-plus-1-attack.md) | battlecry | 战吼：使一个友方病毒随从获得 +1 攻击力 | 1 |
| [battlecry-other-bird-plus-1-plus-1](battlecry-other-bird-plus-1-plus-1.md) | battlecry | 战吼：使你的其他鸟纲随从获得 +1/+1 | 1 |
| [battlecry-other-friendly-humans-plus-1-attack](battlecry-other-friendly-humans-plus-1-attack.md) | battlecry | 战吼：使你的其他人科随从获得 +1 攻击力 | 5 |
| [battlecry-other-insect-growth-once](battlecry-other-insect-growth-once.md) | battlecry | 战吼：使你的其他昆虫纲随从获得一次成长 | 2 |
| [battlecry-plant-growth-twice](battlecry-plant-growth-twice.md) | battlecry | 战吼：使一个友方植物随从获得两次成长 | 1 |
| [battlecry-random-enemy-damage-1](battlecry-random-enemy-damage-1.md) | battlecry | 战吼：对随机一个敌方随从造成 1 点伤害 | 2 |
| [battlecry-random-enemy-damage-2](battlecry-random-enemy-damage-2.md) | battlecry | 战吼：对随机一个敌方随从造成 2 点伤害 | 1 |
| [battlecry-self-growth-twice](battlecry-self-growth-twice.md) | battlecry | 战吼：获得两次成长 | 1 |
| [battlecry-self-plus-1-health](battlecry-self-plus-1-health.md) | battlecry | 战吼：获得 +1 生命值 | 1 |
| [battlecry-summon-2-worker-bees](battlecry-summon-2-worker-bees.md) | battlecry | 战吼：召唤两个 2/2 的工蜂 | 1 |
| [battlecry-summon-3-3-synthetic-cell](battlecry-summon-3-3-synthetic-cell.md) | battlecry | 战吼：召唤一个 3/3 的合成细胞（幼体） | 1 |
| [binomial-nomenclature](binomial-nomenclature.md) | 无 | 使你的所有动物随从获得一次成长 | 0 |
| [biology-evolve](biology-evolve.md) | 无 | 使一个友方动物随从获得进化 | 1 |
| [cold-damage-treatise](cold-damage-treatise.md) | 无 | 使你的所有随从获得 +0/+1 | 0 |
| [combat-optimal-attack](combat-optimal-attack.md) | 无 | 使一个友方随从获得无视嘲讽直到回合结束 | 1 |
| [combat-random-enemy-damage-3](combat-random-enemy-damage-3.md) | 无 | 战斗开始时对随机一个敌方随从造成 3 点伤害 | 1 |
| [combatstart-all-enemies-contagion-1](combatstart-all-enemies-contagion-1.md) | 无 | 战斗开始时，使敌方所有随从获得 1 层传染 | 3 |
| [combatstart-all-enemies-contagion-2](combatstart-all-enemies-contagion-2.md) | 无 | 战斗开始时，使敌方所有随从获得 2 层传染 | 2 |
| [combatstart-all-enemies-damage-2](combatstart-all-enemies-damage-2.md) | 无 | 战斗开始时，对敌方所有随从造成 2 点伤害 | 2 |
| [combatstart-all-enemies-damage-3](combatstart-all-enemies-damage-3.md) | 无 | 战斗开始时，对敌方所有随从造成 3 点伤害 | 2 |
| [combatstart-all-enemies-damage-4](combatstart-all-enemies-damage-4.md) | 无 | 战斗开始时，对敌方所有随从造成 4 点伤害 | 1 |
| [combatstart-all-enemies-damage-5](combatstart-all-enemies-damage-5.md) | 无 | 战斗开始时，对敌方所有随从造成 5 点伤害 | 1 |
| [combatstart-all-enemies-minus-1-attack](combatstart-all-enemies-minus-1-attack.md) | 无 | 战斗开始时，使敌方所有随从获得 -1 攻击力 | 1 |
| [combatstart-if-2-biology-self-plus-2-attack](combatstart-if-2-biology-self-plus-2-attack.md) | 无 | 战斗开始时，若你控制至少 2 个动物，获得 +2 攻击力 | 1 |
| [combatstart-leftmost-and-adjacent-damage-2](combatstart-leftmost-and-adjacent-damage-2.md) | 无 | 战斗开始时，对敌方最左侧随从及其相邻随从造成 2 点伤害 | 1 |
| [combatstart-leftmost-damage-3](combatstart-leftmost-damage-3.md) | 无 | 战斗开始时，对敌方最左侧随从造成 3 点伤害 | 3 |
| [combatstart-leftmost-damage-4](combatstart-leftmost-damage-4.md) | 无 | 战斗开始时，对敌方最左侧随从造成 4 点伤害 | 1 |
| [combatstart-leftmost-damage-6](combatstart-leftmost-damage-6.md) | 无 | 战斗开始时，对敌方最左侧随从造成 6 点伤害 | 1 |
| [combatstart-lowest-attack-enemy-damage-3](combatstart-lowest-attack-enemy-damage-3.md) | 无 | 战斗开始时，对攻击力最低的敌方随从造成 3 点伤害 | 1 |
| [combatstart-random-enemy-contagion-1](combatstart-random-enemy-contagion-1.md) | 无 | 战斗开始时，使一个随机敌方随从获得 1 层传染 | 11 |
| [combatstart-random-enemy-contagion-2](combatstart-random-enemy-contagion-2.md) | 无 | 战斗开始时，使一个随机敌方随从获得 2 层传染 | 3 |
| [combatstart-random-enemy-damage-2](combatstart-random-enemy-damage-2.md) | 无 | 战斗开始时，随机对一个敌方随从造成 2 点伤害 | 1 |
| [combatstart-random-enemy-minus-2-attack](combatstart-random-enemy-minus-2-attack.md) | 无 | 战斗开始时，使一个随机敌方随从获得 -2 攻击力直到回合结束 | 1 |
| [combatstart-remove-all-human-growth](combatstart-remove-all-human-growth.md) | 无 | 战斗开始时，移除所有人科随从的全部成长 | 1 |
| [combatstart-remove-top-attack-enemy-growth](combatstart-remove-top-attack-enemy-growth.md) | 无 | 战斗开始时，移除敌方攻击力最高随从的全部成长 | 2 |
| [combatstart-top-attack-enemy-cant-attack](combatstart-top-attack-enemy-cant-attack.md) | 无 | 战斗开始时，使敌方攻击力最高的随从本回合无法攻击 | 1 |
| [combatstart-top-attack-enemy-minus-0-minus-2](combatstart-top-attack-enemy-minus-0-minus-2.md) | 无 | 战斗开始时，使敌方攻击力最高的随从获得 -0/-2 | 1 |
| [convert-3](convert-3.md) | 无 | 转化：战斗开始时，若一个敌方随从有 3 层传染，消灭它并召唤一个 2/2 的细菌 | 1 |
| [convert-4](convert-4.md) | 无 | 转化：战斗开始时，若一个敌方随从有 4 层传染，消灭它并召唤一个 4/4 的细菌 | 1 |
| [deathrattle-all-enemies-contagion-1](deathrattle-all-enemies-contagion-1.md) | deathrattle | 亡语：使敌方所有随从获得 1 层传染 | 1 |
| [deathrattle-all-enemies-damage-2](deathrattle-all-enemies-damage-2.md) | deathrattle | 亡语：对敌方所有随从造成 2 点伤害 | 2 |
| [deathrattle-all-enemies-humans-minus-2-health](deathrattle-all-enemies-humans-minus-2-health.md) | deathrattle | 亡语：使敌方所有人科随从获得 -2 生命 | 1 |
| [deathrattle-all-enemies-minus-1-health](deathrattle-all-enemies-minus-1-health.md) | deathrattle | 亡语：使敌方所有随从获得 -1 生命 | 4 |
| [deathrattle-all-friendly-plus-0-plus-1](deathrattle-all-friendly-plus-0-plus-1.md) | 无 | 亡语：使你的所有随从获得 +0/+1 | 1 |
| [deathrattle-friendly-bacteria-growth-once](deathrattle-friendly-bacteria-growth-once.md) | deathrattle | 亡语：使一个友方细菌域随从获得一次成长 | 1 |
| [deathrattle-friendly-cnidarian-growth-once](deathrattle-friendly-cnidarian-growth-once.md) | deathrattle | 亡语：使一个友方水螅纲随从获得一次成长 | 1 |
| [deathrattle-friendly-insect-growth-twice](deathrattle-friendly-insect-growth-twice.md) | deathrattle | 亡语：使一个友方昆虫纲随从获得两次成长 | 1 |
| [deathrattle-friendly-protozoa-growth-once](deathrattle-friendly-protozoa-growth-once.md) | deathrattle | 亡语：使一个友方原生生物界随从获得一次成长 | 1 |
| [deathrattle-random-enemy-damage-2](deathrattle-random-enemy-damage-2.md) | deathrattle | 亡语：对随机一个敌方随从造成 2 点伤害 | 0 |
| [deathrattle-random-enemy-hp-minus-1](deathrattle-random-enemy-hp-minus-1.md) | deathrattle | 亡语：使随机一个敌方随从获得 -1 生命值 | 1 |
| [deathrattle-random-enemy-minus-1-attack](deathrattle-random-enemy-minus-1-attack.md) | 无 | 亡语：使随机一个敌方随从获得 -1 攻击力 | 2 |
| [deathrattle-random-enemy-minus-1-health](deathrattle-random-enemy-minus-1-health.md) | deathrattle | 亡语：使随机一个敌方随从获得 -1 生命 | 1 |
| [deathrattle-summon-1-worker-bee](deathrattle-summon-1-worker-bee.md) | deathrattle | 分裂：死亡时召唤一个 1/1 的工蜂 | 2 |
| [deathrattle-summon-2-2-hela-cell-clone](deathrattle-summon-2-2-hela-cell-clone.md) | deathrattle | 亡语：召唤一个 2/2 的海拉细胞（复制体） | 1 |
| [deathrattle-summon-2-ammonites](deathrattle-summon-2-ammonites.md) | deathrattle | 分裂：死亡时召唤两个 5/5 的菊石幼体 | 1 |
| [deathrattle-summon-2-amoeba-larvae-2-2](deathrattle-summon-2-amoeba-larvae-2-2.md) | deathrattle | 分裂：死亡时召唤两个 2/2 的变形虫幼体 | 1 |
| [deathrattle-summon-2-coral-polyps](deathrattle-summon-2-coral-polyps.md) | deathrattle | 分裂：死亡时召唤两个 1/1 的珊瑚虫 | 1 |
| [deathrattle-summon-2-seabirds](deathrattle-summon-2-seabirds.md) | deathrattle | 分裂：死亡时召唤两个 3/3 的海鸟 | 1 |
| [deathrattle-summon-2-spores-2-2](deathrattle-summon-2-spores-2-2.md) | deathrattle | 分裂：死亡时召唤两个 2/2 的孢子 | 2 |
| [deathrattle-summon-3-spores-3-3](deathrattle-summon-3-spores-3-3.md) | deathrattle | 分裂：死亡时召唤三个 3/3 的孢子 | 1 |
| [deathrattle-summon-3-termites](deathrattle-summon-3-termites.md) | deathrattle | 分裂：死亡时召唤三个 2/2 的白蚁 | 1 |
| [deathrattle-summon-4-volvox-cells](deathrattle-summon-4-volvox-cells.md) | deathrattle | 分裂：死亡时召唤四个 1/1 的团藻细胞 | 1 |
| [deathrattle-summon-amoeba-larva-1](deathrattle-summon-amoeba-larva-1.md) | deathrattle | 分裂：死亡时召唤一个 1/1 的变形虫幼体 | 2 |
| [deathrattle-summon-coral-polyp](deathrattle-summon-coral-polyp.md) | deathrattle | 分裂：死亡时召唤一个 1/1 的珊瑚虫 | 1 |
| [deathrattle-summon-cordyceps](deathrattle-summon-cordyceps.md) | deathrattle | 分裂：死亡时召唤一个 3/3 的虫草 | 1 |
| [deathrattle-summon-fish-fry-2](deathrattle-summon-fish-fry-2.md) | deathrattle | 分裂：死亡时召唤两个 1/1 的鱼苗 | 3 |
| [deathrattle-summon-fish-fry-3](deathrattle-summon-fish-fry-3.md) | deathrattle | 分裂：死亡时召唤三个 1/1 的鱼苗 | 1 |
| [deathrattle-summon-octopus-larva-1](deathrattle-summon-octopus-larva-1.md) | deathrattle | 分裂：死亡时召唤一个 1/1 的章鱼幼体 | 1 |
| [deathrattle-summon-schizont](deathrattle-summon-schizont.md) | deathrattle | 分裂：死亡时召唤一个 3/3 的裂殖体 | 1 |
| [deathrattle-summon-spore-token-1](deathrattle-summon-spore-token-1.md) | deathrattle | 分裂：死亡时召唤一个 1/1 的孢子 | 1 |
| [deathrattle-summon-spore-token-2](deathrattle-summon-spore-token-2.md) | deathrattle | 分裂：死亡时召唤一个 2/2 的孢子 | 1 |
| [deathrattle-summon-termite](deathrattle-summon-termite.md) | deathrattle | 亡语：召唤一个 1/2 的白蚁 | 1 |
| [decompose-ally-kill-pre-kill](decompose-ally-kill-pre-kill.md) | decompose | 分解：友方随从击杀敌方随从后，获得其被击杀前的生命值 | 1 |
| [decompose-friendly-death-original](decompose-friendly-death-original.md) | decompose | 分解：友方随从死亡后，获得其原始生命值 | 1 |
| [decompose-kill-original](decompose-kill-original.md) | decompose | 分解：本随从击杀随从后，获得其原始生命值 | 7 |
| [detonate-1](detonate-1.md) | 无 | 引爆：战斗开始时，移除一个敌方随从的全部传染层，每层造成 1 点伤害 | 1 |
| [detonate-2](detonate-2.md) | 无 | 引爆：战斗开始时，移除一个敌方随从的全部传染层，每层造成 2 点伤害 | 4 |
| [devour-1](devour-1.md) | 无 | 吞噬：战斗开始时，消灭一个攻击力不高于 1 的敌方随从，在本场战斗内永久获得其攻击力与生命值 | 1 |
| [devour-2](devour-2.md) | 无 | 吞噬：战斗开始时，消灭一个攻击力不高于 2 的敌方随从，在本场战斗内永久获得其攻击力与生命值 | 1 |
| [devour-3](devour-3.md) | 无 | 吞噬：战斗开始时，消灭一个攻击力不高于 3 的敌方随从，在本场战斗内永久获得其攻击力与生命值 | 2 |
| [devour-4](devour-4.md) | 无 | 吞噬：战斗开始时，消灭一个攻击力不高于 4 的敌方随从，在本场战斗内永久获得其攻击力与生命值 | 2 |
| [dive-1](dive-1.md) | 无 | 俯冲：战斗开始时，对攻击力最低的敌方随从造成等同于自身攻击力的伤害 | 4 |
| [division-of-labor](division-of-labor.md) | 无 | 使一个友方人科随从获得一次成长；若你控制至少 3 个人科随从，改为两次 | 1 |
| [efficient-algorithm](efficient-algorithm.md) | 无 | 刷新商店并获得一张当前星级的随从 | 1 |
| [enemy-random-cannot-attack](enemy-random-cannot-attack.md) | 无 | 使随机一个敌方随从本回合无法攻击 | 1 |
| [evolution-discovery](evolution-discovery.md) | 无 | 发现一张本局启用的玩法随从 | 1 |
| [faraday-cage](faraday-cage.md) | 无 | 使一个友方随从获得护盾 | 1 |
| [free-refresh](free-refresh.md) | 无 | 免费刷新一次商店 | 1 |
| [gain-next-level-minion](gain-next-level-minion.md) | 无 | 获得一张获得此牌时你当前时代 +1 星的随从 | 1 |
| [heredity](heredity.md) | 无 | 使一个友方植物随从获得一次成长 | 1 |
| [human-growth-twice](human-growth-twice.md) | 无 | 使一个友方人科随从获得两次成长 | 1 |
| [invention-factory](invention-factory.md) | 无 | 使你的其他机械随从各获得一次成长 | 2 |
| [keyword-armor-1](keyword-armor-1.md) | armor 1 | 减伤 1：受到伤害时，减免 1 点伤害 | 10 |
| [keyword-armor-2](keyword-armor-2.md) | armor 2 | 减伤 2：受到伤害时，减免 2 点伤害 | 3 |
| [keyword-assemble-1](keyword-assemble-1.md) | assemble 1 | 装配 1：进入战场时，使你的其他机械随从各获得 1 次成长 | 11 |
| [keyword-assemble-2](keyword-assemble-2.md) | assemble 2 | 装配 2：进入战场时，使你的其他机械随从各获得 2 次成长 | 8 |
| [keyword-breed-1](keyword-breed-1.md) | breed 1 | 繁衍 1：回合开始时获得 +1 生命 | 11 |
| [keyword-breed-2](keyword-breed-2.md) | breed 2 | 繁衍 2：回合开始时获得 +2 生命 | 4 |
| [keyword-breed-3](keyword-breed-3.md) | breed 3 | 繁衍 3：回合开始时获得 +3 生命 | 4 |
| [keyword-contagion](keyword-contagion.md) | contagion | 传染：造成伤害后，目标每回合 -1 生命 | 16 |
| [keyword-divine-shield](keyword-divine-shield.md) | divine_shield | 护盾：抵消一次伤害 | 7 |
| [keyword-evolve](keyword-evolve.md) | evolve | 进化：从 +2/+2、护盾、复生中选择一个 | 6 |
| [keyword-ignore-taunt](keyword-ignore-taunt.md) | ignore_taunt | 无视嘲讽：攻击时无视敌方嘲讽 | 12 |
| [keyword-poison](keyword-poison.md) | poison | 剧毒：造成伤害后，若目标仍存活则将其消灭 | 8 |
| [keyword-reborn](keyword-reborn.md) | reborn | 复生：以 1 点生命复活一次 | 4 |
| [keyword-sow](keyword-sow.md) | sow | 播种：死亡时召唤一个 1/1 的幼苗 | 16 |
| [keyword-stealth](keyword-stealth.md) | stealth | 潜行：无法被单体攻击与单体效果选中，造成伤害后打破 | 4 |
| [keyword-taunt](keyword-taunt.md) | taunt | 嘲讽 | 22 |
| [keyword-windfury](keyword-windfury.md) | windfury | 连击：每回合可攻击两次 | 11 |
| [latent-3](latent-3.md) | 无 | 潜伏：回合开始时，每有 1 层传染获得 +1 攻击力；达到 3 层时对敌方所有随从造成 3 点伤害 | 1 |
| [light-speed-strike](light-speed-strike.md) | 无 | 使一个友方随从本回合获得连击 | 1 |
| [mass-energy-equivalence](mass-energy-equivalence.md) | 无 | 使你的所有随从获得一次成长 | 0 |
| [materia-medica](materia-medica.md) | 无 | 使一个友方植物随从获得两次成长 | 0 |
| [mechanism-art](mechanism-art.md) | 无 | 使一个友方机械随从获得一次成长 | 0 |
| [microbiology](microbiology.md) | 无 | 战斗开始时使敌方最左侧随从获得 1 层传染 | 5 |
| [mimic-1](mimic-1.md) | 无 | 战吼：复制一个友方随从的当前攻击力与生命值直到本回合结束 | 6 |
| [mimic-2](mimic-2.md) | 无 | 拟态：战斗开始时，复制一个敌方随从的当前攻击力与生命值直到本回合结束 | 2 |
| [mimic-3](mimic-3.md) | 无 | 拟态：战斗开始时，复制一个敌方随从的当前攻击力、生命值与关键词直到本回合结束 | 1 |
| [observation](observation.md) | 无 | 每回合首次刷新时额外出现一张当前时代的随机随从 | 2 |
| [radiation](radiation.md) | 无 | 战斗开始时对随机一个敌方随从造成 2 点伤害 | 1 |
| [radium](radium.md) | 无 | 使一个友方随从获得两次成长 | 0 |
| [recall-1](recall-1.md) | 无 | 回手：死亡时返回手牌并保留成长 | 1 |
| [revive-1](revive-1.md) | 无 | 复苏：以 +1/+1 复活一次 | 2 |
| [revive-2](revive-2.md) | 无 | 复苏：以 +2/+2 复活一次 | 1 |
| [revive-3](revive-3.md) | 无 | 复苏：以 +3/+3 复活一次 | 1 |
| [salvage-machine](salvage-machine.md) | 无 | 零件回收：死亡时获得一张随机机械随从牌 | 1 |
| [seven-voyages](seven-voyages.md) | 无 | 随机确定航程 n（1 至 7）。此后每个航行回合结束时按当回合战绩结算：胜利或平局取棋盘占位数与空位数中的较大者，失败取较小者，累计至航程结束；航程结束后的招募阶段开始时，累计金币一次性到账。 | 1 |
| [shout](shout.md) | 无 | 使一个友方随从获得一次成长 | 30 |
| [sonnet](sonnet.md) | 无 | 使你的所有随从获得一次成长 | 0 |
| [trigger-2-humans-self-plus-1-attack](trigger-2-humans-self-plus-1-attack.md) | 无 | 若你控制至少 2 个人科随从，获得 +1 攻击力 | 1 |
| [trigger-2-humans-self-plus-1-plus-1](trigger-2-humans-self-plus-1-plus-1.md) | 无 | 若你控制至少 2 个人科随从，获得 +1/+1 | 1 |
| [trigger-3-humans-self-plus-1-plus-1](trigger-3-humans-self-plus-1-plus-1.md) | 无 | 若你控制至少 3 个人科随从，获得 +1/+1 | 0 |
| [trigger-3-humans-self-plus-2-attack](trigger-3-humans-self-plus-2-attack.md) | 无 | 若你控制至少 3 个人科随从，获得 +2 攻击力 | 1 |
| [trigger-all-friendly-growth-once](trigger-all-friendly-growth-once.md) | 无 | 使你的随从各获得一次成长 | 2 |
| [trigger-all-friendly-growth-twice](trigger-all-friendly-growth-twice.md) | 无 | 使你的随从获得两次成长 | 1 |
| [trigger-all-friendly-plus-0-plus-1](trigger-all-friendly-plus-0-plus-1.md) | 无 | 使你的随从获得 +0/+1 | 1 |
| [trigger-all-friendly-plus-1-plus-1](trigger-all-friendly-plus-1-plus-1.md) | 无 | 使你的所有随从获得 +1/+1 | 3 |
| [trigger-all-mech-growth-once](trigger-all-mech-growth-once.md) | 无 | 使你的机械随从各获得一次成长 | 2 |
| [trigger-bacteria-single-armor-2-this-turn](trigger-bacteria-single-armor-2-this-turn.md) | 无 | 使一个友方细菌域随从获得减伤 2 直到回合结束 | 1 |
| [trigger-bird-single-windfury-this-turn](trigger-bird-single-windfury-this-turn.md) | 无 | 使一个友方鸟纲随从获得连击直到回合结束 | 1 |
| [trigger-cephalopod-single-poison](trigger-cephalopod-single-poison.md) | 无 | 使一个友方头足纲随从获得剧毒 | 1 |
| [trigger-cnidarian-all-plus-0-plus-4](trigger-cnidarian-all-plus-0-plus-4.md) | 无 | 使你的水螅纲随从获得 +0/+4 | 1 |
| [trigger-cnidarian-permanent-taunt](trigger-cnidarian-permanent-taunt.md) | battlecry | 战吼：使一个友方水螅纲随从获得嘲讽 | 1 |
| [trigger-cnidarian-single-poison-this-turn](trigger-cnidarian-single-poison-this-turn.md) | 无 | 使一个友方水螅纲随从获得剧毒直到回合结束 | 1 |
| [trigger-cnidarian-single-reborn](trigger-cnidarian-single-reborn.md) | 无 | 使一个友方水螅纲随从获得复生 | 1 |
| [trigger-copy-friendly-minion](trigger-copy-friendly-minion.md) | 无 | 复制一个友方随从 | 2 |
| [trigger-copy-hand-minion](trigger-copy-hand-minion.md) | 无 | 复制你手牌中的一张随从牌 | 1 |
| [trigger-discover-tribe-minion](trigger-discover-tribe-minion.md) | 无 | 发现一张本局玩法的随从 | 3 |
| [trigger-enemy-death-self-plus-1-plus-1](trigger-enemy-death-self-plus-1-plus-1.md) | 无 | 每当一个敌方随从死亡，获得 +1/+1 | 1 |
| [trigger-fish-all-growth-once](trigger-fish-all-growth-once.md) | 无 | 使你的鱼纲随从获得一次成长 | 1 |
| [trigger-fish-all-reborn](trigger-fish-all-reborn.md) | 无 | 使你的鱼纲随从获得复生 | 1 |
| [trigger-fish-single-plus-0-plus-2](trigger-fish-single-plus-0-plus-2.md) | 无 | 使一个友方鱼纲随从获得 +0/+2 | 1 |
| [trigger-friendly-biology-play-plus-1-attack](trigger-friendly-biology-play-plus-1-attack.md) | 无 | 每当一个友方动物随从进入战场，获得 +1 攻击力 | 1 |
| [trigger-friendly-death-self-plus-1-attack](trigger-friendly-death-self-plus-1-attack.md) | 无 | 每当一个友方随从死亡，获得 +1 攻击力 | 1 |
| [trigger-friendly-evolve-self-plus-1-plus-1](trigger-friendly-evolve-self-plus-1-plus-1.md) | 无 | 每当一个友方随从进化，获得 +1/+1 | 1 |
| [trigger-friendly-growth-extra-plus-0-plus-1](trigger-friendly-growth-extra-plus-0-plus-1.md) | 无 | 每当一个友方随从获得成长，它额外获得 +0/+1 | 1 |
| [trigger-friendly-growth-self-plus-1-attack](trigger-friendly-growth-self-plus-1-attack.md) | 无 | 每当一个友方随从获得成长，获得 +1 攻击力 | 1 |
| [trigger-friendly-growth-thrice](trigger-friendly-growth-thrice.md) | 无 | 使一个友方随从获得三次成长 | 1 |
| [trigger-friendly-growth-twice](trigger-friendly-growth-twice.md) | 无 | 使一个友方随从获得两次成长 | 3 |
| [trigger-friendly-human-death-plus-1-plus-1](trigger-friendly-human-death-plus-1-plus-1.md) | 无 | 每当一个友方人科随从死亡，获得 +1/+1 | 1 |
| [trigger-friendly-human-play-plus-1-attack](trigger-friendly-human-play-plus-1-attack.md) | 无 | 每当一个友方人科随从进入战场，获得 +1 攻击力 | 1 |
| [trigger-friendly-human-taunt-until-turn-end](trigger-friendly-human-taunt-until-turn-end.md) | 无 | 使一个友方人科随从获得嘲讽直到回合结束 | 1 |
| [trigger-friendly-permanent-taunt](trigger-friendly-permanent-taunt.md) | battlecry | 战吼：使一个友方随从获得嘲讽 | 1 |
| [trigger-friendly-plant-breed-2](trigger-friendly-plant-breed-2.md) | 无 | 使一个友方植物随从获得繁衍 2 | 1 |
| [trigger-friendly-plant-sow](trigger-friendly-plant-sow.md) | 无 | 使一个友方植物随从获得播种 | 1 |
| [trigger-friendly-plus-0-plus-4-this-turn](trigger-friendly-plus-0-plus-4-this-turn.md) | 无 | 使一个友方随从获得 +0/+4 直到回合结束 | 1 |
| [trigger-friendly-plus-1-plus-1](trigger-friendly-plus-1-plus-1.md) | 无 | 使一个友方随从获得 +1/+1 | 2 |
| [trigger-friendly-plus-2-attack](trigger-friendly-plus-2-attack.md) | 无 | 使一个友方随从获得 +2 攻击力 | 2 |
| [trigger-friendly-plus-2-attack-this-turn](trigger-friendly-plus-2-attack-this-turn.md) | 无 | 使一个友方随从获得 +2 攻击力直到回合结束 | 1 |
| [trigger-friendly-single-armor-2-this-turn](trigger-friendly-single-armor-2-this-turn.md) | 无 | 使一个友方随从获得减伤 2 直到回合结束 | 1 |
| [trigger-friendly-single-divine-shield](trigger-friendly-single-divine-shield.md) | 无 | 使一个友方随从获得护盾 | 1 |
| [trigger-friendly-single-growth-once](trigger-friendly-single-growth-once.md) | 无 | 使一个友方随从获得一次成长 | 1 |
| [trigger-friendly-single-plus-0-plus-1-this-turn](trigger-friendly-single-plus-0-plus-1-this-turn.md) | 无 | 使一个友方随从获得 +0/+1 直到回合结束 | 1 |
| [trigger-friendly-single-plus-0-plus-2](trigger-friendly-single-plus-0-plus-2.md) | 无 | 使一个友方随从获得 +0/+2 | 1 |
| [trigger-friendly-single-plus-1-plus-1-this-turn](trigger-friendly-single-plus-1-plus-1-this-turn.md) | 无 | 使一个友方随从获得 +1/+1 直到回合结束 | 1 |
| [trigger-friendly-single-plus-2-plus-2](trigger-friendly-single-plus-2-plus-2.md) | 无 | 使一个友方随从获得 +2/+2 | 1 |
| [trigger-friendly-single-poison-this-turn](trigger-friendly-single-poison-this-turn.md) | 无 | 使一个友方随从获得剧毒直到回合结束 | 1 |
| [trigger-friendly-single-reborn](trigger-friendly-single-reborn.md) | 无 | 使一个友方随从获得复生 | 1 |
| [trigger-friendly-single-stealth-this-turn](trigger-friendly-single-stealth-this-turn.md) | 无 | 使一个友方随从获得潜行直到回合结束 | 2 |
| [trigger-friendly-taunt-until-turn-end](trigger-friendly-taunt-until-turn-end.md) | 无 | 使一个友方随从获得嘲讽直到回合结束 | 2 |
| [trigger-fungus-single-plus-0-plus-3](trigger-fungus-single-plus-0-plus-3.md) | 无 | 使一个友方真菌随从获得 +0/+3 | 1 |
| [trigger-insect-all-plus-1-plus-1](trigger-insect-all-plus-1-plus-1.md) | 无 | 使你的昆虫纲随从获得 +1/+1 | 1 |
| [trigger-insect-single-reborn](trigger-insect-single-reborn.md) | 无 | 使一个友方昆虫纲随从获得复生 | 1 |
| [trigger-machine-all-growth-once](trigger-machine-all-growth-once.md) | 无 | 使你的机械随从获得一次成长 | 1 |
| [trigger-machine-all-plus-1-plus-1](trigger-machine-all-plus-1-plus-1.md) | 无 | 使你的机械随从获得 +1/+1 | 1 |
| [trigger-machine-single-reborn](trigger-machine-single-reborn.md) | 无 | 使一个友方机械随从获得复生 | 1 |
| [trigger-mammal-all-growth-once](trigger-mammal-all-growth-once.md) | 无 | 使你的哺乳纲随从获得一次成长 | 1 |
| [trigger-mammal-all-plus-1-plus-1](trigger-mammal-all-plus-1-plus-1.md) | 无 | 使你的哺乳纲随从获得 +1/+1 | 1 |
| [trigger-mammal-all-plus-2-attack](trigger-mammal-all-plus-2-attack.md) | 无 | 使你的哺乳纲随从获得 +2 攻击力 | 1 |
| [trigger-mech-optimal-attack-this-turn](trigger-mech-optimal-attack-this-turn.md) | 无 | 你的机械随从获得无视嘲讽直到回合结束 | 1 |
| [trigger-minion-windfury-this-turn](trigger-minion-windfury-this-turn.md) | 无 | 使一个随从本回合获得连击 | 2 |
| [trigger-other-friendly-humans-plus-1-attack](trigger-other-friendly-humans-plus-1-attack.md) | 无 | 使你的其他人科随从各获得 +1 攻击力 | 1 |
| [trigger-plant-all-breed-2](trigger-plant-all-breed-2.md) | 无 | 使你的植物随从获得繁衍 2 | 1 |
| [trigger-plant-all-growth-twice](trigger-plant-all-growth-twice.md) | 无 | 使你的植物随从获得两次成长 | 1 |
| [trigger-play-effect-friendly-plus-0-plus-1](trigger-play-effect-friendly-plus-0-plus-1.md) | 无 | 每当你使用一张效果牌，使一个友方随从获得 +0/+1 | 1 |
| [trigger-protozoa-single-growth-twice](trigger-protozoa-single-growth-twice.md) | 无 | 使一个友方原生生物界随从获得两次成长 | 1 |
| [trigger-protozoa-single-taunt-this-turn](trigger-protozoa-single-taunt-this-turn.md) | 无 | 使一个友方原生生物界随从获得嘲讽直到回合结束 | 1 |
| [trigger-refresh-shop-free](trigger-refresh-shop-free.md) | 无 | 刷新商店 | 1 |
| [trigger-reptile-all-plus-0-plus-3](trigger-reptile-all-plus-0-plus-3.md) | 无 | 使你的爬行纲随从获得 +0/+3 | 1 |
| [trigger-reptile-single-armor-2-this-turn](trigger-reptile-single-armor-2-this-turn.md) | 无 | 使一个友方爬行纲随从获得减伤 2 直到回合结束 | 1 |
| [trigger-reptile-single-poison-this-turn](trigger-reptile-single-poison-this-turn.md) | 无 | 使一个友方爬行纲随从获得剧毒直到回合结束 | 1 |
| [trigger-summon-1-neutral-token-2-2](trigger-summon-1-neutral-token-2-2.md) | 无 | 召唤一个 2/2 的随从 | 1 |
| [trigger-summon-2-amoeba-larva-3-3](trigger-summon-2-amoeba-larva-3-3.md) | 无 | 召唤两个 3/3 的变形虫幼体 | 1 |
| [trigger-summon-2-neutral-token-2-2](trigger-summon-2-neutral-token-2-2.md) | 无 | 召唤两个 2/2 的随从 | 1 |
| [trigger-summon-2-spores-2-2](trigger-summon-2-spores-2-2.md) | 无 | 召唤两个 2/2 的孢子 | 1 |
| [trigger-summon-2-worker-bees](trigger-summon-2-worker-bees.md) | 无 | 召唤两个 2/2 的工蜂 | 1 |
| [trigger-turn-start-self-plus-0-plus-1](trigger-turn-start-self-plus-0-plus-1.md) | 无 | 回合开始时获得 +0/+1 | 1 |
| [trigger-virus-single-plus-1-plus-2](trigger-virus-single-plus-1-plus-2.md) | 无 | 使一个友方病毒随从获得 +1/+2 | 1 |
| [universal-manuscript](universal-manuscript.md) | 无 | 复制一张你手牌中的随从牌 | 1 |
| [virus-growth-once](virus-growth-once.md) | 无 | 使一个友方病毒随从获得一次成长 | 1 |
