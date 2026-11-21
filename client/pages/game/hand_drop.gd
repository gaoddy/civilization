class_name HandDrop
extends PanelContainer
## 手牌带底部的购买落点：只接受可购买的商店卡拖拽数据。
##
## 只负责判定与转发；是否可买由拖拽数据里的 `buyable` 决定，页面再用视图版本
## `epoch` 与操作门控做落点复查。手牌带之外的空白不接受拖入。

signal shop_dropped(payload: Dictionary)


func _ready() -> void:
    mouse_filter = Control.MOUSE_FILTER_STOP


func _can_drop_data(_at_position: Vector2, data: Variant) -> bool:
    return accepts_shop(data)


func _drop_data(_at_position: Vector2, data: Variant) -> void:
    if not accepts_shop(data):
        return
    shop_dropped.emit((data as Dictionary).duplicate())


## 商店卡拖拽数据：只接受可购买槽位。
static func accepts_shop(data: Variant) -> bool:
    if not (data is Dictionary):
        return false
    var dragged: Dictionary = data
    return dragged.has("shopSlot") and bool(dragged.get("buyable", false))
