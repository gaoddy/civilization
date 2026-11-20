---
id: on-play-battlecry-stats-all-health
kind: effect
trigger: OnPlay
op: ModifyStats
selector: friendly:all
arg.health: int
params: health={health},duration=permanent
text: 战吼：使你的所有随从获得 +0/+{health}
---
