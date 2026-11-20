---
id: on-play-stats-single-both
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:single
arg.attack: int
arg.health: int
arg.duration: duration
params: attack={attack},health={health},duration={duration}
text: 使一个友方随从获得 +{attack}/+{health}{duration.suffix}
---
