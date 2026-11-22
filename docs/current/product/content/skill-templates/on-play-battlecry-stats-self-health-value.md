---
id: on-play-battlecry-stats-self-health-value
kind: effect
trigger: OnPlay
op: ModifyStats
selector: self
arg.health: int
params: health={health},duration=permanent
text: 战吼：获得 +{health} 生命值
---
