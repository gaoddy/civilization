# 按钮

> 状态：已实现
> 入口：`UIControls.styled_button(text, variant, min_height)`，`client/ui/controls.gd`

## 用途

统一按钮的描边、底色、文本色与禁用态，避免每个页面各自覆盖主题。

## 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `text` | `String` | 按钮文本，支持 `\n` 换行 |
| `variant` | `String` | `default`、`primary`、`danger`、`ghost`、`slot`、`micro` |
| `min_height` | `int` | 最小高度，默认 30 |

## 变体

| 变体 | 用途 |
| --- | --- |
| `default` | 常规操作：刷新、升时代、冻结、英雄技能、取消 |
| `primary` | 页面主操作：结束招募 |
| `danger` | 破坏性操作，当前页面未使用 |
| `ghost` | 无底色的小操作，悬停时出现底色 |
| `slot` | 商店槽与手牌这类卡牌式大按钮 |
| `micro` | 预留：密度更高的卡内次要操作；当前页面未使用（棋盘卡换位与出售改为拖拽） |

## 状态与事件

- 状态：默认、悬停、按下、聚焦、禁用，规则见 [interactions.md](../interactions.md#1-控件状态)。
- 事件：标准 `Button.pressed`，由页面连接意图发送。
- 禁用时必须同时给出 `tooltip_text` 说明原因。
- `apply_button_style` 会给按钮叠一层 [框体材质](frame-material.md)，按钮因此默认带引导流光；`PillButton` 走同一规则。
- 页面需要收窄流光范围时，用 `FrameMaterial.set_guide_preference(button, false)` 关掉个别按钮；偏好随材质重扫保留。大厅只保留「开始匹配」的流光，开始匹配后移除。

## 禁用场景

- 不在页面里直接改按钮的 `StyleBox` 或字体颜色；能力不足时扩展本组件的变体。

## 使用者

- 登录、大厅、房间、对局、结算页。
