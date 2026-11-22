---
id: activate-growth-single-playstyle
kind: effect
trigger: OnActivate
op: Growth
selector: friendly:single:playstyle={playstyle}
arg.playstyle: playstyle
arg.times: int 1..3
params: times={times}
text: 使一个友方{playstyle}随从获得{times.cn}次成长
---
