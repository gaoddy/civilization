---
id: on-play-battlecry-stats-single-health
kind: effect
trigger: OnPlay
op: ModifyStats
selector: friendly:single
arg.health: int
params: health={health},duration=permanent
text: 战吼：使一个友方随从获得 +0/+{health}
---
