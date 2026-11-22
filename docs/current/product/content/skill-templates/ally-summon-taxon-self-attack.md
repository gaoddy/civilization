---
id: ally-summon-taxon-self-attack
kind: effect
trigger: OnAllySummon
op: ModifyStats
selector: self
arg.attack: int
arg.taxon: taxon
params: attack={attack},duration=permanent,when=taxon={taxon}
text: 每当一个友方{taxon}随从进入战场，获得 +{attack} 攻击力
---
