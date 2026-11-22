---
id: ally-evolve-self-stats
kind: effect
trigger: OnAllyEvolve
op: ModifyStats
selector: self
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 每当一个友方随从进化，获得 +{attack}/+{health}
---
