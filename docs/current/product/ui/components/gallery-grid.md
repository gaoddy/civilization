# 图鉴卡面网格

> 状态：已实现
> 入口：`GalleryGrid.new()`，`client/scenes/gallery_grid.gd`（`class_name GalleryGrid`）

## 用途

图鉴中区的卡面网格：桌面、iPad 与手机均为 6 列，网格纵向滚动，滚到接近底部时自动追加下一批；底部显示已加载张数与加载提示。不再有分页按钮。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `card_selected` | `signal(String)` | 点击某张卡面时发出卡牌 id |
| `set_cards(ids, selected_id, hero = false)` | `void` | 用新的卡牌集合重建首批并把滚动归零；`hero` 为真时卡面走英雄样式 |
| `select(selected_id)` | `void` | 只更新已加载卡面的选中描边，不重建 |

- 列数取 `gallery-grid-columns`（6），手机取 `gallery-grid-columns-compact`（6）；列距桌面与 iPad 为 `space-lg`，手机为 `space-md`。
- 每批张数取 `gallery-batch-size`（18），手机取 `gallery-batch-size-compact`（12）。
- 内部节点名：`GalleryScroll`、`GalleryScrollBody`、`GalleryGridRows`、`GalleryLoadStatus`、`GalleryEmpty`。

## 变体与状态

- 集合为空时隐藏滚动区与加载状态，显示「没有符合条件的卡牌」。
- 触屏可在卡面上按住拖动滚动：卡面不消费指针事件，拖拽由滚动区按 Godot 内建触屏拖动处理；拖动不改变选中。
- 首批构建后若内容高度不足一屏，自动继续加载，直到可滚动或全部加载完。
- 滚动条距底部小于一张卡的高度时加载下一批；同一批只追加一次。
- 底部状态：未加载完为「已显示 N / M 张 · 继续下滑加载」，全部加载完为「已显示 M / M 张 · 已全部加载」。
- 改筛选、切换视图或重进图鉴时重建首批并回到顶部；选中卡不在结果内时页面会改为结果首张。
- `hero` 只影响卡面构建（无费用、时代与攻击/生命），加载与选中逻辑不变。

## 事件

- `card_selected(card_id)`：转发任一已加载卡面的点击。

## 禁用场景

- 不持有筛选条件，不知道时代或分类；筛选后的集合由页面传入。
- 不做多选、不做拖拽，也不显示卡牌详情。

## 使用者

- [卡牌图鉴](card-gallery.md)。
- 卡面单元：[图鉴卡面单元](gallery-card-tile.md)。
