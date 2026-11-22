---
id: on-play-battlecry-summon
kind: effect
trigger: OnPlay
op: Summon
selector: self
arg.token: token
arg.count: count 1..4
params: token={token},count={count}
text: 战吼：召唤{count.cn}个 {token.stats} 的{token.name}
---
