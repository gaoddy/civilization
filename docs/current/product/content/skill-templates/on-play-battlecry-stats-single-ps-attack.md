---
id: on-play-battlecry-stats-single-ps-attack
kind: effect
trigger: OnPlay
op: ModifyStats
selector: friendly:single:playstyle={playstyle}
arg.playstyle: playstyle
arg.attack: int
params: attack={attack},duration=permanent
text: 战吼：使一个友方{playstyle}随从获得 +{attack} 攻击力
---
