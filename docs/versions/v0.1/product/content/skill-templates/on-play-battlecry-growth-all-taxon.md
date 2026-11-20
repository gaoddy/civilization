---
id: on-play-battlecry-growth-all-taxon
kind: effect
trigger: OnPlay
op: Growth
selector: friendly:all:taxon={taxon}
arg.taxon: taxon
arg.times: int 1..3
params: times={times}
text: 战吼：使你的所有{taxon}随从获得{times.cn}次成长
---
