---
id: on-play-stats-all-both
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:all
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 使你的随从获得 +{attack}/+{health}
---
