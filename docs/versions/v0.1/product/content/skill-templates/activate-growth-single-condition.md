---
id: activate-growth-single-condition
kind: effect
trigger: OnActivate
op: Growth
selector: friendly:single:taxon={taxon}
arg.taxon: taxon
arg.count: int
params: times=1,timesIf=taxon:{taxon}:2,condition=count:friendly:taxon={taxon}:>={count}
text: 使一个友方{taxon}随从获得一次成长；若你控制至少 {count} 个{taxon}随从，改为两次
---
