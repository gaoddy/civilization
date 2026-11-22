---
id: aura-school-perhealth
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:playstyle={playstyle}:other=true
arg.playstyle: playstyle
arg.attack: int
arg.perHealth: int
params: attack={attack},perHealth={perHealth},maxCount=6,duration=permanent
text: 群聚：你的其他{playstyle}随从获得 +{attack}/+0；每有一个其他{playstyle}随从，再获得 +0/+{perHealth}
---
