---
id: on-play-stats-all-ps-both
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:all:playstyle={playstyle}
arg.playstyle: playstyle
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 使你的{playstyle}随从获得 +{attack}/+{health}
---
