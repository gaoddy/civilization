class_name LobbyActions
extends PanelContainer
## 大厅开局操作：自动匹配、创建房间、房间码输入与加入入口。
##
## 只发出按钮信号并暴露输入框；请求与状态文案由页面处理。

signal match_pressed
signal create_pressed
signal join_pressed

var match_button: Button
var code_input: LineEdit


func _init() -> void:
    name = "LobbyActions"
    custom_minimum_size = Vector2(620, 0)
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))

    var actions := VBoxContainer.new()
    actions.add_theme_constant_override("separation", UITheme.space("space-lg"))
    add_child(actions)

    # 匹配与建房并排，保持操作区高度不变；匹配是主入口。
    var start_row := HBoxContainer.new()
    start_row.add_theme_constant_override("separation", UITheme.space("space-md"))
    actions.add_child(start_row)

    match_button = UIControls.styled_button(tr("ui.lobby.match"), "primary", 50)
    match_button.name = "MatchButton"
    match_button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    match_button.pressed.connect(func(): match_pressed.emit())
    start_row.add_child(match_button)

    var create := UIControls.styled_button(tr("ui.lobby.create_room"), "default", 50)
    create.name = "CreateRoomButton"
    create.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    create.pressed.connect(func(): create_pressed.emit())
    start_row.add_child(create)

    actions.add_child(UIControls.label(tr("ui.lobby.join_section"), UITheme.font_size("font-title"), false))
    var join_row := HBoxContainer.new()
    join_row.add_theme_constant_override("separation", UITheme.space("space-md"))
    actions.add_child(join_row)
    code_input = UIControls.line_edit(tr("ui.lobby.room_code_placeholder"), 220)
    code_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    join_row.add_child(code_input)
    var join := UIControls.button(tr("ui.lobby.join"))
    join.name = "JoinButton"
    join.pressed.connect(func(): join_pressed.emit())
    join_row.add_child(join)
