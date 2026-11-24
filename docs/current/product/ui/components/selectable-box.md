# 可选项框

> 状态：已实现
> 入口：`SelectableBox`（`client/ui/selectable_box.gd`）

## 用途

承载任意内容的可选中容器：负责常态、悬停与选中三态描边，以及框内点击判定。选中值由调用方设置，本组件不读取业务状态，也不发送业务意图。

## 属性

| 属性 / 方法 | 类型 | 说明 |
| --- | --- | --- |
| `configure(base_style, outline, hover_enabled)` | `void` | 设置底板样式、描边变体与是否接悬停回调，每个实例只设置一次 |
| `content()` | `Control` | 内容容器；调用方把展示内容挂到这里 |
| `set_selected(value)` | `void` | 切换选中态 |
| `set_hovered(value)` | `void` | 切换悬停态 |
| `selected` | `bool` | 当前是否选中 |
| `activated` | `Signal` | 左键在框内按下并在阈值内松开时发出 |

- `base_style` 由调用方按场景令牌构建；组件只在其副本上叠加描边，不改动调用方样式。
- 内容区是普通 `Control`，不参与容器布局；调用方需要给内容设置锚点或明确尺寸。
- 内容容器与选中框都忽略鼠标，点击落在框本身。
- 组件不改动调用方的 `mouse_filter` 与鼠标指针形状：图鉴卡面单元使用 `PASS` + 手型，`HeroPickCard` 使用 `STOP` + 默认指针。

## 变体与状态

| 变体 / 状态 | 表现 |
| --- | --- |
| `Outline.INLINE` | 常态沿用底板描边（宽度由调用方决定）；悬停改 `color-border-strong`；选中改 2px `color-accent`，选中优先于悬停 |
| `Outline.OUTER` | 底板描边不变；悬停改 `color-border-strong`；选中在内容区外扩 `space-2xs` 显示 2px `color-accent` 的 `SelectedFrame`，圆角跟随底板 |
| 按下 | 不改变样式；位移超过 12 画布单位视为拖拽，不发出 `activated` |

- 悬停态只在 `hover_enabled` 为真时生效；触控端调用方传 `false`，保持无悬停态。

- `OUTER` 以内容区为基准外扩；内容需要铺满整卡时，调用方使用零内边距底板，选中框才会落在整卡之外。
- `OUTER` 的选中框四角半径逐角沿用底板样式：直角底板得到直角选中框，避免圆弧弯进铺满的内容区后被遮住。
- 指针为手型；组件不处理键盘焦点，选中与点击语义由调用方决定。

## 事件

- `activated`：左键按下与松开都在框内，且画布坐标位移不超过 `TAP_SLOP = 12` 时发出，不携带参数；业务标识由调用方绑定。

## 禁用场景

- 不承载选项数据、业务规则与数据请求，不接受任意颜色、字号与间距参数。
- 不用于只有文字的按钮组或一次性动作按钮：前者用 `Button` + `ButtonGroup`，后者用 `UIControls.styled_button`。

## 使用者

- [图鉴卡面单元](gallery-card-tile.md)：`Outline.INLINE`。
- [对局页选英雄覆盖层](../pages/game.md) 的候选英雄卡 `HeroPickCard`：`Outline.OUTER`。
- [对局页](../pages/game.md) 左栏座位行：`Outline.INLINE`，底板 2px 描边、`radius-md` 圆角、内容边距 2，头像铺满框内；只用悬停态，不调用 `set_selected`，座位行的 `activated` 不接行为，对战信息随悬停详情展示。
