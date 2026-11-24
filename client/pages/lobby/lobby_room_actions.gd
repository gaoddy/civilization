class_name LobbyRoomActions
extends VBoxContainer
## 创建房间模式底部操作：房主只有开始与解散。
##
## 房主默认已准备由服务端保证，这里不给房主准备按钮，也不发送 `Ready`；
## 宾客加入房间后进入房间等待页，不在大厅出现操作区。

signal start_pressed
signal dissolve_pressed

var start_button: Button
var dissolve_button: Button

var _start_hint: Label


func _init() -> void:
    name = "LobbyRoomActions"
    add_theme_constant_override("separation", UITheme.space("space-sm"))

    _start_hint = UIControls.label("", UITheme.font_size("font-caption"))
    _start_hint.name = "RoomStartHint"
    _start_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _start_hint.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    _start_hint.visible = false
    add_child(_start_hint)

    start_button = _action_button(tr("ui.room.start"), "primary", "StartButton")
    start_button.pressed.connect(func() -> void: start_pressed.emit())
    add_child(start_button)

    dissolve_button = _action_button(tr("ui.room.dissolve"), "ghost", "DissolveButton")
    dissolve_button.pressed.connect(func() -> void: dissolve_pressed.emit())
    add_child(dissolve_button)


## state: {connected, can_start, start_reason}
func set_state(state: Dictionary) -> void:
    var connected := bool(state.get("connected", true))
    var can_start := bool(state.get("can_start", false))
    start_button.disabled = not connected or not can_start
    var reason := str(state.get("start_reason", ""))
    start_button.tooltip_text = reason if start_button.disabled else ""
    _start_hint.text = reason if start_button.disabled else ""
    _start_hint.visible = _start_hint.text != ""
    dissolve_button.disabled = not connected


func _action_button(text: String, variant: String, node_name: String) -> Button:
    var button := UIControls.styled_button(
        text, variant, UITheme.stage_metric("lobby-room-action-height")
    )
    button.name = node_name
    return button
