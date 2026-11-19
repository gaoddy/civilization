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
    # 视图刷新重建卡片时先显式复位旧卡：触控端没有鼠标式悬停移出，
    # 放大与置顶可能残留，右侧槽位的 mouse_filter 也要恢复。
    _clear_hover()
    if is_instance_valid(card) and card != hand_card and card.get_parent() == self:
        remove_child(card)
    card = hand_card
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    custom_minimum_size = Vector2(step_width, card.custom_minimum_size.y)
    card.position = Vector2.ZERO
    card.size = card.custom_minimum_size
    if card.get_parent() != self:
        add_child(card)
    card.mouse_entered.connect(_on_card_mouse_entered)
    card.mouse_exited.connect(_on_card_mouse_exited)
    card.drag_started.connect(_clear_hover)


## 悬停以底边中点为轴放大：放大后的矩形包含原矩形，指针不会在卡片下缘反复进出。
##
## Godot 的 GUI 拾取按树序逆序进行、不比较 `z_index`，右侧交叠带本来会落到下一张卡；
## 悬停期间让右侧槽位让出鼠标，置顶卡才能在放大后的整个矩形内保持悬停与拖拽。
func _on_card_mouse_entered() -> void:
    if not is_instance_valid(card):
        return
    z_index = 1
    card.pivot_offset = Vector2(card.size.x * 0.5, card.size.y)
    card.scale = Vector2.ONE * UITheme.factor("hand-hover-scale")
    _set_later_cards_pointer(false)


func _on_card_mouse_exited() -> void:
    _clear_hover()


## 移出或拖拽开始时复位缩放与层级；拖拽预览在信号之后复制，保持原始尺寸。
func _clear_hover() -> void:
    if is_instance_valid(card):
        card.scale = Vector2.ONE
        card.pivot_offset = Vector2.ZERO
    z_index = 0
    _set_later_cards_pointer(true)


## 触控端抬手复位：触摸没有鼠标式悬停移出，放大与置顶必须显式撤回。
## 只在置顶槽位上处理，桌面端的鼠标松开不进入这里。
func _input(event: InputEvent) -> void:
    if not ScreenProfile.is_touch() or z_index == 0:
        return
    if event is InputEventScreenTouch and not event.pressed:
        _clear_hover()
    elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and not event.pressed:
        _clear_hover()


## 触控端拖拽结束（落点成功或被取消）兜底复位；拖拽开始时已经复位过一次。
func _notification(what: int) -> void:
    if what == NOTIFICATION_DRAG_END and ScreenProfile.is_touch():
        _clear_hover()


## 只处理右侧槽位：左侧槽位在树序中晚于本槽被拾取，且不会被本卡盖住。
func _set_later_cards_pointer(enabled: bool) -> void:
    var parent := get_parent()
    if parent == null:
        return
    var filter := Control.MOUSE_FILTER_STOP if enabled else Control.MOUSE_FILTER_IGNORE
    for index in range(get_index() + 1, parent.get_child_count()):
        var sibling := parent.get_child(index) as HandSlot
        if sibling != null and is_instance_valid(sibling.card):
            sibling.card.mouse_filter = filter
