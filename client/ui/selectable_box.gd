class_name SelectableBox
extends PanelContainer
## 可选项框：承载任意内容，表达常态、悬停与选中三态描边。
##
## 选中值由调用方设置；本组件不读取业务状态，也不发送业务意图，只在框内点击时发出
## `activated`。描边有两种表达：`INLINE` 直接替换面板描边，`OUTER` 在框外扩出一圈，
## 与房间候选卡、图鉴卡面单元的现有表现一致。

## 左键在框内按下并在阈值内松开时发出；触屏拖拽等超阈位移不算点击。
signal activated

## 选中描边表达方式。
enum Outline {
	## 选中时把面板描边换成 2px 强调色。
	INLINE,
	## 选中时在面板外扩出一圈 2px 强调色描边，面板本身不变。
	OUTER,
}

## 点击判定阈值：按下与松开的画布坐标位移不超过该值才算点击。
const TAP_SLOP := 12.0
## 选中描边宽度。
const SELECTED_BORDER_WIDTH := 2

## 当前是否选中。
var selected := false

var _outline := Outline.INLINE
var _base_style: StyleBoxFlat
var _hovered := false
var _pressed := false
var _pressed_canvas := Vector2.ZERO
var _content: Control
var _frame: Panel


## 设置底板与描边变体；`base_style` 由调用方按场景令牌构建，本组件只在其上叠加状态。
## `hover_enabled` 为假时不接悬停回调，触控端保持无悬停态。
func configure(base_style: StyleBoxFlat, outline := Outline.INLINE, hover_enabled := true) -> void:
	if _content != null:
		return
	_base_style = base_style
	_outline = outline
	_content = Control.new()
	_content.name = "Content"
	_content.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_content)
	if _outline == Outline.OUTER:
		_build_frame()
	_apply_style()
	gui_input.connect(_on_gui_input)
	if hover_enabled:
		mouse_entered.connect(_on_hover_changed.bind(true))
		mouse_exited.connect(_on_hover_changed.bind(false))


## 内容容器：调用方把展示内容挂到这里；选中框与内容互不影响布局。
func content() -> Control:
	return _content


## 切换选中态；`OUTER` 变体只显隐外扩描边，`INLINE` 变体替换面板描边。
func set_selected(value: bool) -> void:
	selected = value
	if _frame != null:
		_frame.visible = value
	_apply_style()


## 悬停时描边改为强调色；`INLINE` 的选中态优先于悬停。
func set_hovered(value: bool) -> void:
	_hovered = value
	_apply_style()


func _build_frame() -> void:
	_frame = Panel.new()
	_frame.name = "SelectedFrame"
	_frame.set_anchors_preset(Control.PRESET_FULL_RECT)
	var grow := UITheme.space("space-2xs")
	_frame.offset_left = -grow
	_frame.offset_top = -grow
	_frame.offset_right = grow
	_frame.offset_bottom = grow
	_frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var frame_style := StyleBoxFlat.new()
	frame_style.bg_color = UITheme.color("color-transparent")
	frame_style.border_color = UITheme.color("color-accent")
	frame_style.set_border_width_all(SELECTED_BORDER_WIDTH)
	frame_style.set_corner_radius_all(UITheme.radius("radius-lg"))
	_frame.add_theme_stylebox_override("panel", frame_style)
	_frame.visible = false
	_content.add_child(_frame)


func _apply_style() -> void:
	if _base_style == null:
		return
	var box: StyleBoxFlat = _base_style.duplicate()
	if _outline == Outline.INLINE and selected:
		box.border_color = UITheme.color("color-accent")
		box.set_border_width_all(SELECTED_BORDER_WIDTH)
	elif _hovered:
		box.border_color = UITheme.color("color-border-strong")
	add_theme_stylebox_override("panel", box)


func _on_hover_changed(value: bool) -> void:
	set_hovered(value)


func _on_gui_input(event: InputEvent) -> void:
	if not (event is InputEventMouseButton):
		return
	var mouse := event as InputEventMouseButton
	if mouse.button_index != MOUSE_BUTTON_LEFT:
		return
	if mouse.pressed:
		_pressed = true
		_pressed_canvas = _canvas_point(mouse.position)
		return
	if not _pressed:
		return
	_pressed = false
	# 比较画布坐标而不是局部坐标：滚动时控件会移动，局部坐标会把滚动量算进位移。
	if _canvas_point(mouse.position).distance_to(_pressed_canvas) <= TAP_SLOP:
		activated.emit()


## 控件局部坐标 → 画布坐标，与视口分发事件时的换算互逆。
func _canvas_point(local: Vector2) -> Vector2:
	return get_global_transform_with_canvas() * local
