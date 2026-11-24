class_name LobbyRoomActions
extends VBoxContainer
## 创建房间模式底部操作：房主只有开始与解散，宾客只有准备与离开。
##
## 房主默认已准备由服务端保证，这里不给房主准备按钮，也不发送 `Ready`。

signal ready_toggled
signal start_pressed
signal dissolve_pressed
signal leave_pressed

var ready_button: Button
var start_button: Button
var dissolve_button: Button
var leave_button: Button

var _built_host := -1


func _init() -> void:
    name = "LobbyRoomActions"
    add_theme_constant_override("separation", UITheme.space("space-sm"))


## state: {ready, is_host, connected, can_start, start_reason, human_count, bot_fill}
func set_state(state: Dictionary) -> void:
    var is_host := bool(state.get("is_host", false))
    var connected := bool(state.get("connected", true))
    if _built_host != int(is_host):
        _rebuild(is_host)
    if is_host:
        var can_start := bool(state.get("can_start", false))
        start_button.disabled = not connected or not can_start
        var reason := str(state.get("start_reason", ""))
        start_button.tooltip_text = reason if start_button.disabled else ""
        dissolve_button.disabled = not connected
        return
    ready_button.text = tr("ui.room.unready") if bool(state.get("ready", false)) else tr("ui.room.ready")
    ready_button.disabled = not connected
    leave_button.disabled = not connected


func _rebuild(is_host: bool) -> void:
    for child in get_children():
        remove_child(child)
        child.free()
    ready_button = null
    start_button = null
    dissolve_button = null
    leave_button = null
    _built_host = int(is_host)
    if is_host:
        start_button = _action_button(tr("ui.room.start"), "primary", "StartButton")
        start_button.pressed.connect(func() -> void: start_pressed.emit())
        add_child(start_button)
        dissolve_button = _action_button(tr("ui.room.dissolve"), "ghost", "DissolveButton")
        dissolve_button.pressed.connect(func() -> void: dissolve_pressed.emit())
        add_child(dissolve_button)
        return
    ready_button = _action_button(tr("ui.room.ready"), "primary", "ReadyButton")
    ready_button.pressed.connect(func() -> void: ready_toggled.emit())
    add_child(ready_button)
    leave_button = _action_button(tr("ui.room.leave"), "ghost", "LeaveButton")
    leave_button.pressed.connect(func() -> void: leave_pressed.emit())
    add_child(leave_button)


func _action_button(text: String, variant: String, node_name: String) -> Button:
    var button := UIControls.styled_button(
        text, variant, UITheme.stage_metric("lobby-room-action-height")
    )
    button.name = node_name
    return button
