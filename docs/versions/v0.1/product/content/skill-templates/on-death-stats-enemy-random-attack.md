---
id: on-death-stats-enemy-random-attack
kind: effect
trigger: OnDeath
op: ModifyStats
selector: enemy:random
arg.attack: int
params: attack={attack},duration=combat
text: 亡语：使随机一个敌方随从获得 {attack} 攻击力
---
