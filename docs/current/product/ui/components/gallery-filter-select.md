# 图鉴筛选下拉

> 状态：已实现
> 入口：`GalleryFilterSelect.new(node_name, dimension)`，`client/ui/gallery_filter_select.gd`（`class_name GalleryFilterSelect`）

## 用途

图鉴头部的筛选触发器：显示「维度 · 当前值 ▾」，点击后由页面打开对应的筛选面板。三处实例分别对应时代、玩法与类型。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `set_state(value, active)` | `void` | 更新当前值文字与激活态；`active` 为真表示已选非「全部」的条目 |

- `_init(node_name, dimension)`：`node_name` 为节点名（如 `GalleryEraSelect`），`dimension` 为维度展示名（如「时代」）。
- 文字格式固定为 `"%s · %s ▾"`，例如「时代 · 全部 ▾」「玩法 · 哺乳纲 ▾」。
- 使用 `default` 按钮样式，高度取工具行次级按钮高度（`toolbar-button-height`，桌面 22、触控端不低于次要操作命中 32），`font-body-sm`。
- 未激活时文字为 `color-text`；已选非「全部」的条目时转为 `color-accent`。

## 变体与状态

- 三处实例：`GalleryEraSelect`（时代）、`GalleryPlaySelect`（玩法）、`GalleryKindSelect`（类型）。
- 组件只呈现当前值与颜色，不保存筛选条件，也不打开面板；点击由页面接到 `_open_filter`。
- 英雄视图由页面隐藏时代与类型下拉，玩法下拉改为英雄玩法。

## 禁用场景

- 不计算计数，不渲染条目，不决定选中的是「全部」还是具体值。

## 使用者

- [卡牌图鉴](card-gallery.md)。
