# 图鉴卡面单元

> 状态：已实现
> 入口：`GalleryCardTile.new(id, selected)`，`client/scenes/gallery_card_tile.gd`（`class_name GalleryCardTile`）

## 用途

图鉴网格中的单张卡面：复用对局卡面，点击选中，并表达选中与悬停描边。

## 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `card_id` | `String` | 卡牌或 Token 的 id |
| `set_selected(value)` | `void` | 切换选中描边 |

- 最小尺寸为 `card-width` × `card-height`（112×146）。
- 节点名为 `GalleryCard_<id>`；原生 tooltip 为卡名。
- 卡面由 `GameCardFace.face` 构建：原始攻防、玩法、英雄标记、`display_cost` 与时代。

## 变体与状态

- 默认：`raised-tight` 面板，1px `color-border` 描边。
- 悬停：1px `color-border-strong` 描边，指针为手型。
- 选中：2px `color-accent` 金色描边，优先于悬停样式。
- 数量上限由网格每页 40 张决定，本组件不感知。

## 事件

- `card_selected(card_id)`：左键按下时发出，并消费该事件。

## 禁用场景

- 不支持拖拽、右键、购买或出牌。
- 不显示当前数值、冻结、星级等对局状态；只显示卡表原始信息。

## 使用者

- [图鉴卡面网格](gallery-grid.md)。
