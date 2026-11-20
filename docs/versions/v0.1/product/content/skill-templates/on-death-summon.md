---
id: on-death-summon
trigger: OnDeath
op: Summon
selector: self
arg.token: token
arg.count: count 1..3 default=1
params: token={token},count={count}
text: 亡语：召唤{count.cn}个 {token.stats} 的{token.name}
---

# 亡语召唤

渲染「亡语：召唤 N 个 A/B 的 <名称>」，`token` 可以是 Token 注册表 id 或卡牌 id。
