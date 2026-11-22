---
id: on-play-battlecry-stats-single-ps-health
kind: effect
trigger: OnPlay
op: ModifyStats
selector: friendly:single:playstyle={playstyle}
arg.playstyle: playstyle
arg.health: int
params: health={health},duration=permanent
text: 战吼：使一个友方{playstyle}随从获得 +0/+{health}
---
