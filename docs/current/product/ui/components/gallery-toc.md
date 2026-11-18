# 图鉴分类目录

> 状态：已实现
> 入口：`GalleryToc.new()`，`client/scenes/gallery_toc.gd`（`class_name GalleryToc`）

## 用途

图鉴左侧的界级分类筛选：「分类目录」标题、「全部」与 8 个界级分类的纵向单选列表，每项带当前计数。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `taxon_selected` | `signal(String)` | 选中界级分类 id；空串表示「全部」 |
| `set_state(roots, counts, selected)` | `void` | 用根节点、计数与当前选中项重建列表 |

- `roots` 为界级分类 id 数组；`counts` 以分类 id 为键，空串为全部；由页面按时代筛选换算后传入。

## 变体与状态

- 固定宽 `gallery-toc-width`（168），按钮纵向排列，间距 `space-xs`。
- 按钮为 `ghost` 变体的单选 `ButtonGroup`，文字左对齐、横向撑满、高 28。
- 选中项用 `accent-row` 面板样式（圆角 `radius-sm`），文字转 `color-accent`；未选中文字为 `color-text-dim`。
- 计数为 0 且未选中时禁用；当前选中项不禁用。
- 文字格式为「名称 计数」，如「动物界 199」「全部 432」。

## 事件

- `taxon_selected(root_id)`：点击任意可用项时发出。

## 禁用场景

- 只列界级根节点，不展开门、纲、目、科等下层分类。
- 不自行推导分类路径与计数。

## 使用者

- [卡牌图鉴](card-gallery.md)。
