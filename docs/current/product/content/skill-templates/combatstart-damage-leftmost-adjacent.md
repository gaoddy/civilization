---
id: combatstart-damage-leftmost-adjacent
kind: effect
trigger: OnCombatStart
op: DealDamage
selector: enemy:leftmost+adjacent
arg.amount: int
params: amount={amount}
text: 战斗开始时，对敌方最左侧随从及其相邻随从造成 {amount} 点伤害
---
