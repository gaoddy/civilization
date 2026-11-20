---
id: on-death-stats-enemy-all-taxon-health
kind: effect
trigger: OnDeath
op: ModifyStats
selector: enemy:all:taxon={taxon}
arg.taxon: taxon
arg.health: int
params: health={health},duration=combat
text: 亡语：使敌方所有{taxon}随从获得 {health} 生命
---
