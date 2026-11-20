---
id: combatstart-contagion-random
kind: effect
trigger: OnCombatStart
op: GrantKeyword
selector: enemy:random
arg.level: int
params: keyword=contagion,value={level},duration=permanent
text: 战斗开始时，使一个随机敌方随从获得 {level} 层传染
---
