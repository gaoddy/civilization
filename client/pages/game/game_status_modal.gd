class_name GameStatusModal
extends PopupLayer
## 对局页状态模态：居中提示当前状态，带「知道了」手动关闭。
##
## 只负责显示与关闭，不判断状态优先级；页面决定何时打开、何时收起。

signal acknowledged

var _message: Label
var _detail: Label


func _init() -> void:
	super(PopupLayer.Placement.CENTER, UITheme.opacity("scrim-alpha"))
	name = "StatusModal"
	dismiss_on_scrim = false
	# 状态提示不锁对局：遮罩与本层只压暗，指针与拖拽继续落到下层对局控件。
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	scrim.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_build()


func _build() -> void:
	var panel := UIControls.card_panel("raised")
	panel.name = "StatusPanel"

	var inner := VBoxContainer.new()
	inner.add_theme_constant_override("separation", UITheme.space("space-sm"))
	panel.add_child(UIControls.margin(inner, UITheme.space("space-lg")))

	var width := UITheme.stage_metric("status-dialog-width")
	var text_width := width - UITheme.space("space-lg") * 2

	_message = UIControls.label("", UITheme.font_size("font-body"), true)
	_message.name = "StatusMessage"
	_message.custom_minimum_size = Vector2(text_width, 0)
	_message.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	inner.add_child(_message)

	_detail = UIControls.label("", UITheme.font_size("font-caption"), true)
	_detail.name = "StatusDetail"
	_detail.custom_minimum_size = Vector2(text_width, 0)
	_detail.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_detail.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
	_detail.visible = false
	inner.add_child(_detail)

	var actions := HBoxContainer.new()
	actions.alignment = BoxContainer.ALIGNMENT_END
	inner.add_child(actions)

	var confirm := PillButton.new(tr("ui.common.got_it"), "primary")
	confirm.name = "StatusConfirm"
	confirm.pressed.connect(_on_confirm)
	actions.add_child(confirm)

	set_content(panel)


## `tone`：dim（等待 / 重连）、danger（断线 / 错误）、text（普通提示）。
## `detail` 为协议原文等次要说明，空串时不占位。
func show_status(text: String, tone := "dim", detail := "") -> void:
	_message.text = text
	_message.add_theme_color_override("font_color", _tone_color(tone))
	_detail.text = detail
	_detail.visible = detail != ""
	open_layer()


func hide_status() -> void:
	_message.text = ""
	_detail.text = ""
	_detail.visible = false
	close_layer()


func _on_confirm() -> void:
	close_layer()
	acknowledged.emit()


func _tone_color(tone: String) -> Color:
	match tone:
		"danger":
			return UITheme.color("color-danger")
		"text":
			return UITheme.color("color-text")
		_:
			return UITheme.color("color-text-dim")
