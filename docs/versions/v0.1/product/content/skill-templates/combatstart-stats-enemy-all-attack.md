---
id: combatstart-stats-enemy-all-attack
kind: effect
trigger: OnCombatStart
op: ModifyStats
selector: enemy:all
arg.attack: int
params: attack={attack},duration=combat
text: 战斗开始时，使敌方所有随从获得 {attack} 攻击力
---
