---
id: ally-growth-target-health
kind: effect
trigger: OnAllyGrowth
op: ModifyStats
selector: target
arg.health: int
params: health={health},duration=permanent
text: 每当一个友方随从获得成长，它额外获得 +0/+{health}
---
