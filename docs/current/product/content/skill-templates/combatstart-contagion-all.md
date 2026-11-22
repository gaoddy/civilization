---
id: combatstart-contagion-all
kind: effect
trigger: OnCombatStart
op: GrantKeyword
selector: enemy:all
arg.level: int
params: keyword=contagion,value={level},duration=permanent
text: 战斗开始时，使敌方所有随从获得 {level} 层传染
---
