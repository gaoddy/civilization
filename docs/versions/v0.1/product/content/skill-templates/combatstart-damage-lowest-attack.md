---
id: combatstart-damage-lowest-attack
kind: effect
trigger: OnCombatStart
op: DealDamage
selector: enemy:lowestAttack
arg.amount: int
params: amount={amount}
text: 战斗开始时，对攻击力最低的敌方随从造成 {amount} 点伤害
---
