---
id: on-play-stats-single-ps-both
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:single:playstyle={playstyle}
arg.playstyle: playstyle
arg.attack: int
arg.health: int
arg.duration: duration
params: attack={attack},health={health},duration={duration}
text: 使一个友方{playstyle}随从获得 +{attack}/+{health}{duration.suffix}
---
