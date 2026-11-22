---
id: friendly-death-self-attack
kind: effect
trigger: OnFriendlyDeath
op: ModifyStats
selector: self
arg.attack: int
params: attack={attack},duration=permanent
text: 每当一个友方随从死亡，获得 +{attack} 攻击力
---
