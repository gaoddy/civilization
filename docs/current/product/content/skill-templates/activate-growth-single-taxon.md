---
id: activate-growth-single-taxon
kind: effect
trigger: OnActivate
op: Growth
selector: friendly:single:taxon={taxon}
arg.taxon: taxon
arg.times: int 1..3
params: times={times}
text: 使一个友方{taxon}随从获得{times.cn}次成长
---
