---
id: ally-growth-self-attack
kind: effect
trigger: OnAllyGrowth
op: ModifyStats
selector: self
arg.attack: int
params: attack={attack},duration=permanent
text: 每当一个友方随从获得成长，获得 +{attack} 攻击力
---
