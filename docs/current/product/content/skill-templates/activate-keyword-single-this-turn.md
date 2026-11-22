---
id: activate-keyword-single-this-turn
kind: effect
trigger: OnActivate
op: GrantKeyword
selector: friendly:single
arg.keyword: keyword
params: keyword={keyword.name},value={keyword.level},duration=combat
text: 使一个友方随从本回合获得{keyword}
---
