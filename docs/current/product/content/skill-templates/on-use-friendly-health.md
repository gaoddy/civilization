---
id: on-use-friendly-health
kind: effect
trigger: OnUse
op: ModifyStats
selector: friendly:single
arg.health: int
params: health={health},duration=permanent
text: 每当你使用一张效果牌，使一个友方随从获得 +0/+{health}
---
