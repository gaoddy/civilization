class_name GameToolButtons
extends HBoxContainer
## 对局页右上角悬浮工具组：图鉴、提示与退出对局，各自一枚胶囊按钮。
##
## 只发按钮信号，不构造意图；提示与退出对局跟随连接状态，图鉴保持可用。

signal gallery_pressed
signal hint_pressed
signal leave_game_pressed

var _hint_button: PillButton
var _leave_button: PillButton


func _init() -> void:
	name = "GameTools"
	add_theme_constant_override("separation", UITheme.space("space-xs"))
	alignment = BoxContainer.ALIGNMENT_END

	var gallery := _action_button("GalleryButton", tr("ui.gallery.entry_match"))
	gallery.pressed.connect(func() -> void: gallery_pressed.emit())
	add_child(gallery)

	_hint_button = _action_button("HintButton", tr("ui.guide.hint"))
	_hint_button.pressed.connect(func() -> void: hint_pressed.emit())
	add_child(_hint_button)

	_leave_button = _action_button("LeaveGameButton", tr("ui.room.exit_game"))
	_leave_button.pressed.connect(func() -> void: leave_game_pressed.emit())
	add_child(_leave_button)


## 提示与退出对局跟随连接状态；图鉴保持可用。
func set_connected(connected: bool) -> void:
	_hint_button.disabled = not connected
	_leave_button.disabled = not connected


func _action_button(node_name: String, text: String) -> PillButton:
	var button := PillButton.new(text)
	button.name = node_name
	return button
