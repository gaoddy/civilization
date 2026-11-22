---
id: aura-all-taxon-health
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:taxon={taxon}
arg.taxon: taxon
arg.health: int
params: health={health},duration=permanent
text: 你的{taxon}随从获得 +0/+{health}
---
