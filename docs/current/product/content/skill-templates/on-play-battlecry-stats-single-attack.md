---
id: on-play-battlecry-stats-single-attack
kind: effect
trigger: OnPlay
op: ModifyStats
selector: friendly:single
arg.attack: int
params: attack={attack},duration=permanent
text: 战吼：使一个友方随从获得 +{attack} 攻击力
---
