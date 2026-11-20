# 图鉴视图切换

> 状态：已实现
> 入口：`GalleryModeBar.new()`，`client/scenes/gallery_mode_bar.gd`（`class_name GalleryModeBar`）

## 用途

图鉴标题右侧的视图切换：同一时刻只显示当前视图的一个按钮，点击切到另一视图，决定顶部筛选栏、左侧目录与网格使用哪套卡池与筛选。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `mode_selected(mode)` | `signal(String)` | 切到另一视图；`minion` 为随从，`hero` 为英雄 |
| `set_state(mode, hero_available)` | `void` | 同步按钮文字、当前视图与可用性 |

- 节点名为 `GalleryModeBar`，按钮为 `ModeToggle`。
- 按钮文字为当前视图名称：`ui.gallery.mode_minion`（随从）或 `ui.gallery.mode_hero`（英雄）；不并列显示两个选项。
- 按钮使用 `primary` 样式，高 30，`font-body-sm`。

## 变体与状态

- 默认随从视图；每次打开图鉴由页面重置为随从。
- 英雄集合为空时按钮禁用并显示「随从」，当前视图保持随从。
- 点击当前按钮不直接改变状态，而是发出另一视图；由页面更新筛选后再回写按钮文字。

## 禁用场景

- 不保存筛选、不计算计数、不决定卡池；这些都由 [卡牌图鉴](card-gallery.md) 负责。

## 使用者

- [卡牌图鉴](card-gallery.md)。
