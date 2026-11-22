# 图鉴筛选面板

> 状态：已实现
> 入口：`GalleryFilterPanel.new()`，`client/ui/gallery_filter_panel.gd`（`class_name GalleryFilterPanel`）

## 用途

图鉴筛选下拉展开后的条目面板：挂在触发按钮下沿，列出当前维度的条目、名称与计数。只呈现条目并发出选择，筛选状态由页面持有。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `item_selected(id)` | `signal(String)` | 选中条目 id；空串表示「全部」 |
| `open_at(anchor, items, counts, selected, prefix)` | `void` | 以 `anchor` 为面板左上角重建并显示 |
| `close()` / `is_open()` | `void` / `bool` | 收起 / 查询显隐 |

- `items` 为 `{id, name}` 条目数组；`counts` 以 id 为键，空串为全部。
- `prefix` 决定条目节点名：全部项为 `<prefix>All`，其余为 `<prefix>` + id 首字母大写，例如 `Chapter1`、`TaxonMammal`、`KindEffect`、`HeroCategoryMammal`。
- 面板宽 `gallery-filter-panel-width`（240），`surface` 面板样式，条目纵向排列、间距 `space-2xs`。
- 面板继承 `PopupLayer` 的 `ANCHORED` 定位，遮罩透明（透明度 0）只负责接收「点外部关闭」；`edge_margin` 取 `space-md`，超出页面时夹紧。

## 变体与状态

- 条目为 `ghost` 样式的切换按钮，高 30、横向撑满；名称左对齐，计数右对齐并使用 `font-caption` / `color-text-muted`。
- 当前选中项使用 `accent-row` 面板样式（圆角 `radius-sm`），名称转 `color-accent`。
- 计数为 0 且未选中时禁用并置灰；当前选中项即使计数为 0 也不禁用。
- 点任一条目先收起面板，再发出 `item_selected`；点面板外关闭；按 `Esc` 由页面收起。
- 同一下拉按钮再次点击时由页面收起，不重新打开。

## 禁用场景

- 不做多选，不承载计数计算、分类推导或筛选状态。
- 不打开或关闭详情浮层。

## 使用者

- [卡牌图鉴](card-gallery.md)。
