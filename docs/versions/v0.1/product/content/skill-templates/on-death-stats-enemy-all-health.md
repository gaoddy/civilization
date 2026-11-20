---
id: on-death-stats-enemy-all-health
kind: effect
trigger: OnDeath
op: ModifyStats
selector: enemy:all
arg.health: int
params: health={health},duration=combat
text: 亡语：使敌方所有随从获得 {health} 生命
---
