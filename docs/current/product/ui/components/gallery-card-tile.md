# 图鉴卡面单元

> 状态：已实现
> 入口：`GalleryCardTile.new(id, selected, hero = false)`，`client/ui/gallery_card_tile.gd`（`class_name GalleryCardTile`）

## 用途

图鉴网格中的单张卡面：复用对局卡面，点击选中；选中与悬停描边由 [可选项框](selectable-box.md) 的内联变体提供。

## 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `card_id` | `String` | 卡牌 id |
| `set_selected(value)` | `void` | 切换选中描边；继承自 `SelectableBox` |

- 最小尺寸为 `card-width` × `card-height`（112×146）。
- 节点名为 `GalleryCard_<id>`；原生 tooltip 为卡名。
- 普通卡面由 `GalleryCardFace.build` 构建：原始攻防、玩法、`display_cost` 与时代，不显示效果文本。
- `hero` 为真时改由 `GalleryCardFace.build_hero` 构建：封面、名称与玩法，不含角标、技能文本、费用、时代与攻击/生命。

## 变体与状态

- 默认：`raised-flush` 面板（1px 描边内缩），1px `color-border` 描边。
- 悬停：1px `color-border-strong` 描边，指针为手型。
- 选中：2px `color-accent` 金色描边，优先于悬停样式。
- 以上三态由 `SelectableBox.Outline.INLINE` 提供；本组件只传入 `raised-flush` 底板。
- 同时存在的数量由网格每批加载张数与该批所在滚动区决定（桌面与 iPad 每批 18，手机每批 12），本组件不感知。
- `mouse_filter` 为 `PASS`，按下与拖动继续冒泡到滚动区；手机触屏在卡面上按住拖动即滚动网格，位移超过 12 像素视为滚动，不切换详情。

## 事件

- `card_selected(card_id)`：左键在卡面上按下并松开、且位移不超过 12 像素时发出；由 `SelectableBox.activated` 转发。卡面不消费按下与拖动事件，使触屏拖拽可以由滚动区接管。

## 禁用场景

- 不支持拖拽、右键、购买或出牌。
- 不显示当前数值与冻结等对局状态；顶部星为卡表 `level` 的 1–7 星，与对局卡面一致。

## 使用者

- [图鉴卡面网格](gallery-grid.md)。
