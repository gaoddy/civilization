class_name GameTopBar
extends HBoxContainer
## 对局页顶部工具条：左侧状态文本，右侧图鉴、提示与退出对局。
##
## 只发按钮信号，不构造意图；状态文本由页面写入。

signal gallery_pressed
signal hint_pressed
signal leave_game_pressed

var status: Label

var _hint_button: Button
var _leave_button: Button


func _init() -> void:
    name = "TopBar"
    custom_minimum_size = Vector2(0, UITheme.stage_metric("top-bar-height"))
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_theme_constant_override("separation", UITheme.space("space-sm"))
    alignment = BoxContainer.ALIGNMENT_BEGIN

    status = UI.label("", UITheme.font_size("font-body-sm"), false)
    status.name = "StatusLabel"
    status.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    status.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    status.max_lines_visible = 1
    status.clip_text = true
    status.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    status.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    add_child(status)

    var gallery := _action_button("GalleryButton", tr("ui.gallery.entry_match"))
    gallery.pressed.connect(func() -> void: gallery_pressed.emit())
    add_child(gallery)

    _hint_button = _action_button("HintButton", tr("ui.guide.hint"))
    _hint_button.pressed.connect(func() -> void: hint_pressed.emit())
    add_child(_hint_button)

    _leave_button = _action_button("LeaveGameButton", tr("ui.room.exit_game"))
    _leave_button.pressed.connect(func() -> void: leave_game_pressed.emit())
    add_child(_leave_button)


func set_status(text: String, font_color: Color, tooltip := "") -> void:
    status.text = text
    status.add_theme_color_override("font_color", font_color)
    status.tooltip_text = tooltip


## 提示与退出对局跟随连接状态；图鉴保持可用。
func set_connected(connected: bool) -> void:
    _hint_button.disabled = not connected
    _leave_button.disabled = not connected


func _action_button(node_name: String, text: String) -> Button:
    var button := UI.styled_button(text, "micro", 26)
    button.name = node_name
    button.add_theme_font_size_override("font_size", UITheme.font_size("font-caption"))
    button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    return button
