---
id: turn-start-self-health
kind: effect
trigger: OnTurnStart
op: ModifyStats
selector: self
arg.health: int
params: health={health},duration=permanent
text: 回合开始时获得 +0/+{health}
---
