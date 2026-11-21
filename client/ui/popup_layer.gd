class_name PopupLayer
extends Control
## 弹出层外壳：全屏遮罩、内容宿主、定位与关闭策略，不承载业务内容。
##
## 子类在 `_init` 里调用 `super(placement, scrim_alpha, scrim_tint)`，把面板交给
## `set_content`，再用 `open` / `open_at` / `close` 控制显隐。遮罩颜色与透明度由
## 子类传入，保持各场景既有视觉；`Placement` 只决定内容块落在哪里。

## 点击遮罩关闭时发出；需要区分「用户取消」与「程序关闭」的宿主可监听。
signal dismissed

enum Placement {
	## 内容块在整屏居中。
	CENTER,
	## 内容块左上角贴在 `open_at` 传入的锚点上，并夹紧在图层内。
	ANCHORED,
	## 内容块贴住对应边，另一轴居中。
	EDGE_TOP,
	EDGE_BOTTOM,
	EDGE_LEFT,
	EDGE_RIGHT,
}

const SCRIM_NAME := "PopupScrim"
const BODY_NAME := "PopupBody"

## 当前定位方式；子类在 `_init` 里通过 `super` 传入。
var placement := Placement.CENTER

## 全屏遮罩；鼠标过滤由子类按场景调整。
var scrim: ColorRect

## 内容宿主：`CENTER` 时是 `CenterContainer`，其余定位方式是整屏 `Control`。
var content_host: Control

## 当前内容块；由 `set_content` 设置。
var content: Control

## 点击遮罩是否关闭；默认关闭需要宿主显式打开。
var dismiss_on_scrim := false

## 贴边与锚点定位时内容块与图层边缘的间距。
var edge_margin := 8

var _anchor := Vector2.ZERO


func _init(placement_value := Placement.CENTER, scrim_alpha := 0.55, scrim_tint := Color.BLACK) -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)
	visible = false
	placement = placement_value
	edge_margin = UITheme.space("space-md")

	scrim = ColorRect.new()
	scrim.name = SCRIM_NAME
	scrim.set_anchors_preset(Control.PRESET_FULL_RECT)
	var tint := scrim_tint
	tint.a = scrim_alpha
	scrim.color = tint
	scrim.gui_input.connect(_on_scrim_input)
	add_child(scrim)

	if placement == Placement.CENTER:
		content_host = CenterContainer.new()
	else:
		content_host = Control.new()
	content_host.name = BODY_NAME
	content_host.set_anchors_preset(Control.PRESET_FULL_RECT)
	content_host.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(content_host)

	resized.connect(_place_content)


## 替换内容块；旧内容会随帧释放。
func set_content(node: Control) -> void:
	clear_content()
	content = node
	content_host.add_child(node)
	if placement != Placement.CENTER and not node.resized.is_connected(_place_content):
		node.resized.connect(_place_content)
	_place_content()


## 释放当前内容块。
func clear_content() -> void:
	if content != null and is_instance_valid(content):
		if content.resized.is_connected(_place_content):
			content.resized.disconnect(_place_content)
		content.queue_free()
	content = null


## 显示弹层；贴边与锚点定位会按内容自然尺寸重新摆放。
func open_layer() -> void:
	visible = true
	if placement != Placement.CENTER and content != null:
		content.reset_size()
		_place_content()


## 以 `anchor` 为内容块左上角显示弹层，仅 `ANCHORED` 定位使用。
func open_layer_at(anchor: Vector2) -> void:
	_anchor = anchor
	open_layer()


func close_layer() -> void:
	visible = false


func is_layer_open() -> bool:
	return visible


func _place_content() -> void:
	if content == null or not is_instance_valid(content):
		return
	var limit := size - content.size
	match placement:
		Placement.ANCHORED:
			content.position = Vector2(
				clampf(_anchor.x, edge_margin, maxf(edge_margin, limit.x - edge_margin)),
				clampf(_anchor.y, edge_margin, maxf(edge_margin, limit.y - edge_margin)),
			)
		Placement.EDGE_TOP:
			content.position = Vector2((size.x - content.size.x) * 0.5, edge_margin)
		Placement.EDGE_BOTTOM:
			content.position = Vector2(
				(size.x - content.size.x) * 0.5, size.y - content.size.y - edge_margin
			)
		Placement.EDGE_LEFT:
			content.position = Vector2(edge_margin, (size.y - content.size.y) * 0.5)
		Placement.EDGE_RIGHT:
			content.position = Vector2(
				size.x - content.size.x - edge_margin, (size.y - content.size.y) * 0.5
			)
		_:
			pass


func _on_scrim_input(event: InputEvent) -> void:
	if not dismiss_on_scrim:
		return
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		close_layer()
		dismissed.emit()
		accept_event()
