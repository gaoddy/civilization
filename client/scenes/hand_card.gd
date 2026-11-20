class_name HandCard
extends PanelContainer
## 手牌卡：拖拽是唯一出牌方式，点击不产生任何意图。
##
## 页面把「现在能不能拖」汇总成 [`HandCard.drag_allowed`]（阶段、视图新鲜度、
## 倒计时、连接、待选与 `playable`），本控件只负责在被允许时交出拖拽数据。
## 拖拽数据同时带上页面视图版本 `epoch`：落点方按它判断这次拖拽是否已过期。
## 本卡同时也是商店卡拖入手牌区的落点：接受可购买的商店卡数据并转发给手牌行。

## {epoch, handIndex, instanceId, cardId, cardKind: "minion"|"effect", requiresTarget, targets}
## `targets` 为服务端下发的合法目标 `{zone, id}`，只有需要目标的效果牌非空。
var payload: Dictionary = {}

## 由页面按阶段、视图新鲜度、连接、待选与 `playable` 汇总；`false` 时拖拽不开始。
var drag_allowed := false

## 拖拽真正开始时同步发出，供手牌槽在复制预览之前复位悬停缩放。
signal drag_started

## 需要目标的牌拖到本卡（本人手牌候选）上松手时发出。
signal target_dropped(data: Dictionary, target: Dictionary)

## 商店卡拖到本卡上松手时发出，由手牌行统一上抛给页面。
signal shop_dropped(payload: Dictionary)


func _get_drag_data(at_position: Vector2) -> Variant:
    if not drag_allowed or payload.is_empty():
        return null
    # 预览会复制当前的 scale/pivot，必须先复位悬停再复制。
    # `at_position` 是悬停放大后的局部坐标，先换算到画布再按复位后的卡片取起手点。
    var grabbed := get_global_transform_with_canvas() * at_position
    drag_started.emit()
    var grab := get_global_transform_with_canvas().affine_inverse() * grabbed
    var preview := duplicate() as Control
    preview.modulate = Color(1, 1, 1, 0.7)
    preview.custom_minimum_size = custom_minimum_size
    set_drag_preview(UI.drag_preview(preview, grab))
    return payload.duplicate()


func _can_drop_data(_at_position: Vector2, data: Variant) -> bool:
    return not _accepted_target(data).is_empty() or HandDrop.accepts_shop(data)


func _drop_data(_at_position: Vector2, data: Variant) -> void:
    var target := _accepted_target(data)
    if not target.is_empty():
        target_dropped.emit(data as Dictionary, target)
        return
    if HandDrop.accepts_shop(data):
        shop_dropped.emit((data as Dictionary).duplicate())


## 本卡作为手牌目标候选时返回拖拽数据里的该候选；否则返回空字典。
##
## 只接受「需要目标的效果牌」；拖到自己身上（实例编号相同）一律拒绝。
func _accepted_target(data: Variant) -> Dictionary:
    if not (data is Dictionary):
        return {}
    var dragged: Dictionary = data
    if not dragged.has("handIndex") or str(dragged.get("cardKind", "")) != "effect":
        return {}
    if not bool(dragged.get("requiresTarget", false)):
        return {}
    var own_id := str(payload.get("instanceId", ""))
    if own_id == "" or own_id == str(dragged.get("instanceId", "")):
        return {}
    for candidate in dragged.get("targets", []):
        if not (candidate is Dictionary):
            continue
        if str(candidate.get("zone", "")) == "hand" and str(candidate.get("id", "")) == own_id:
            return candidate
    return {}
