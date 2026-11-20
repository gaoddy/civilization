# 图鉴分类目录

> 状态：已实现
> 入口：`GalleryToc.new()`，`client/scenes/gallery_toc.gd`（`class_name GalleryToc`）

## 用途

图鉴随从视图左侧的分类筛选：「全部」与顶层分类的纵向单选列表，无标题，每项带当前计数；选中项下方展开「全部 / 随从 / 效果」二级筛选。英雄视图不显示本组件。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `taxon_selected` | `signal(String)` | 选中顶层分类 id；空串表示「全部」 |
| `kind_selected` | `signal(String)` | 选中二级筛选 id；空串表示「全部」 |
| `set_state(categories, counts, selected, kinds, kind_counts, selected_kind)` | `void` | 用顶层分类、二级项、两组计数与当前选中项重建列表 |

- `categories` 为 `{id, name}` 条目数组；`counts` 以分类 id 为键，空串为全部。
- `kinds` 为二级筛选条目；`kind_counts` 以二级 id 为键，空串为全部。
- 两组计数都由页面按其它筛选维度换算后传入；组件不自行推导分类或计数。

## 变体与状态

- 固定宽 `gallery-toc-width`（140），手机 `gallery-toc-width-compact`（120）；按钮纵向排列，间距 `space-xs`。
- 顶层按钮为 `ghost` 变体的单选 `ButtonGroup`，文字左对齐、横向撑满、高 28。
- 选中「全部」或任一顶层分类后，其下方显示二级按钮（节点名 `KindAll`、`KindMinion`、`KindEffect`），左侧缩进 `space-2xl`，使用独立 `ButtonGroup`。
- 选中项用 `accent-row` 面板样式（圆角 `radius-sm`），文字转 `color-accent`；未选中文字为 `color-text-dim`。
- 计数为 0 且未选中时禁用；当前选中项不禁用。二级项沿用同一规则。
- 文字格式为「名称 计数」，如「哺乳纲 122」「机械 82」「随从 332」；计数按图鉴当前筛选口径计算，多玩法卡在多个分类中各计一次，「全部」按唯一卡牌 id 计。

## 事件

- `taxon_selected(root_id)`：点击顶层项时发出；切换顶层分类后二级筛选回到「全部」。
- `kind_selected(kind)`：点击二级项时发出。

## 禁用场景

- 顶层分类列「全部」与 13 个玩法，不再有「中立」；英雄不作为顶层分类。
- 不展开玩法内部或科学分类下层节点。
- 分类归属由页面层按卡牌全部 `play:` 归属计算；效果牌不作为顶层分类。
- 英雄视图由页面整体隐藏本组件，改由顶部英雄分类栏承担筛选。
- 不自行推导分类路径与计数。

## 使用者

- [卡牌图鉴](card-gallery.md)。
