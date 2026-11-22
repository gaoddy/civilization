---
id: on-play-keyword-single-this-turn
kind: effect
trigger: OnPlayEffect
op: GrantKeyword
selector: friendly:single
arg.keyword: keyword
params: keyword={keyword.name},value={keyword.level},duration=combat
text: 使一个随从本回合获得{keyword}
---
