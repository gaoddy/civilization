---
id: on-play-stats-single-health
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:single
arg.health: int
arg.duration: duration
params: health={health},duration={duration}
text: 使一个友方随从获得 +0/+{health}{duration.suffix}
---
