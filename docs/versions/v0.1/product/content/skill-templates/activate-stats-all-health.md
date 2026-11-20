---
id: activate-stats-all-health
kind: effect
trigger: OnActivate
op: ModifyStats
selector: friendly:all
arg.health: int
params: health={health},duration=permanent
text: 使你的所有随从获得 +0/+{health}
---
