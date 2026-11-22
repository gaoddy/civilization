---
id: activate-seven-voyages
kind: effect
trigger: OnActivate
op: SevenVoyages
selector: self
arg.turns_max: int 1..7 default=7
params: turns_max={turns_max}
text: 随机确定航程 n（1–{turns_max}）。此后每个航行回合结束时按当回合战绩结算：胜利或平局取棋盘占位数与空位数中的较大者，失败取较小者，累计至航程结束；航程结束后的招募阶段开始时，累计金币一次性到账。
---
