class_name HandSlot
extends Control
## 手牌槽：以固定步进承载一张手牌卡，卡片向右溢出与下一槽重叠。
##
## 槽位自身只有步进宽（card-width - hand-card-overlap），不是容器，因此不会
## 改写卡片的完整尺寸。卡片贴槽左上角，溢出的部分由右侧的下一槽覆盖。

## 槽位承载的手牌卡；由页面在 [`HandSlot.setup`] 中注入。
var card: HandCard


## `step_width` 是相邻槽的全局 x 差；卡片保持 `custom_minimum_size` 的完整尺寸。
func setup(hand_card: HandCard, step_width: float) -> void:
    card = hand_card
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    custom_minimum_size = Vector2(step_width, card.custom_minimum_size.y)
    card.position = Vector2.ZERO
    card.size = card.custom_minimum_size
    add_child(card)
