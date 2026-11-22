---
id: on-play-battlecry-growth-all-ps-other
kind: effect
trigger: OnPlay
op: Growth
selector: friendly:all:playstyle={playstyle}:other=true
arg.playstyle: playstyle
arg.times: int 1..3
params: times={times}
text: 战吼：使你的其他{playstyle}随从获得{times.cn}次成长
---
