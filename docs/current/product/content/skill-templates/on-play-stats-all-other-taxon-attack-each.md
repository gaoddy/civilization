---
id: on-play-stats-all-other-taxon-attack-each
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:all:taxon={taxon}:other=true
arg.taxon: taxon
arg.attack: int
params: attack={attack},duration=permanent
text: 使你的其他{taxon}随从各获得 +{attack} 攻击力
---
