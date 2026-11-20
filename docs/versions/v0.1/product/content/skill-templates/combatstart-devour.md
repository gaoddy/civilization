---
id: combatstart-devour
kind: effect
trigger: OnCombatStart
op: Devour
selector: enemy:single
arg.value: int
params: value={value},duration=permanent
text: 吞噬：战斗开始时，消灭一个攻击力不高于 {value} 的敌方随从，在本场战斗内永久获得其攻击力与生命值
---
