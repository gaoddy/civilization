---
id: combatstart-detonate
kind: effect
trigger: OnCombatStart
op: Detonate
selector: enemy:single
arg.amount: int
params: amount={amount}
text: 引爆：战斗开始时，移除一个敌方随从的全部传染层，每层造成 {amount} 点伤害
---
