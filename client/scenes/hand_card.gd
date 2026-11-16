class_name HandCard
extends PanelContainer
## 手牌卡：拖拽是唯一出牌方式，点击不产生任何意图。
##
## 页面把「现在能不能拖」汇总成 [`HandCard.drag_allowed`]（阶段、视图新鲜度、
## 倒计时、连接、待选与 `playable`），本控件只负责在被允许时交出拖拽数据。
## 拖拽数据同时带上页面视图版本 `epoch`：落点方按它判断这次拖拽是否已过期。

## {epoch, handIndex, cardId, cardKind: "minion"|"effect", requiresTarget}
var payload: Dictionary = {}

## 由页面按阶段、视图新鲜度、连接、待选与 `playable` 汇总；`false` 时拖拽不开始。
var drag_allowed := false


func _get_drag_data(_at_position: Vector2) -> Variant:
    if not drag_allowed or payload.is_empty():
        return null
    var preview := duplicate() as Control
    preview.modulate = Color(1, 1, 1, 0.7)
    preview.custom_minimum_size = custom_minimum_size
    set_drag_preview(preview)
    return payload.duplicate()
