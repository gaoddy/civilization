---
id: on-play-battlecry-assemble
kind: effect
trigger: OnPlay
op: Growth
selector: friendly:all:playstyle=machine:other=true
arg.times: int 1..3
params: times={times}
text: 装配 {times}：进入战场时，使你的其他机械随从各获得 {times} 次成长
---
