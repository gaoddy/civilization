# 图鉴视图切换

> 状态：已实现
> 入口：`GalleryModeBar.new()`，`client/ui/gallery_mode_bar.gd`（`class_name GalleryModeBar`）

## 用途

图鉴头部的视图分段：并排显示「随从」「英雄」两个按钮，当前视图高亮，点击切到另一视图，决定筛选下拉与网格使用哪套卡池与筛选。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `mode_selected(mode)` | `signal(String)` | 点击某一分段时发出；`minion` 为随从，`hero` 为英雄 |
| `set_state(mode, hero_available)` | `void` | 同步分段选中态、样式与英雄分段可用性 |

- 节点名为 `GalleryModeBar`，按钮为 `ModeMinion` 与 `ModeHero`。
- 两枚按钮共用一个 `ButtonGroup`，`allow_unpress = false`，始终有一段选中。
- 两枚按钮为全局胶囊按钮（`PillButton`），显式高度 56×`toolbar-button-height`（26，桌面与触控端一致，命中与视觉一致），`font-body-sm`；选中段 `primary` 变体（金色底、深色字），另一段 `neutral` 变体。

## 变体与状态

- 默认随从视图；每次打开图鉴由页面重置为随从。
- 英雄集合为空时英雄分段禁用，当前视图保持随从。
- 点击当前分段时页面忽略重复选择；由页面先更新筛选，再回写分段状态。

## 禁用场景

- 不保存筛选、不计算计数、不决定卡池；这些都由 [卡牌图鉴](card-gallery.md) 负责。

## 使用者

- [卡牌图鉴](card-gallery.md)。
