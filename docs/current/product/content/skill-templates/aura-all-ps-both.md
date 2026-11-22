---
id: aura-all-ps-both
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:playstyle={playstyle}
arg.playstyle: playstyle
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 你的{playstyle}随从获得 +{attack}/+{health}
---
