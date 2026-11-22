---
id: on-play-battlecry-stats-single-taxon-health
kind: effect
trigger: OnPlay
op: ModifyStats
selector: friendly:single:taxon={taxon}
arg.taxon: taxon
arg.health: int
params: health={health},duration=permanent
text: 战吼：使一个友方{taxon}随从获得 +0/+{health}
---
