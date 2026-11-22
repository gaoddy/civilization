---
id: on-death-keyword-contagion-all
kind: effect
trigger: OnDeath
op: GrantKeyword
selector: enemy:all
arg.keyword: keyword
params: keyword={keyword.name},value={keyword.level},duration=permanent
text: 亡语：使敌方所有随从获得 {keyword.level} 层{keyword.name}
---
