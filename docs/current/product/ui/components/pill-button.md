# 胶囊按钮

> 状态：已实现
> 入口：`PillButton`，`client/ui/pill_button.gd`
> 使用：`PillButton.new(text, variant)`

## 1. 用途

全局共享的圆角药丸按钮：对局右上角工具组与状态模态的「知道了」。单行文本，只发出继承的 `pressed` 信号，不构造业务意图。

## 2. 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `text` | `String` | 单行按钮文案 |
| `variant` | `String` | `neutral`（默认）、`primary`、`danger` |
| `disabled` | `bool` | 继承 `Button`；禁用时底色转 `color-surface`、文字转 `color-text-muted` |

## 3. 样式与令牌

- 圆角 `radius-pill`；左右内边距 `space-lg`，上下 `space-2xs`；描边 1px。
- 高度 `pill-button-height`（26），iPhone 取 `pill-button-height-compact`（24）；触控端不低于 `hit_small()`。
- 字号 `font-body-sm`。
- 视觉由 `UITheme.pill_button_style(variant, state)` 提供：`normal` / `hover` / `pressed` / `focus` / `disabled`。
- `neutral` 使用 `color-surface-raised` 底与 `color-border` 描边，悬停提亮并转 `color-border-strong`；`primary` 使用 `color-accent-strong` 底与 `color-bg` 文字；`danger` 使用 `color-danger` 底。

## 4. 事件

- 继承 `pressed`；不新增信号。

## 5. 禁用场景

- 不用于多行文案；不把多个语义塞进一枚按钮。
- 需要图标、费用或次数角标时使用 `GameIconButton`。

## 6. 使用者

- `GameToolButtons` 的图鉴、提示与退出对局。
- `GameStatusModal` 的「知道了」。

## 7. 验证

- `make client-check`、`make client-smoke`：`ui_smoke_side._check_tool_buttons` 断言三枚入口均为 `PillButton`，断线时提示与退出对局禁用。

## 8. 关联

- 按钮变体：[button.md](button.md)
- 数值胶囊：[pill.md](pill.md)
- 对局页：[../pages/game.md](../pages/game.md)
