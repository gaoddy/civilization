---
id: friendly-death-taxon-self-both
kind: effect
trigger: OnFriendlyDeath
op: ModifyStats
selector: self
arg.attack: int
arg.health: int
arg.taxon: taxon
params: attack={attack},health={health},duration=permanent,when=taxon={taxon}
text: 每当一个友方{taxon}随从死亡，获得 +{attack}/+{health}
---
