---
id: aura-all-taxon-both
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:taxon={taxon}
arg.taxon: taxon
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 你的{taxon}随从获得 +{attack}/+{health}
---
