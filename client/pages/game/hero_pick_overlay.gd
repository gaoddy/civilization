class_name HeroPickOverlay
extends Control
## 对局页选英雄覆盖层（方向 A 全屏遮罩）：标题、倒计时、已选进度、
## 本人候选、8 个座位已选点、确认与超时提示。
##
## 覆盖层铺满屏幕并接收输入；候选选择由本层按指针/长按统一处理，
## 确认只发出 `hero_confirmed`，由页面在连接可用时发送意图并回写锁定态。

signal hero_confirmed(card_id: String)

## 座位点外圈与圆点的尺寸；圆点居中放在外圈内。
const SEAT_DOT_SIZE := 8
const SEAT_DOT_OFFSET := 4

var _self_id := ""
## 载荷 `players[]` 里解析到的本人座位；-1 表示载荷未命中，回退房间快照。
var _payload_seat := -1

var _title: Label
var _timer: Label
var _progress: Label
var _waiting: Label
var _status: Label
var _timeout: Label
var _confirm: Button
var _list: HeroPickList
var _seats: HBoxContainer
var _detail: HeroPickDetailLayer

var _deadline_unix := 0.0
## 上一帧显示的剩余整秒；相同则不重排文字。
var _remaining_second := -1
var _selected := ""
## 由服务端「本人已选」或页面发送成功后的 `set_confirmed` 置真。
var _locked := false

var _pointer := Vector2.ZERO
# 与 DetailLayer 共用同一组长按阈值：覆盖层只判短按选中，详情显示仍归详情层。
var _touch := LongPressTracker.new(DetailLayer.LONG_PRESS_MS, DetailLayer.TOUCH_CANCEL_DISTANCE)


func _init() -> void:
	name = "HeroPickOverlay"
	set_anchors_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build()


func _build() -> void:
	var scrim := ColorRect.new()
	scrim.name = "HeroPickScrim"
	scrim.color = UITheme.alpha("color-bg", 0.82)
	scrim.set_anchors_preset(Control.PRESET_FULL_RECT)
	scrim.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(scrim)

	var center := CenterContainer.new()
	center.name = "HeroPickCenter"
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(center)

	var column := VBoxContainer.new()
	column.name = "HeroPickColumn"
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	column.add_theme_constant_override("separation", UITheme.space("space-md"))
	center.add_child(column)

	_title = UIControls.label(tr("ui.game.hero_pick.title"), UITheme.font_size("font-title-lg"), false)
	_title.name = "HeroPickTitle"
	_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_title.add_theme_color_override("font_color", UITheme.color("color-text"))
	_title.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(_title)

	var status := HBoxContainer.new()
	status.name = "HeroPickStatus"
	status.alignment = BoxContainer.ALIGNMENT_CENTER
	status.mouse_filter = Control.MOUSE_FILTER_IGNORE
	status.add_theme_constant_override("separation", UITheme.space("space-md-plus"))
	column.add_child(status)

	_timer = UIControls.label("00:30", UITheme.font_size("font-title"), false)
	_timer.name = "HeroPickTimer"
	_timer.add_theme_color_override("font_color", UITheme.color("color-accent"))
	_timer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	status.add_child(_timer)

	_progress = UIControls.label("", UITheme.font_size("font-body"), false)
	_progress.name = "HeroPickProgress"
	_progress.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
	_progress.mouse_filter = Control.MOUSE_FILTER_IGNORE
	status.add_child(_progress)

	_waiting = UIControls.label(tr("ui.game.hero_pick.waiting"), UITheme.font_size("font-body-sm"), false)
	_waiting.name = "HeroPickWaiting"
	_waiting.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_waiting.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
	_waiting.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(_waiting)

	# 断线时确认不会发出意图：提示必须留在覆盖层内，不能交给被遮住的页面模态。
	_status = UIControls.label("", UITheme.font_size("font-body-sm"), false)
	_status.name = "HeroPickStatusLine"
	_status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_status.add_theme_color_override("font_color", UITheme.color("color-danger"))
	_status.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_status.visible = false
	column.add_child(_status)

	_list = HeroPickList.new()
	column.add_child(_list)

	_seats = HBoxContainer.new()
	_seats.name = "HeroPickSeats"
	_seats.alignment = BoxContainer.ALIGNMENT_CENTER
	_seats.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_seats.add_theme_constant_override("separation", UITheme.space("space-xs"))
	column.add_child(_seats)

	_timeout = UIControls.label(tr("ui.game.hero_pick.timeout"), UITheme.font_size("font-body-sm"), false)
	_timeout.name = "HeroPickTimeout"
	_timeout.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_timeout.add_theme_color_override("font_color", UITheme.color("color-danger"))
	_timeout.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_timeout.visible = false
	column.add_child(_timeout)

	_confirm = UIControls.styled_button(tr("ui.game.hero_pick.confirm"), "primary", 30)
	_confirm.name = "HeroPickConfirm"
	_confirm.disabled = true
	_confirm.pressed.connect(_on_confirm_pressed)
	column.add_child(_confirm)

	# 详情层位于内容之上但不拦截输入；锚点与详情行由覆盖层设置。
	_detail = HeroPickDetailLayer.new()
	_detail.row = _list
	add_child(_detail)


## payload: HeroPickStarted 原文；self_id 用来定位本人座位点。
func setup(payload: Dictionary, self_id: String) -> void:
	_self_id = self_id
	_deadline_unix = UIFormat.unix_from_rfc3339(str(payload.get("deadline", "")))
	_remaining_second = -1
	_timeout.visible = false
	_status.visible = false
	_timer.add_theme_color_override("font_color", UITheme.color("color-accent"))

	var players := Array(payload.get("players", []))
	_payload_seat = _seat_in_players(players)
	# 服务端「本人已选」是锁定态的唯一权威：重连、他人刷新和本地确认都据此校正。
	_locked = _self_picked(players)
	var ids := _candidate_ids(payload)
	if not _locked and not ids.has(_selected):
		_selected = ""

	_render_seats(players)
	_refresh_progress(players)

	_detail.reset()
	if _list.changed(ids):
		_list.rebuild(ids, _selected, _locked)
	else:
		_list.apply_state(_selected, _locked)
	for card in _list.cards():
		_detail.track(card, {"detail_kind": "hero", "heroId": card.hero_id})
	_list.sync_hover(_pointer)
	_tick_timer()
	_sync_confirm_state()


## 本人是否已确认（按钮置灰、候选锁定）；只在发送成功或服务端回填后调用。
func set_confirmed(card_id: String) -> void:
	_locked = true
	_selected = card_id
	_list.apply_state(_selected, true)
	_detail.hide_panel()
	_waiting.visible = false
	_status.visible = false
	_confirm.text = tr("ui.game.hero_pick.confirmed")
	_confirm.disabled = true


## 断线或发送不可用时的可见反馈：提示留在覆盖层内，不交给被遮住的页面模态。
func show_offline_notice() -> void:
	_status.text = (
		tr("ui.game.status.reconnecting") if Net.socket != null else tr("ui.game.status.disconnected")
	)
	_status.visible = true


func _process(delta: float) -> void:
	_tick_timer()
	if _touch.active() and not _touch.cancelled:
		# 返回值表示长按刚触发；详情展示归详情层，本层只在抬手时据此不选中。
		_touch.advance(delta)


## 倒计时：按截止时刻与本地时钟重算整秒；到 0 停住并显示超时提示。
func _tick_timer() -> void:
	if _deadline_unix <= 0.0:
		return
	var remaining := maxi(0, ceili(_deadline_unix - Time.get_unix_time_from_system()))
	if remaining == _remaining_second:
		return
	_remaining_second = remaining
	_timer.text = _format_seconds(remaining)
	if remaining == 0:
		_timer.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
		_timeout.visible = true
		# 服务端到点自动结算，本地确认已无意义。
		_confirm.disabled = true


func _expired() -> bool:
	return _deadline_unix > 0.0 and _remaining_second <= 0


func _format_seconds(total: int) -> String:
	return "%02d:%02d" % [total / 60, total % 60]


## 本人座位：优先用载荷 `players[].playerId`；匹配局全新进程重连时 `RoomUpdate`
## 与 `MatchFound` 都不会重放，`room_players` 为空、`match_seat` 为 -1，载荷是唯一来源。
func _self_seat() -> int:
	if _payload_seat >= 0:
		return _payload_seat
	for player in GameStore.room_players:
		if str(player.get("playerId", "")) == _self_id:
			return int(player.get("seat", -1))
	var room_seat := GameStore.my_seat()
	if room_seat >= 0:
		return room_seat
	return GameStore.match_seat if GameStore.match_seat >= 0 else -1


## 在载荷 `players[]` 里按 playerId 定位本人座位；找不到返回 -1。
func _seat_in_players(players: Array) -> int:
	if _self_id.is_empty():
		return -1
	for entry in players:
		if entry is Dictionary and str(entry.get("playerId", "")) == _self_id:
			return int(entry.get("seat", -1))
	return -1


func _candidate_ids(payload: Dictionary) -> Array[String]:
	var ids: Array[String] = []
	for value in payload.get("candidates", []):
		ids.append(str(value))
	return ids


func _render_seats(players: Array) -> void:
	UIControls.clear(_seats)
	var self_seat := _self_seat()
	for entry in players:
		if not (entry is Dictionary):
			continue
		var seat := int(entry.get("seat", -1))
		_seats.add_child(_seat_dot(seat, bool(entry.get("picked", false)), seat == self_seat))


## 座位点：已选为绿点，本人用金色外圈；机器人座位不特殊标注。
func _seat_dot(seat: int, picked: bool, is_self: bool) -> Control:
	var holder := Control.new()
	holder.name = "HeroPickSeat%d" % seat
	holder.custom_minimum_size = Vector2(
		SEAT_DOT_SIZE + SEAT_DOT_OFFSET * 2, SEAT_DOT_SIZE + SEAT_DOT_OFFSET * 2
	)
	holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
	holder.set_meta("seat", seat)
	holder.set_meta("picked", picked)
	holder.set_meta("is_self", is_self)

	var ring := Panel.new()
	ring.name = "SeatRing"
	ring.set_anchors_preset(Control.PRESET_FULL_RECT)
	ring.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var ring_style := UITheme.panel_style("bare")
	ring_style.set_content_margin_all(0)
	ring_style.set_border_width_all(2)
	ring_style.border_color = UITheme.color("color-accent")
	ring_style.set_corner_radius_all(UITheme.radius("radius-pill"))
	ring.add_theme_stylebox_override("panel", ring_style)
	ring.visible = is_self
	holder.add_child(ring)

	var dot := Panel.new()
	dot.name = "SeatDot"
	dot.position = Vector2(SEAT_DOT_OFFSET, SEAT_DOT_OFFSET)
	dot.size = Vector2(SEAT_DOT_SIZE, SEAT_DOT_SIZE)
	dot.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var dot_style := UITheme.panel_style("bare")
	dot_style.set_content_margin_all(0)
	dot_style.set_corner_radius_all(UITheme.radius("radius-pill"))
	dot_style.bg_color = (
		UITheme.color("color-success") if picked else UITheme.color("color-text-muted")
	)
	dot.add_theme_stylebox_override("panel", dot_style)
	holder.add_child(dot)
	return holder


func _refresh_progress(players: Array) -> void:
	var picked := 0
	for entry in players:
		if entry is Dictionary and bool(entry.get("picked", false)):
			picked += 1
	_progress.text = tr("ui.game.hero_pick.progress") % picked
	_waiting.visible = not _locked


## 服务端「本人已选」：本人座位点 picked 为真即视为已确认。
func _self_picked(players: Array) -> bool:
	var seat := _self_seat()
	if seat < 0:
		return false
	for entry in players:
		if entry is Dictionary and int(entry.get("seat", -1)) == seat:
			return bool(entry.get("picked", false))
	return false


func _sync_confirm_state() -> void:
	if _locked:
		_confirm.text = tr("ui.game.hero_pick.confirmed")
		_confirm.disabled = true
		return
	_confirm.text = tr("ui.game.hero_pick.confirm")
	_confirm.disabled = _selected == "" or _expired()


## 确认只发出意图；本地锁定由页面在发送成功后回写 `set_confirmed`。
## 断线时发送必然失败，因此先给出覆盖层内可见反馈并保持可重试。
func _on_confirm_pressed() -> void:
	if _locked or _selected == "" or _expired():
		return
	if not Net.is_open():
		show_offline_notice()
		_sync_confirm_state()
		return
	_status.visible = false
	hero_confirmed.emit(_selected)


func _select(hero_id: String) -> void:
	if _locked or hero_id == "":
		return
	_selected = hero_id
	_list.apply_state(_selected, false)
	_sync_confirm_state()


func _input(event: InputEvent) -> void:
	if ScreenProfile.is_touch():
		if event is InputEventScreenTouch:
			if event.pressed:
				_begin_touch(event)
			else:
				_end_touch(event)
		elif event is InputEventScreenDrag:
			_drag_touch(event)
		return
	if event is InputEventMouseButton:
		var mouse := event as InputEventMouseButton
		_pointer = _canvas_point(mouse.position)
		if mouse.pressed and mouse.button_index == MOUSE_BUTTON_LEFT:
			var card := _list.card_at(_pointer)
			if card != null:
				_select(card.hero_id)
	elif event is InputEventMouseMotion:
		_pointer = _canvas_point((event as InputEventMouseMotion).position)
		_list.sync_hover(_pointer)


func _begin_touch(event: InputEventScreenTouch) -> void:
	if _touch.active() or _locked:
		return
	var point := _canvas_point(event.position)
	var card := _list.card_at(point)
	if card == null:
		return
	_touch.begin(event.index, point, card)


func _drag_touch(event: InputEventScreenDrag) -> void:
	if not _touch.matches(event.index):
		return
	_touch.dragged(event.index, _canvas_point(event.position))


func _end_touch(event: InputEventScreenTouch) -> void:
	if not _touch.matches(event.index):
		return
	var card := _touch.target
	var cancelled := _touch.cancelled
	var fired := _touch.fired
	var elapsed := _touch.elapsed_ms
	_touch.cancel()
	if card == null or not is_instance_valid(card) or cancelled or fired or elapsed >= _touch.long_press_ms:
		return
	if _locked or card.locked:
		return
	if not card.get_global_rect().has_point(_canvas_point(event.position)):
		return
	_select(card.hero_id)


## 视口坐标转为画布坐标，与详情层的 `_input` 命中同一坐标系。
func _canvas_point(viewport_point: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform().affine_inverse() * viewport_point
