---
id: activate-keyword-single-taxon
kind: effect
trigger: OnActivate
op: GrantKeyword
selector: friendly:single:taxon={taxon}
arg.taxon: taxon
arg.keyword: keyword
arg.duration: duration
params: keyword={keyword.name},value={keyword.level},duration={duration}
text: 使一个友方{taxon}随从获得{keyword}{duration.suffix}
---
