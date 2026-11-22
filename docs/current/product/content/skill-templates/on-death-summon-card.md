---
id: on-death-summon-card
kind: effect
trigger: OnDeath
op: Summon
selector: self
arg.card: card
arg.count: count 1..4
params: card={card},count={count}
text: 亡语：召唤{count.cn}个 {card.stats} 的{card.name}
---
