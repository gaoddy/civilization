---
id: conditional-taxon-self-both
kind: effect
trigger: ConditionalAura
op: ModifyStats
selector: self
arg.attack: int
arg.health: int
arg.taxon: taxon
arg.count: int
params: attack={attack},health={health},duration=permanent,condition=count:friendly:taxon={taxon}:>={count}
text: 若你控制至少 {count} 个{taxon}随从，获得 +{attack}/+{health}
---
