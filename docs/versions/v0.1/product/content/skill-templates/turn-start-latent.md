---
id: turn-start-latent
kind: effect
trigger: OnTurnStart
op: Latent
selector: self
arg.amount: int
arg.threshold: int
arg.count: int
params: amount={amount},threshold={threshold},count={count}
text: 潜伏：回合开始时，每有 {amount} 层传染获得 +{amount} 攻击力；达到 {threshold} 层时对敌方所有随从造成 {count} 点伤害
---
