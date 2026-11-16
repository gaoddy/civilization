class_name BoardDrop
extends HFlowContainer
## 我方棋盘的拖拽落点：按指针位置判定是否接受手牌，并把落点换算成插入下标。
##
## 行级命中判定（而不是在每张卡之间塞落点控件）：落点宽度不占布局，7 张卡时也
## 不会挤压卡片；指针落在随从卡上时，该卡既是插入参照（随从牌插入到它左边），
## 也是效果牌的目标随从。

## 有效落点才发；`position` 为插入下标（随从牌用），`target_minion_id` 为目标随从（效果牌用）。
signal dropped(data: Dictionary, position: int, target_minion_id: String)

## 棋盘满时由页面标记，随从牌一律不接受。
var board_full := false


func _can_drop_data(at_position: Vector2, data: Variant) -> bool:
    return not _resolve(at_position, data).is_empty()


func _drop_data(at_position: Vector2, data: Variant) -> void:
    var resolved := _resolve(at_position, data)
    if resolved.is_empty():
        return
    dropped.emit(
        data as Dictionary,
        int(resolved.get("position", 0)),
        str(resolved.get("target_minion_id", ""))
    )


## 判定落点：返回空字典表示拒绝，否则带 `position` 与 `target_minion_id`。
func _resolve(at_position: Vector2, data: Variant) -> Dictionary:
    if not (data is Dictionary):
        return {}
    var payload: Dictionary = data
    if not payload.has("handIndex"):
        return {}
    var card_kind := str(payload.get("cardKind", ""))
    var requires_target := bool(payload.get("requiresTarget", false))

    var hit := _minion_card_at(at_position)
    if hit != null:
        if card_kind == "effect" and requires_target:
            return {"position": 0, "target_minion_id": str(hit.get_meta("minion_id", ""))}
        return {"position": _index_of(hit), "target_minion_id": ""}

    if card_kind == "minion":
        if board_full:
            return {}
        return {"position": _insert_index(at_position), "target_minion_id": ""}
    if card_kind == "effect" and not requires_target:
        return {"position": 0, "target_minion_id": ""}
    return {}


## 指针下的随从卡（带 `minion_id` 元数据的子控件）；空白或空位返回 `null`。
func _minion_card_at(point: Vector2) -> Control:
    var global_point: Vector2 = get_global_transform() * point
    for child in get_children():
        if not (child is Control):
            continue
        var control: Control = child
        if not control.has_meta("minion_id"):
            continue
        if control.get_global_rect().has_point(global_point):
            return control
    return null


## 随从卡在棋盘上的下标；优先读元数据，缺失时退回子节点序号。
func _index_of(card: Control) -> int:
    if card.has_meta("minion_index"):
        return int(card.get_meta("minion_index"))
    return card.get_index()


## 空白落点的插入下标：指针在随从卡中心之前就插到它左边，否则依次后移。
func _insert_index(point: Vector2) -> int:
    var global_x: float = (get_global_transform() * point).x
    var index := 0
    for child in get_children():
        if not (child is Control):
            continue
        var control: Control = child
        if not control.has_meta("minion_id"):
            continue
        if global_x < control.get_global_rect().get_center().x:
            return index
        index += 1
    return index
