class_name LobbyPlayerCard
extends PanelContainer
## 右栏顶部玩家卡：头像缩写、显示名与编号；不放段位与入口。

var _initial: Label
var _name: Label
var _code: Label


func _init() -> void:
    name = "LobbyPlayerCard"
    add_theme_stylebox_override("panel", UITheme.panel_style("raised-tight"))

    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    add_child(row)

    var avatar := PanelContainer.new()
    avatar.name = "PlayerAvatar"
    avatar.custom_minimum_size = Vector2(
        UITheme.stage_metric("lobby-player-avatar"), UITheme.stage_metric("lobby-player-avatar")
    )
    avatar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    avatar.add_theme_stylebox_override("panel", UITheme.panel_style("accent"))
    row.add_child(avatar)

    var center := CenterContainer.new()
    avatar.add_child(center)
    _initial = UIControls.label("", UITheme.font_size("font-title-lg"), false)
    _initial.add_theme_color_override("font_color", UITheme.color("color-accent"))
    center.add_child(_initial)

    var info := VBoxContainer.new()
    info.alignment = BoxContainer.ALIGNMENT_CENTER
    info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    info.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    row.add_child(info)
    _name = UIControls.label("", UITheme.font_size("font-title"), false)
    _name.name = "PlayerName"
    info.add_child(_name)
    _code = UIControls.label("", UITheme.font_size("font-body-sm"), false)
    _code.name = "PlayerCode"
    _code.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    info.add_child(_code)


func set_identity(display_name: String, player_code: String) -> void:
    _initial.text = display_name.substr(0, 1) if display_name != "" else "?"
    _name.text = display_name
    _code.text = player_code
