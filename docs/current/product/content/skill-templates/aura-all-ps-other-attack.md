---
id: aura-all-ps-other-attack
kind: effect
trigger: Aura
op: ModifyStats
selector: friendly:all:playstyle={playstyle}:other=true
arg.playstyle: playstyle
arg.attack: int
params: attack={attack},duration=permanent
text: 你的其他{playstyle}随从获得 +{attack} 攻击力
---
