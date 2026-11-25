class_name LobbyQuickActions
extends VBoxContainer
## 右栏底部快捷操作：匹配与取消共用主按钮，创建房间在下一行独占整行。
##
## 只按宿主给定的状态切换文案与禁用；点击匹配按钮时把当前排队状态原样发出，
## 由大厅决定入队还是取消。

signal match_toggled(queueing: bool)
signal create_pressed

var match_button: Button
var create_button: Button


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

    create_button = UIControls.styled_button(
        tr("ui.lobby.create_room"), "default", UITheme.stage_metric("lobby-secondary-action-height")
    )
    create_button.name = "CreateRoomButton"
    create_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    create_button.pressed.connect(func() -> void: create_pressed.emit())
    add_child(create_button)
    # 大厅只保留「开始匹配」的引导流光，创建房间等次要入口保持静态边框。
    FrameMaterial.set_guide_preference(create_button, false)


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
    create_button.disabled = not connected or queueing or creating or joining
    # 开始匹配后按钮变为取消，流光随之移除；取消排队回到开始匹配时再恢复。
    FrameMaterial.set_guide_preference(match_button, not queueing)
