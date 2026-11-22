---
id: conditional-taxon-self-attack
kind: effect
trigger: ConditionalAura
op: ModifyStats
selector: self
arg.attack: int
arg.taxon: taxon
arg.count: int
params: attack={attack},duration=permanent,condition=count:friendly:taxon={taxon}:>={count}
text: 若你控制至少 {count} 个{taxon}随从，获得 +{attack} 攻击力
text_golden: 若你控制至少 {count} 个{taxon}随从，获得 +{attack.x2} 攻击力
---
