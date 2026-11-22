---
id: aura-school-perattack
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:playstyle={playstyle}:other=true
arg.playstyle: playstyle
arg.attack: int
arg.health: int
arg.perAttack: int
params: attack={attack},health={health},perAttack={perAttack},maxCount=6
text: 群聚：你的其他{playstyle}随从获得 +{attack}/+{health}；每有一个其他{playstyle}随从，再获得 +{perAttack} 攻击力
---
