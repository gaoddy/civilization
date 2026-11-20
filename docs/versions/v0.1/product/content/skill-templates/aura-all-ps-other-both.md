---
id: aura-all-ps-other-both
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:playstyle={playstyle}:other=true
arg.playstyle: playstyle
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 你的其他{playstyle}随从获得 +{attack}/+{health}
---
