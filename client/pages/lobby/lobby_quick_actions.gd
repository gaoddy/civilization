class_name LobbyQuickActions
extends VBoxContainer
## 右栏底部快捷操作：匹配与取消共用主按钮，创建 / 加入在下一行。
##
## 只按宿主给定的状态切换文案与禁用；点击匹配按钮时把当前排队状态原样发出，
## 由大厅决定入队还是取消。

signal match_toggled(queueing: bool)
signal create_pressed
signal join_pressed

var match_button: Button
var create_button: Button
var join_button: Button


func _init() -> void:
    name = "LobbyQuickActions"
    add_theme_constant_override("separation", UITheme.space("space-md"))

    match_button = UIControls.styled_button(
        tr("ui.lobby.start_match"),
        "primary",
        UITheme.stage_metric("lobby-primary-action-height"),
    )
    match_button.name = "MatchButton"
    match_button.pressed.connect(func() -> void: match_toggled.emit(GameStore.queueing))
    add_child(match_button)

    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    add_child(row)

    create_button = UIControls.styled_button(
        tr("ui.lobby.create_room"), "default", UITheme.stage_metric("lobby-secondary-action-height")
    )
    create_button.name = "CreateRoomButton"
    create_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    create_button.pressed.connect(func() -> void: create_pressed.emit())
    row.add_child(create_button)

    join_button = UIControls.styled_button(
        tr("ui.lobby.join_room"), "default", UITheme.stage_metric("lobby-secondary-action-height")
    )
    join_button.name = "JoinRoomButton"
    join_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    join_button.pressed.connect(func() -> void: join_pressed.emit())
    row.add_child(join_button)


func set_state(state: Dictionary) -> void:
    var queueing := bool(state.get("queueing", false))
    var connected := bool(state.get("connected", true))
    var creating := bool(state.get("creating", false))
    var joining := bool(state.get("joining", false))
    var matched := bool(state.get("matched", false))
    match_button.text = (
        tr("ui.lobby.cancel_match") if queueing else tr("ui.lobby.start_match")
    )
    match_button.disabled = not connected or creating or joining or matched
    create_button.disabled = not connected or queueing or creating
    join_button.disabled = not connected or queueing or joining
