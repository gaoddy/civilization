class_name GameAimLayer
extends Control
## 技能瞄准层：从技能出处或战吼来源画一条指向指针的弧线，松手即对命中目标提交。
##
## 三种状态：
## - 技能模式：英雄技能按钮按下时由页面开启，抬起命中候选提交 `UseHeroPower`，
##   落在非候选或按 `Esc` 取消，不发意图。
## - 待选模式：服务端下发战吼目标待选时开启，按下开始瞄准；抬起命中候选提交
##   `ChooseTarget`，落在非候选或按 `Esc` 提交 `AbandonTarget`。
## - 拖动模式：只观察 Godot 的手牌拖拽，为需要目标的牌画同一套视觉，不参与提交。
##
## 输入仅在技能与待选模式下由本层消费；鼠标与触控分别处理，坐标统一换算到画布。
## 命中判定集中在这里：按候选 `{zone, id}` 查找带 `target_zone` / `target_id`
## 元数据的卡控件，取它的全局矩形。

## 选中的候选目标 `{zone, id}`。
signal target_chosen(target: Dictionary)
## 待选模式下松手落在非候选处或按 `Esc`：放弃本次入场。
signal target_abandoned
## 技能模式下落在非候选处或按 `Esc`：取消本次技能。
signal aim_cancelled

enum Mode {
	## 不在瞄准：不绘制、不接收输入。
	NONE,
	## 英雄技能瞄准。
	SKILL,
	## 服务端战吼目标待选。
	PENDING,
	## 手牌拖拽的只读视觉。
	DRAG,
}

## 二次贝塞尔采样段数：段数越多弧线越平滑，虚线与光点按折线累计。
const ARC_SAMPLES := 32
## 弧线控制点的垂直偏移比例（相对源与指针的距离）。
const ARC_BEND := 0.22
## 沿弧线流动的光点数量。
const FLOW_DOTS := 3

var _mode := Mode.NONE
var _candidates: Array = []
var _source_id := ""
var _source_rect := Rect2()
var _pointer := Vector2.ZERO
var _pointer_valid := false
var _pressed := false
var _touch_index := -1
var _hovered: Dictionary = {}
var _time := 0.0


func _init() -> void:
	name = "AimLayer"
	set_anchors_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_process(true)


## 是否处于技能或待选模式（页面据此决定是否 clearing）。
func aiming() -> bool:
	return _mode == Mode.SKILL or _mode == Mode.PENDING


func skill_active() -> bool:
	return _mode == Mode.SKILL


func pending_active() -> bool:
	return _mode == Mode.PENDING


## 进入技能瞄准：按下按钮时调用，抬起由本层接管。
func begin_skill(source_rect: Rect2, candidates: Array) -> void:
	if candidates.is_empty():
		return
	_mode = Mode.SKILL
	_source_id = ""
	_source_rect = _local_rect(source_rect)
	_candidates = candidates.duplicate()
	_pointer = _local_point(get_global_mouse_position())
	_pointer_valid = true
	_pressed = true
	_touch_index = -1
	_update_hover()
	queue_redraw()


## 进入战吼待选瞄准：来源随从按 `minion_id` 查找；同一来源与候选重复调用不重置手势。
func begin_pending(source_minion_id: String, candidates: Array) -> void:
	if candidates.is_empty():
		clear()
		return
	if _mode == Mode.PENDING and _source_id == source_minion_id and _candidates == candidates:
		_source_rect = _board_rect(source_minion_id)
		return
	var source := _find_control("board", source_minion_id)
	_mode = Mode.PENDING
	_source_id = source_minion_id
	_source_rect = _local_rect(source.get_global_rect()) if source != null else Rect2()
	_candidates = candidates.duplicate()
	_pointer = _local_point(get_global_mouse_position())
	_pointer_valid = not ScreenProfile.is_touch()
	_pressed = false
	_touch_index = -1
	_hovered = {}
	queue_redraw()


## 关闭瞄准：保留的只是一次操作，不是页面状态；视图刷新后按服务端待选重新开启。
func clear() -> void:
	if _mode == Mode.NONE:
		return
	_mode = Mode.NONE
	_candidates.clear()
	_source_id = ""
	_source_rect = Rect2()
	_pointer_valid = false
	_pressed = false
	_touch_index = -1
	_hovered = {}
	queue_redraw()


# ── 输入 ──────────────────────────────────────────────────────────────────

func _input(event: InputEvent) -> void:
	if not aiming():
		return
	if event is InputEventKey:
		var key := event as InputEventKey
		if key.pressed and not key.echo and key.keycode == KEY_ESCAPE:
			_cancel()
			get_viewport().set_input_as_handled()
		return
	if ScreenProfile.is_touch():
		_input_touch(event)
	else:
		_input_mouse(event)


func _input_mouse(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_set_pointer(_canvas_point((event as InputEventMouseMotion).position))
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.button_index != MOUSE_BUTTON_LEFT:
			return
		# 按下与抬起都吃掉：技能按钮的点击、底层拖拽与其它按钮不参与本次手势。
		get_viewport().set_input_as_handled()
		if button.pressed:
			_pressed = true
			_set_pointer(_canvas_point(button.position))
		else:
			_release()


func _input_touch(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		var touch := event as InputEventScreenTouch
		if _touch_index != -1 and touch.index != _touch_index:
			return
		get_viewport().set_input_as_handled()
		if touch.pressed:
			_touch_index = touch.index
			_pressed = true
			_set_pointer(_canvas_point(touch.position))
		else:
			_touch_index = -1
			_release()
	elif event is InputEventScreenDrag:
		var drag := event as InputEventScreenDrag
		# 技能模式由按钮按下开启，本层没见到起始触点：第一段拖动接管该触点编号。
		if _touch_index == -1 and _pressed:
			_touch_index = drag.index
		if drag.index != _touch_index:
			return
		get_viewport().set_input_as_handled()
		_set_pointer(_canvas_point(drag.position))


## 抬手：技能模式落在非候选处取消，待选模式落在非候选处放弃。
func _release() -> void:
	if not _pressed:
		return
	_pressed = false
	var hit := _hovered.duplicate()
	var released_mode := _mode
	clear()
	if hit.is_empty():
		if released_mode == Mode.PENDING:
			target_abandoned.emit()
		else:
			aim_cancelled.emit()
		return
	target_chosen.emit(hit)


## `Esc`：技能取消、待选放弃，与落空同义。
func _cancel() -> void:
	var cancelled_mode := _mode
	clear()
	if cancelled_mode == Mode.PENDING:
		target_abandoned.emit()
	elif cancelled_mode == Mode.SKILL:
		aim_cancelled.emit()


func _set_pointer(point: Vector2) -> void:
	_pointer = point
	_pointer_valid = true
	_update_hover()
	queue_redraw()


func _update_hover() -> void:
	_hovered = {}
	if not _pointer_valid:
		return
	for candidate in _candidates:
		if not (candidate is Dictionary):
			continue
		var rect := _candidate_rect(candidate)
		if rect.size != Vector2.ZERO and rect.has_point(_pointer):
			_hovered = candidate
			return


# ── 拖动视觉 ──────────────────────────────────────────────────────────────

func _process(delta: float) -> void:
	_time += delta
	if _mode == Mode.NONE or _mode == Mode.DRAG:
		_refresh_drag_mode()
	if _mode == Mode.PENDING:
		# 待选与棋盘重建常在同一帧：入场卡布局落定前 `get_global_rect()` 还是零位置，
		# 起点逐帧按来源卡当前矩形校准，视图重建后也能跟上。
		_source_rect = _board_rect(_source_id)
	if _mode == Mode.PENDING and not _pressed and not ScreenProfile.is_touch():
		_set_pointer(_local_point(get_global_mouse_position()))
	elif _mode != Mode.NONE and _pointer_valid:
		# 视图重建可能移动候选卡：按住期间逐帧重判命中。
		_update_hover()
	if _mode != Mode.NONE:
		queue_redraw()


## 需要目标的手牌拖拽：读 Godot 拖拽数据，只画视觉；落点仍由卡片与棋盘处理。
func _refresh_drag_mode() -> void:
	var viewport := get_viewport()
	var data: Variant = viewport.gui_get_drag_data() if viewport != null and viewport.gui_is_dragging() else null
	if not (data is Dictionary):
		_clear_drag()
		return
	var payload: Dictionary = data
	var targets := Array(payload.get("targets", []))
	var instance_id := str(payload.get("instanceId", ""))
	if targets.is_empty() or instance_id == "":
		_clear_drag()
		return
	var source := _find_control("hand", instance_id)
	if source == null:
		_clear_drag()
		return
	_mode = Mode.DRAG
	# 候选必须复制：`clear()` 会清空数组，直接持有拖拽数据里的数组会污染卡片的载荷。
	_candidates = targets.duplicate()
	_source_id = instance_id
	_source_rect = _local_rect(source.get_global_rect())
	_pointer = _local_point(get_global_mouse_position())
	_pointer_valid = true
	_update_hover()


func _clear_drag() -> void:
	if _mode != Mode.DRAG:
		return
	clear()


# ── 候选查找 ──────────────────────────────────────────────────────────────

## 在页面内查找带目标元数据的控件；拖拽预览不在页面树下，因此不会被命中。
func _find_control(zone: String, id: String) -> Control:
	var root := get_parent()
	if root == null:
		return null
	return _scan(root, zone, id)


func _scan(node: Node, zone: String, id: String) -> Control:
	for child in node.get_children():
		if child is Control:
			var control := child as Control
			if (
				control.has_meta("target_zone")
				and str(control.get_meta("target_zone")) == zone
				and str(control.get_meta("target_id")) == id
				and control.is_visible_in_tree()
			):
				return control
		var found := _scan(child, zone, id)
		if found != null:
			return found
	return null


func _candidate_rect(candidate: Dictionary) -> Rect2:
	var control := _find_control(
		str(candidate.get("zone", "")), str(candidate.get("id", ""))
	)
	return _local_rect(control.get_global_rect()) if control != null else Rect2()


func _board_rect(minion_id: String) -> Rect2:
	var control := _find_control("board", minion_id)
	return _local_rect(control.get_global_rect()) if control != null else Rect2()


# ── 绘制 ──────────────────────────────────────────────────────────────────

func _draw() -> void:
	if _mode == Mode.NONE or not _pointer_valid:
		return
	var accent := UITheme.color("color-accent")
	var muted := UITheme.color("color-text-muted")
	var danger := UITheme.color("color-danger")
	var source := _source_center()
	var curve := _curve_points(source, _pointer)
	var valid := not _hovered.is_empty()
	var line := accent if valid else muted

	_draw_dashed_curve(curve, line)
	_draw_flow(curve, line)
	_draw_source_ring(source, line)
	_draw_candidate_marks(accent)
	_draw_arrow_head(curve, line)
	if valid:
		_draw_hover_outline(_candidate_rect(_hovered), accent)
	elif _pressed:
		_draw_cross(_pointer, danger)


func _source_center() -> Vector2:
	if _source_rect.size != Vector2.ZERO:
		return _source_rect.get_center()
	return _pointer


## 二次贝塞尔：控制点在源与指针连线的中垂线上，始终向上弯。
func _curve_points(from: Vector2, to: Vector2) -> PackedVector2Array:
	var points := PackedVector2Array()
	var delta := to - from
	var length := delta.length()
	var normal := Vector2.ZERO
	if length > 0.001:
		normal = Vector2(-delta.y, delta.x) / length
		if normal.y > 0.0:
			normal = -normal
	var control := (from + to) * 0.5 + normal * length * ARC_BEND
	for index in ARC_SAMPLES + 1:
		var t := float(index) / float(ARC_SAMPLES)
		points.append(from.lerp(control, t).lerp(control.lerp(to, t), t))
	return points


func _arc_width() -> float:
	return UITheme.factor("aim-arc-width", 2.5)


func _draw_dashed_curve(points: PackedVector2Array, color: Color) -> void:
	if points.size() < 2:
		return
	var dash := UITheme.factor("aim-dash", 10.0)
	var gap := UITheme.factor("aim-gap", 6.0)
	var width := _arc_width()
	var drawing := true
	var remaining := dash
	for index in points.size() - 1:
		var start_point := points[index]
		var end_point := points[index + 1]
		var segment := start_point.distance_to(end_point)
		if segment <= 0.0001:
			continue
		var direction := (end_point - start_point) / segment
		var travelled := 0.0
		while travelled < segment:
			var take := minf(remaining, segment - travelled)
			if drawing:
				draw_line(
					start_point + direction * travelled,
					start_point + direction * (travelled + take),
					color,
					width,
					true,
				)
			travelled += take
			remaining -= take
			if remaining <= 0.0001:
				drawing = not drawing
				remaining = dash if drawing else gap


## 沿弧线流动的光点：按时间推进，首尾循环。
func _draw_flow(points: PackedVector2Array, color: Color) -> void:
	if points.size() < 2:
		return
	var radius := UITheme.factor("aim-dot-radius", 2.5)
	var progress := fmod(_time * UITheme.factor("aim-flow-speed", 0.55), 1.0)
	for index in FLOW_DOTS:
		var t := fmod(progress + float(index) / float(FLOW_DOTS), 1.0)
		draw_circle(_point_along(points, t), radius, color)


func _point_along(points: PackedVector2Array, t: float) -> Vector2:
	var position := clampf(t, 0.0, 1.0) * float(points.size() - 1)
	var index := mini(int(position), points.size() - 2)
	return points[index].lerp(points[index + 1], position - float(index))


func _draw_source_ring(center: Vector2, color: Color) -> void:
	draw_arc(
		center,
		UITheme.factor("aim-source-radius", 12.0),
		0.0,
		TAU,
		32,
		color,
		_arc_width(),
		true,
	)


## 每个候选四角金色角标：不遮挡卡面内容，同时标出可落点区域。
func _draw_candidate_marks(color: Color) -> void:
	var mark := UITheme.factor("aim-mark-length", 8.0)
	var inset := UITheme.factor("aim-mark-inset", 3.0)
	var width := _arc_width()
	for candidate in _candidates:
		if not (candidate is Dictionary):
			continue
		var rect := _candidate_rect(candidate)
		if rect.size == Vector2.ZERO:
			continue
		var inner := rect.grow(-inset)
		var corners := [
			[inner.position, Vector2(mark, 0), Vector2(0, mark)],
			[Vector2(inner.end.x, inner.position.y), Vector2(-mark, 0), Vector2(0, mark)],
			[inner.end, Vector2(-mark, 0), Vector2(0, -mark)],
			[Vector2(inner.position.x, inner.end.y), Vector2(mark, 0), Vector2(0, -mark)],
		]
		for corner in corners:
			var origin: Vector2 = corner[0]
			draw_line(origin, origin + Vector2(corner[1]), color, width, true)
			draw_line(origin, origin + Vector2(corner[2]), color, width, true)


## 命中候选的呼吸描边：金色随 `aim-pulse` 起伏。
func _draw_hover_outline(rect: Rect2, color: Color) -> void:
	if rect.size == Vector2.ZERO:
		return
	var period := maxf(UITheme.factor("aim-pulse", 1.2), 0.1)
	var pulse := 0.55 + 0.45 * (0.5 + 0.5 * sin(_time * TAU / period))
	var edge := color
	edge.a = color.a * pulse
	var inset := UITheme.factor("aim-mark-inset", 3.0)
	draw_rect(rect.grow(-inset), edge, false, UITheme.factor("aim-outline-width", 2.0), true)


func _draw_arrow_head(points: PackedVector2Array, color: Color) -> void:
	if points.size() < 2:
		return
	var tip := points[points.size() - 1]
	var previous := points[points.size() - 2]
	var direction := (tip - previous).normalized()
	if direction == Vector2.ZERO:
		return
	var length := UITheme.factor("aim-mark-length", 8.0) * 1.4
	draw_line(tip, tip - direction.rotated(0.45) * length, color, _arc_width(), true)
	draw_line(tip, tip - direction.rotated(-0.45) * length, color, _arc_width(), true)


## 无效落点的红色叉号：替换箭头延续到指针后的终点标记。
func _draw_cross(point: Vector2, color: Color) -> void:
	var size := UITheme.factor("aim-cross", 6.0)
	var width := UITheme.factor("aim-outline-width", 2.0)
	draw_line(point + Vector2(-size, -size), point + Vector2(size, size), color, width, true)
	draw_line(point + Vector2(-size, size), point + Vector2(size, -size), color, width, true)


func _canvas_point(viewport_point: Vector2) -> Vector2:
	var viewport := get_viewport()
	if viewport == null:
		return viewport_point
	return _local_point(viewport.get_canvas_transform().affine_inverse() * viewport_point)


## 全局画布矩形转本层局部矩形；尺寸按本层全局缩放换算。
func _local_rect(global: Rect2) -> Rect2:
	if global.size == Vector2.ZERO:
		return Rect2()
	var scale := get_global_transform().get_scale().x
	return Rect2(_local_point(global.position), global.size / scale)


func _local_point(global: Vector2) -> Vector2:
	return get_global_transform().affine_inverse() * global


## 冒烟只读：当前来源矩形，内容框局部坐标。
func source_rect_local() -> Rect2:
	return _source_rect
