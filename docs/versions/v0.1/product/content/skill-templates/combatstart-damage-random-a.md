---
id: combatstart-damage-random-a
kind: effect
trigger: OnCombatStart
op: DealDamage
selector: enemy:random
arg.amount: int
params: amount={amount}
text: 战斗开始时，对随机一个敌方随从造成 {amount} 点伤害
---
