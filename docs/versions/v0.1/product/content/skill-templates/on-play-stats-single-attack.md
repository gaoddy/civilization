---
id: on-play-stats-single-attack
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:single
arg.attack: int
arg.duration: duration
params: attack={attack},duration={duration}
text: 使一个友方随从获得 +{attack} 攻击力{duration.suffix}
---
