# 图鉴卡面网格

> 状态：已实现
> 入口：`GalleryGrid.new()`，`client/scenes/gallery_grid.gd`（`class_name GalleryGrid`）

## 用途

图鉴中区的卡面网格与分页：桌面 6 列、手机 4 列，每页张数按可见行数固定，只构建当前页，底部为上一页 / 页码 / 下一页；不设滚动容器。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `card_selected` | `signal(String)` | 点击某张卡面时发出卡牌 id |
| `set_cards(ids, selected_id)` | `void` | 用新的卡牌集合重建第 1 页 |
| `select(selected_id)` | `void` | 只更新选中描边，不重建当前页 |

- 页大小取 `gallery-page-size`（18 = 6 列 × 3 行），手机取 `gallery-page-size-compact`（8 = 4 列 × 2 行）；列数取 `gallery-grid-columns`（6），手机取 `gallery-grid-columns-compact`（4）。
- 内部节点名：`GalleryGridRows`、`GalleryEmpty`、`GalleryPager`、`GalleryPrev`、`GalleryPageLabel`、`GalleryNext`。
- 网格直接铺在组件内，没有 `ScrollContainer`；每页正好填满可见高度，翻页代替滚动。

## 变体与状态

- 集合为空时隐藏网格与分页条，显示「没有符合条件的卡牌」。
- 页码文案为「第 N/M 页」；首页禁用上一页，末页禁用下一页。
- 列/行间距 `space-lg`；网格不滚动，内容超出视口高度时按页切分。

## 事件

- `card_selected(card_id)`：转发当前页任一卡面的点击。

## 禁用场景

- 不持有筛选条件，不知道时代或分类；筛选后的集合由页面传入。
- 不做多选、不做拖拽，也不显示卡牌详情。

## 使用者

- [卡牌图鉴](card-gallery.md)。
- 卡面单元：[图鉴卡面单元](gallery-card-tile.md)。
