---
id: combatstart-contagion-leftmost
kind: effect
trigger: OnCombatStart
op: GrantKeyword
selector: enemy:leftmost
arg.level: int
params: keyword=contagion,value={level},duration=permanent
text: 战斗开始时使敌方最左侧随从获得 {level} 层传染
---
