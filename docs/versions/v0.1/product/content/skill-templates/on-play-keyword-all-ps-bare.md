---
id: on-play-keyword-all-ps-bare
kind: effect
trigger: OnPlayEffect
op: GrantKeyword
selector: friendly:all:playstyle={playstyle}
arg.playstyle: playstyle
arg.keyword: keyword
arg.duration: duration
params: keyword={keyword.name},value={keyword.level},duration={duration}
text: 你的{playstyle}随从获得{keyword}{duration.suffix}
---
