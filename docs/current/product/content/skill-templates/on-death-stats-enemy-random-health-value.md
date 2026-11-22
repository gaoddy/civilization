---
id: on-death-stats-enemy-random-health-value
kind: effect
trigger: OnDeath
op: ModifyStats
selector: enemy:random
arg.health: int
params: health={health},duration=combat
text: 亡语：使随机一个敌方随从获得 {health} 生命值
---
