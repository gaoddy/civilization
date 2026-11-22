---
id: on-play-stats-single-ps-health
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:single:playstyle={playstyle}
arg.playstyle: playstyle
arg.health: int
arg.duration: duration
params: health={health},duration={duration}
text: 使一个友方{playstyle}随从获得 +0/+{health}{duration.suffix}
---
