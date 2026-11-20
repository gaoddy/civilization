---
id: on-play-stats-all-health
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:all
arg.health: int
params: health={health},duration=permanent
text: 使你的随从获得 +0/+{health}
---
