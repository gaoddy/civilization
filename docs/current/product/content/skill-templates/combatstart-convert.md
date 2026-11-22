---
id: combatstart-convert
kind: effect
trigger: OnCombatStart
op: Convert
selector: enemy:single
arg.threshold: int
arg.token: token
arg.count: count 1..4
params: threshold={threshold},token={token},count={count}
text: 转化：战斗开始时，若一个敌方随从有 {threshold} 层传染，消灭它并召唤{count.cn}个 {token.stats} 的{token.name}
text_golden: 转化：战斗开始时，若一个敌方随从有 {threshold} 层传染，消灭它并召唤{count.x2}个 {token.stats} 的{token.name}
---
