---
id: aura-all-ps-health
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:playstyle={playstyle}
arg.playstyle: playstyle
arg.health: int
params: health={health},duration=permanent
text: 你的{playstyle}随从获得 +0/+{health}
---
