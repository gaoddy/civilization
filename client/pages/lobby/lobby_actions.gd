class_name LobbyActions
extends PanelContainer
## 大厅开局操作：创建房间、房间码输入与加入入口。
##
## 只发出按钮信号并暴露输入框；请求与状态文案由页面处理。

signal create_pressed
signal join_pressed

var code_input: LineEdit


func _init() -> void:
    name = "LobbyActions"
    custom_minimum_size = Vector2(620, 0)
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))

    var actions := VBoxContainer.new()
    actions.add_theme_constant_override("separation", UITheme.space("space-lg"))
    add_child(actions)

    var create := UIControls.styled_button(tr("ui.lobby.create_room"), "primary", 50)
    create.name = "CreateRoomButton"
    create.pressed.connect(func(): create_pressed.emit())
    actions.add_child(create)

    actions.add_child(UIControls.label(tr("ui.lobby.join_section"), UITheme.font_size("font-title"), false))
    var join_row := HBoxContainer.new()
    join_row.add_theme_constant_override("separation", UITheme.space("space-md"))
    actions.add_child(join_row)
    code_input = LineEdit.new()
    code_input.placeholder_text = tr("ui.lobby.room_code_placeholder")
    code_input.custom_minimum_size = Vector2(220, 0)
    code_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    join_row.add_child(code_input)
    var join := UIControls.button(tr("ui.lobby.join"))
    join.name = "JoinButton"
    join.pressed.connect(func(): join_pressed.emit())
    join_row.add_child(join)
