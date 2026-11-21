# 图鉴顶部筛选栏

> 状态：已实现
> 入口：`GalleryFilterBar.new(node_name, button_prefix)`，`client/ui/gallery_filter_bar.gd`（`class_name GalleryFilterBar`）

## 用途

图鉴头部一行的横向单选筛选栏，供随从视图的时代章节与英雄视图的英雄分类共用。按钮在剩余宽度内居中，默认名称在上、计数在下。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `item_selected(id)` | `signal(String)` | 选中条目 id；空串表示「全部」 |
| `set_state(items, counts, selected)` | `void` | 用条目、计数与当前选中项重建按钮 |

- `items` 为 `{id, name}` 条目数组；`counts` 以 id 为键，空串为全部。
- `node_name` 决定节点名，`button_prefix` 决定按钮节点名：全部项为 `<prefix>All`，其余为 `<prefix>` + id 首字母大写。
- 计数由页面按当前筛选换算后传入；组件不自行推导分类或计数。

## 变体与状态

- 两处实例：`GalleryChapters`（前缀 `Chapter`）与 `GalleryHeroCategories`（前缀 `HeroCategory`）。
- 按钮为 `toggle_mode` 的 `ButtonGroup` 单选，`allow_unpress = false`，始终有一项选中。
- 选中项用 `primary` 样式（金色底、深色字）；未选中为 `default` 变体，悬停提亮描边。
- 计数为 0 且未选中时禁用并置灰；当前选中项即使计数为 0 也不禁用。
- 按钮高 44，`font-body-sm`；文字为两行，名称在上、计数在下，如「史前 / 63」「哺乳纲 / 7」。
- 按钮组在剩余宽度内水平居中，宽度不足时按 `HFlowContainer` 规则换行。

## 事件

- `item_selected(id)`：点击任意可用按钮时发出；`id = ""` 表示「全部」。

## 禁用场景

- 不做多选，不承载分类推导与计数计算。
- 英雄视图没有时代维度，由页面切换到英雄分类栏，不传零计数。

## 使用者

- [卡牌图鉴](card-gallery.md)。
