---
id: on-play-battlecry-damage-random
kind: effect
trigger: OnCombatStart
op: DealDamage
selector: enemy:random
arg.amount: int
params: amount={amount}
text: 战吼：对随机一个敌方随从造成 {amount} 点伤害
---
