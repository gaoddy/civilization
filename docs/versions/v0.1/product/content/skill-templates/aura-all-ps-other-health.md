---
id: aura-all-ps-other-health
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:playstyle={playstyle}:other=true
arg.playstyle: playstyle
arg.health: int
params: health={health},duration=permanent
text: 你的其他{playstyle}随从获得 +0/+{health}
---
