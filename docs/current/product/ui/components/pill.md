# 数值胶囊

> 状态：已实现
> 入口：`UIControls.pill_label(text, variant, font_token)`，`client/ui/controls.gd`

## 用途

对局的数值与状态标签：招募倒计时、战斗状态、河道金币、幽灵与名次标记。

## 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `text` | `String` | 单行文本，不换行 |
| `variant` | `String` | `neutral`（默认）、`accent`、`success`、`danger`、`info`、`plain` |
| `font_token` | `String` | 字体令牌，默认 `font-body` |

## 变体

| 变体 | 底色与描边 | 文本色 |
| --- | --- | --- |
| `neutral` | `color-surface-raised`（55% 不透明）+ `color-border` | `color-text` |
| `accent` | 金色 16% 底 + 金色 60% 描边 | `color-accent` |
| `success` | 成功色 16% 底 + 60% 描边 | `color-success` |
| `danger` | 危险色 16% 底 + 60% 描边 | `color-danger` |
| `info` | 信息色 16% 底 + 60% 描边 | `color-info` |
| `plain` | 透明 | `color-text` |

## 表面

除 `plain` 外，胶囊在创建时叠一层 [框体材质](frame-material.md) 的数字底板：底板铺在胶囊底色之下，文字与边框仍在上层。底板颜色取变体色（`neutral` 取 `color-border`），胶囊底色因此压到 55% 不透明以内，底板才透得出来。

## 事件

无。

## 禁用场景

- 不用于长文本或多行说明。
- 不把多个语义塞进一个胶囊；一个胶囊只表达一个数值或一个状态。

## 使用者

- 招募河道的 `RecruitTimerLabel` 倒计时与 `RecruitGoldTile` 金币、战斗河道的 `CombatStatusPill` 与 `CombatGoldTile` 金币。
- 战斗对手身份块的幽灵与名次标记。
- 图标按钮与英雄技能的 `CostBadge` / `UsesBadge` 由 `GameIconButton` 另行绘制。卡面顶栏的时代象形与费用旋钮由 `CardTopRow` 直接绘制，不使用胶囊底。
