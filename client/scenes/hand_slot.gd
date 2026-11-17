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
    card.mouse_entered.connect(_on_card_mouse_entered)
    card.mouse_exited.connect(_on_card_mouse_exited)
    card.drag_started.connect(_clear_hover)


## 悬停以底边中点为轴放大：放大后的矩形包含原矩形，指针不会在卡片下缘反复进出。
func _on_card_mouse_entered() -> void:
    if not is_instance_valid(card):
        return
    z_index = 1
    card.pivot_offset = Vector2(card.size.x * 0.5, card.size.y)
    card.scale = Vector2.ONE * UITheme.factor("hand-hover-scale")


func _on_card_mouse_exited() -> void:
    _clear_hover()


## 移出或拖拽开始时复位缩放与层级；拖拽预览在信号之后复制，保持原始尺寸。
func _clear_hover() -> void:
    if is_instance_valid(card):
        card.scale = Vector2.ONE
        card.pivot_offset = Vector2.ZERO
    z_index = 0
