# 图鉴章节栏

> 状态：已实现
> 入口：`GalleryChapterBar.new()`，`client/scenes/gallery_chapter_bar.gd`（`class_name GalleryChapterBar`）

## 用途

图鉴头部一行的时代筛选：「全部」与 1–7 时代的单选按钮横排在标题与返回按钮之间居中，每个按钮名称在上、计数在下；与标题、返回按钮同处一行。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `chapter_selected` | `signal(int)` | 选中时代序号；`0` 表示「全部」 |
| `set_state(counts, selected)` | `void` | 用计数与当前选中项重建按钮 |

- `counts` 以时代序号为键，`0` 为全部；由页面按另一维筛选换算后传入。
- `selected` 为当前时代序号。

## 变体与状态

- 按钮为 `toggle_mode` 的 `ButtonGroup` 单选，`allow_unpress = false`，始终有一项选中。
- 选中项用 `primary` 样式（金色底、深色字）；未选中为 `default` 变体，悬停提亮描边。
- 计数为 0 且未选中时禁用并置灰；当前选中项即使计数为 0 也不禁用。
- 按钮高 44，`font-body-sm`；文字为两行，名称在上、计数在下，如「史前 / 63」「全部 / 432」。
- 按钮组在头部剩余宽度内水平居中，宽度不足时按 `HFlowContainer` 规则换行。

## 事件

- `chapter_selected(era)`：点击任意可用按钮时发出。

## 禁用场景

- 不做多选，不承载分类筛选（分类由 [图鉴目录](gallery-toc.md) 承担）。
- 英雄分区没有时代维度，由页面整条隐藏，不传零计数。
- 不自行计算计数，计数只来自页面。

## 使用者

- [卡牌图鉴](card-gallery.md)。
