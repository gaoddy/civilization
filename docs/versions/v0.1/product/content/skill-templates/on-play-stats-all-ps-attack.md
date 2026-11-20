---
id: on-play-stats-all-ps-attack
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:all:playstyle={playstyle}
arg.playstyle: playstyle
arg.attack: int
params: attack={attack},duration=permanent
text: 使你的{playstyle}随从获得 +{attack} 攻击力
---
