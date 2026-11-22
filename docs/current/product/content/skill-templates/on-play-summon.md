---
id: on-play-summon
kind: effect
trigger: OnPlayEffect
op: Summon
selector: self
arg.token: token
arg.count: count 1..4
params: token={token},count={count}
text: 召唤{count.cn}个 {token.stats} 的{token.name}
---
