---
id: on-death-summon-sow
kind: effect
trigger: OnDeath
op: Summon
selector: self
arg.token: token
arg.count: count 1..4
params: token={token},count={count}
text: 播种：死亡时召唤{count.cn}个 {token.stats} 的{token.name}
---
