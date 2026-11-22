---
id: aura-all-taxon-other-attack
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:taxon={taxon}:other=true
arg.taxon: taxon
arg.attack: int
params: attack={attack},duration=permanent
text: 你的其他{taxon}随从获得 +{attack} 攻击力
---
