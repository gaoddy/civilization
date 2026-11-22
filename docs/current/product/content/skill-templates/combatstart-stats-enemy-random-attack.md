---
id: combatstart-stats-enemy-random-attack
kind: effect
trigger: OnCombatStart
op: ModifyStats
selector: enemy:random
arg.attack: int
params: attack={attack},duration=combat
text: 战斗开始时，使一个随机敌方随从获得 {attack} 攻击力直到回合结束
---
