# 图鉴视图切换

> 状态：已实现
> 入口：`GalleryModeBar.new()`，`client/scenes/gallery_mode_bar.gd`（`class_name GalleryModeBar`）

## 用途

图鉴标题右侧的「随从 / 英雄」两态切换，决定顶部筛选栏、左侧目录与网格使用哪套卡池与筛选。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `mode_selected(mode)` | `signal(String)` | 选中视图；`minion` 为随从，`hero` 为英雄 |
| `set_state(mode, hero_available)` | `void` | 同步当前选中项与英雄项可用性 |

- 节点名为 `GalleryModeBar`，按钮为 `ModeMinion` 与 `ModeHero`。
- 按钮使用 `default` 未选中、`primary` 选中样式，高 30，`font-body-sm`；两项同处一个 `ButtonGroup`，始终有一项选中。

## 变体与状态

- 默认随从视图；每次打开图鉴由页面重置为随从。
- 英雄集合为空时 `ModeHero` 禁用，当前视图保持随从。
- 点击当前未选中的一项发出 `mode_selected`；点击当前项不重复发出视图变化。

## 禁用场景

- 不保存筛选、不计算计数、不决定卡池；这些都由 [卡牌图鉴](card-gallery.md) 负责。

## 使用者

- [卡牌图鉴](card-gallery.md)。
