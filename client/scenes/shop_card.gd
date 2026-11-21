class_name ShopCard
extends PanelContainer
## 商店卡：拖到手牌区购买，点击不产生任何意图。
##
## 页面把「现在能不能买」汇总成 [`ShopCard.drag_allowed`]（阶段、视图新鲜度、
## 倒计时、连接、待选与 `buyable`），本控件只负责在被允许时交出拖拽数据。
## 拖拽数据带视图版本 `epoch`：手牌落点回传页面后按它判断这次拖拽是否已过期。

## {epoch, shopSlot, cardId, buyable}
var payload: Dictionary = {}

## 由页面按阶段、视图新鲜度、连接、待选与 `buyable` 汇总；`false` 时拖拽不开始。
var drag_allowed := false


func _ready() -> void:
    # PASS：保留本卡的拖拽起手，同时让棋盘卡的出售落点继续冒泡到商店面板。
    mouse_filter = Control.MOUSE_FILTER_PASS


func _get_drag_data(at_position: Vector2) -> Variant:
    if not drag_allowed or payload.is_empty():
        return null
    var preview := duplicate() as Control
    preview.modulate = Color(1, 1, 1, 0.7)
    preview.custom_minimum_size = custom_minimum_size
    set_drag_preview(UI.drag_preview(preview, at_position, self))
    return payload.duplicate()
