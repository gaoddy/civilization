# 数值胶囊

> 状态：已实现
> 入口：`UIControls.pill_label(text, variant, font_token)`，`client/ui/controls.gd`

## 用途

对局的数值与状态标签：招募倒计时、战斗状态、河道金币、幽灵与名次标记，以及商店卡的费用角标。

## 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `text` | `String` | 单行文本，不换行 |
| `variant` | `String` | `neutral`（默认）、`accent`、`success`、`danger`、`info`、`plain` |
| `font_token` | `String` | 字体令牌，默认 `font-body` |

## 变体

| 变体 | 底色与描边 | 文本色 |
| --- | --- | --- |
| `neutral` | `color-surface-raised` + `color-border` | `color-text` |
| `accent` | 金色 16% 底 + 金色 60% 描边 | `color-accent` |
| `success` | 成功色 16% 底 + 60% 描边 | `color-success` |
| `danger` | 危险色 16% 底 + 60% 描边 | `color-danger` |
| `info` | 信息色 16% 底 + 60% 描边 | `color-info` |
| `plain` | 透明 | `color-text` |

## 事件

无。

## 禁用场景

- 不用于长文本或多行说明。
- 不把多个语义塞进一个胶囊；一个胶囊只表达一个数值或一个状态。

## 使用者

- 招募河道的 `RecruitTimerLabel` 倒计时与 `RecruitGoldTile` 金币、战斗河道的 `CombatStatusPill` 与 `CombatGoldTile` 金币。
- 战斗对手身份块的幽灵与名次标记。
- 商店卡的 `CostBadge`；图标按钮与英雄技能的 `CostBadge` / `UsesBadge` 由 `GameIconButton` 另行绘制。卡面右上角的时代改为无框 `EraGlyph`，不再使用胶囊角标。
