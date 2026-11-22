---
id: on-play-keyword-single-ps
kind: effect
trigger: OnPlayEffect
op: GrantKeyword
selector: friendly:single:playstyle={playstyle}
arg.playstyle: playstyle
arg.keyword: keyword
arg.duration: duration
params: keyword={keyword.name},value={keyword.level},duration={duration}
text: 使一个友方{playstyle}随从获得{keyword}{duration.suffix}
---
