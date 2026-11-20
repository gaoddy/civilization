---
id: combatstart-stats-enemy-top-health
kind: effect
trigger: OnCombatStart
op: ModifyStats
selector: enemy:topAttack
arg.health: int
params: health={health},duration=combat
text: 战斗开始时，使敌方攻击力最高的随从获得 -0/{health}
---
