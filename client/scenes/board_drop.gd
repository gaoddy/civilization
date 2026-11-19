class_name BoardDrop
extends HFlowContainer
## 我方棋盘的拖拽落点：接受手牌与棋盘随从两种拖拽数据。
##
## 手牌落点：按指针位置判定是否接受，并把落点换算成插入下标。
## 棋盘卡落点：拖起后隐藏原卡、插入一个同尺寸透明占位，指针移动时实时移动占位，
## 其余随从随之让位；松手时返回「移除后剩余随从序列中的插入下标」，与
## `RearrangeBoard.to` 的语义一致。
##
## 行级命中判定（而不是在每张卡之间塞落点控件）：落点宽度不占布局，7 张卡时也
## 不会挤压卡片；指针落在随从卡上时，该卡既是插入参照（随从牌插入到它左边），
## 也是效果牌的目标随从。

## 有效落点才发；`position` 为插入下标（随从牌用），`target` 为目标 `{zone, id}`（效果牌用）。
signal dropped(data: Dictionary, position: int, target: Dictionary)

## 棋盘满时由页面标记，手牌随从牌一律不接受。
var board_full := false

## 棋盘卡拖拽状态：被拖随从 id、原始子节点顺序、透明占位与当前插入下标。
var _drag_id := ""
var _drag_card: Control = null
var _spacer: Control = null
var _original_order: Array = []
var _preview_index := -1

## 手牌拖拽期间监听指针位置，只在无效落点上显示禁止光标。
var _watching_hand_drag := false


func _ready() -> void:
    set_process(false)


func _notification(what: int) -> void:
    if what == NOTIFICATION_DRAG_BEGIN:
        var payload = get_viewport().gui_get_drag_data()
        if payload is Dictionary and payload.has("minionId"):
            begin_drag(payload)
        elif payload is Dictionary and payload.has("handIndex"):
            _watching_hand_drag = true
            set_process(true)
    elif what == NOTIFICATION_DRAG_END:
        end_drag()
        _stop_hand_watch()


## 开始棋盘卡拖拽预览：隐藏被拖卡片，在它原来的位置放一个同尺寸透明占位。
func begin_drag(payload: Dictionary) -> void:
    end_drag()
    var minion_id := str(payload.get("minionId", ""))
    var card := _card_by_id(minion_id)
    if card == null:
        return
    _drag_id = minion_id
    _drag_card = card
    _original_order.clear()
    for child in get_children():
        if child is Control:
            _original_order.append(child)
    _preview_index = _index_of(card)
    _spacer = Control.new()
    _spacer.name = "BoardDragSpacer"
    _spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _spacer.custom_minimum_size = Vector2(
        UITheme.stage_metric("card-width"),
        UITheme.stage_metric("card-height")
    )
    add_child(_spacer)
    move_child(_spacer, card.get_index())
    card.visible = false
    set_process(true)


## 结束预览（落点或取消）：移除占位、恢复被拖卡片与原始顺序。
func end_drag() -> void:
    if _drag_id == "" and _spacer == null:
        return
    if is_instance_valid(_spacer):
        _spacer.queue_free()
    _spacer = null
    if is_instance_valid(_drag_card):
        _drag_card.visible = true
    var order := 0
    for child in _original_order:
        if is_instance_valid(child) and child.get_parent() == self:
            move_child(child, order)
            order += 1
    _drag_id = ""
    _drag_card = null
    _original_order.clear()
    _preview_index = -1
    set_process(false)


func _process(_delta: float) -> void:
    if _watching_hand_drag:
        _update_hand_cursor()
    if _drag_id == "":
        return
    # 拖拽途中视图重建会整块换掉棋盘子节点：旧卡与占位失效时按取消收尾。
    if (
        not is_instance_valid(_spacer)
        or _spacer.get_parent() != self
        or not is_instance_valid(_drag_card)
        or _drag_card.get_parent() != self
    ):
        end_drag()
        return
    var point := get_global_mouse_position()
    if not get_global_rect().has_point(point):
        return
    var index := _gap_index(point)
    if index == _preview_index:
        return
    _preview_index = index
    move_child(_spacer, _gap_child_index(index))


## 手牌拖拽的落点判定：空位仍可放置；棋盘外、已满或需要目标却悬停在空白处拒绝。
func hand_drop_accepted(global_point: Vector2, payload: Dictionary) -> bool:
    if not get_global_rect().has_point(global_point):
        return false
    var local: Vector2 = get_global_transform().affine_inverse() * global_point
    return not _resolve(local, payload).is_empty()


## 拖拽过程中按指针所在位置切换光标：只有棋盘内的无效落点显示禁止。
##
## 指针离开棋盘后交还给手牌卡的 `_can_drop_data`，不能把整屏都标成禁止。
func _update_hand_cursor() -> void:
    var viewport := get_viewport()
    if viewport == null or not viewport.gui_is_dragging():
        _stop_hand_watch()
        return
    var payload = viewport.gui_get_drag_data()
    if not (payload is Dictionary) or not payload.has("handIndex"):
        _stop_hand_watch()
        return
    var point := get_global_mouse_position()
    if not get_global_rect().has_point(point):
        Input.set_default_cursor_shape(Input.CURSOR_ARROW)
        return
    var accepted := hand_drop_accepted(point, payload)
    Input.set_default_cursor_shape(Input.CURSOR_ARROW if accepted else Input.CURSOR_FORBIDDEN)


## 停止手牌拖拽光标监听并恢复正常指针。
func _stop_hand_watch() -> void:
    if not _watching_hand_drag:
        return
    _watching_hand_drag = false
    Input.set_default_cursor_shape(Input.CURSOR_ARROW)
    if _drag_id == "":
        set_process(false)


func _can_drop_data(at_position: Vector2, data: Variant) -> bool:
    return not _resolve(at_position, data).is_empty()


func _drop_data(at_position: Vector2, data: Variant) -> void:
    var resolved := _resolve(at_position, data)
    if resolved.is_empty():
        return
    dropped.emit(
        data as Dictionary,
        int(resolved.get("position", 0)),
        Dictionary(resolved.get("target", {}))
    )
    end_drag()


## 判定落点：返回空字典表示拒绝，否则带 `position` 与 `target`。
func _resolve(at_position: Vector2, data: Variant) -> Dictionary:
    if not (data is Dictionary):
        return {}
    var payload: Dictionary = data
    if payload.has("minionId"):
        # 棋盘卡只想换位：被拖随从必须与当前预览一致。
        if _drag_id == "" or str(payload.get("minionId", "")) != _drag_id:
            return {}
        return {"position": _gap_index(_to_global(at_position)), "target": {}}

    if not payload.has("handIndex"):
        return {}
    var card_kind := str(payload.get("cardKind", ""))
    var requires_target := bool(payload.get("requiresTarget", false))

    var hit := _minion_card_at(at_position)
    if hit != null:
        if card_kind == "effect" and requires_target:
            # 只有落点随从在服务端下发的候选里才接受，否则退回手牌。
            var target := _target_for(payload, "board", str(hit.get_meta("minion_id", "")))
            if target.is_empty():
                return {}
            return {"position": 0, "target": target}
        if card_kind == "minion" and board_full:
            return {}
        return {"position": _index_of(hit), "target": {}}

    if card_kind == "minion":
        if board_full:
            return {}
        return {"position": _insert_index(at_position), "target": {}}
    if card_kind == "effect" and not requires_target:
        return {"position": 0, "target": {}}
    return {}


## 拖拽数据的目标候选里是否包含指定区域与实例编号；命中时返回该候选。
func _target_for(payload: Dictionary, zone: String, id: String) -> Dictionary:
    if id == "":
        return {}
    for candidate in payload.get("targets", []):
        if not (candidate is Dictionary):
            continue
        if str(candidate.get("zone", "")) == zone and str(candidate.get("id", "")) == id:
            return candidate
    return {}


## 指针下的随从卡（带 `minion_id` 元数据的可见子控件）；空白或空位返回 `null`。
func _minion_card_at(point: Vector2) -> Control:
    var global_point: Vector2 = get_global_transform() * point
    for child in get_children():
        if not (child is Control):
            continue
        var control: Control = child
        if not control.visible or not control.has_meta("minion_id"):
            continue
        if control.get_global_rect().has_point(global_point):
            return control
    return null


## 随从卡在棋盘上的下标；优先读元数据，缺失时退回子节点序号。
func _index_of(card: Control) -> int:
    if card.has_meta("minion_index"):
        return int(card.get_meta("minion_index"))
    return card.get_index()


## 棋盘卡插入下标：当前预览中指针左侧的随从数量（不含被拖随从与占位）。
func _gap_index(global_point: Vector2) -> int:
    var index := 0
    for child in get_children():
        if not (child is Control):
            continue
        var control: Control = child
        if control == _drag_card or control == _spacer:
            continue
        if not control.visible or not control.has_meta("minion_id"):
            continue
        if global_point.x > control.get_global_rect().get_center().x:
            index += 1
    return index


## 把插入下标换算成占位控件的子节点下标：跳过占位自身，但保留隐藏原卡的卡位。
func _gap_child_index(gap: int) -> int:
    var position := 0
    var seen := 0
    for child in get_children():
        if not (child is Control):
            continue
        var control: Control = child
        if control == _spacer:
            continue
        if control == _drag_card:
            position += 1
            continue
        if control.visible and control.has_meta("minion_id"):
            if seen == gap:
                return position
            seen += 1
        elif seen == gap:
            return position
        position += 1
    return position


## 空白落点的插入下标：指针在随从卡中心之前就插到它左边，否则依次后移。
func _insert_index(point: Vector2) -> int:
    var global_x: float = (get_global_transform() * point).x
    var index := 0
    for child in get_children():
        if not (child is Control):
            continue
        var control: Control = child
        if not control.visible or not control.has_meta("minion_id"):
            continue
        if global_x < control.get_global_rect().get_center().x:
            return index
        index += 1
    return index


func _card_by_id(minion_id: String) -> Control:
    for child in get_children():
        if child is Control and child.has_meta("minion_id") and str(child.get_meta("minion_id")) == minion_id:
            return child
    return null


## 落点坐标换到画布坐标，与 `get_global_rect()` 同一坐标系。
func _to_global(point: Vector2) -> Vector2:
    return get_global_transform() * point
