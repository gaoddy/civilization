class_name GalleryModeBar
extends HBoxContainer
## 图鉴视图切换：随从 / 英雄两态单选，位于标题右侧。
##
## 只发出选择；视图状态、可用性与筛选由页面决定。

signal mode_selected(mode: String)

const MODE_MINION := "minion"
const MODE_HERO := "hero"

var _minion: Button
var _hero: Button


func _init() -> void:
    name = "GalleryModeBar"
    add_theme_constant_override("separation", UITheme.space("space-xs"))
    var group := ButtonGroup.new()
    group.allow_unpress = false
    _minion = _add_button("ModeMinion", tr("ui.gallery.mode_minion"), MODE_MINION, group)
    _hero = _add_button("ModeHero", tr("ui.gallery.mode_hero"), MODE_HERO, group)


## `mode` 为当前视图；英雄集合为空时禁用英雄项。
func set_state(mode: String, hero_available: bool) -> void:
    _minion.button_pressed = mode == MODE_MINION
    _hero.button_pressed = mode == MODE_HERO
    _hero.disabled = not hero_available


func _add_button(
    node_name: String,
    text: String,
    mode: String,
    group: ButtonGroup,
) -> Button:
    var button := UI.styled_button(text, "default", 30)
    button.name = node_name
    button.toggle_mode = true
    button.button_group = group
    button.custom_minimum_size = Vector2(56, 30)
    button.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    button.add_theme_stylebox_override("pressed", UITheme.button_style("primary", "normal"))
    button.add_theme_stylebox_override("hover_pressed", UITheme.button_style("primary", "hover"))
    button.add_theme_color_override("font_pressed_color", UITheme.color("color-bg"))
    button.add_theme_color_override("font_hover_pressed_color", UITheme.color("color-bg"))
    button.pressed.connect(func() -> void: mode_selected.emit(mode))
    add_child(button)
    return button
