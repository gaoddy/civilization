class_name GalleryModeBar
extends HBoxContainer
## 图鉴视图切换：随从 / 英雄双段控件，点击切换到另一视图。
##
## 只发出选择；视图状态、可用性与筛选由页面决定。

signal mode_selected(mode: String)

const MODE_MINION := "minion"
const MODE_HERO := "hero"

var _minion: Button
var _hero: Button
var _group := ButtonGroup.new()


func _init() -> void:
    name = "GalleryModeBar"
    add_theme_constant_override("separation", UITheme.space("space-none"))
    _group.allow_unpress = false
    _minion = _build_button("ModeMinion", tr("ui.gallery.mode_minion"))
    _hero = _build_button("ModeHero", tr("ui.gallery.mode_hero"))


## `mode` 为当前视图；英雄集合为空时禁用英雄段。
func set_state(mode: String, hero_available: bool) -> void:
    _minion.set_pressed_no_signal(mode == MODE_MINION)
    _hero.set_pressed_no_signal(mode == MODE_HERO)
    _hero.disabled = not hero_available
    _apply_style(_minion, mode == MODE_MINION)
    _apply_style(_hero, mode == MODE_HERO and hero_available)


func _build_button(node_name: String, text: String) -> Button:
    var button := UIControls.compact_button(text, "default", 56)
    button.name = node_name
    button.toggle_mode = true
    button.button_group = _group
    button.focus_mode = Control.FOCUS_ALL
    button.pressed.connect(func() -> void:
        mode_selected.emit(MODE_HERO if button == _hero else MODE_MINION)
    )
    add_child(button)
    return button


func _apply_style(button: Button, active: bool) -> void:
    var variant := "primary" if active else "default"
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        button.add_theme_stylebox_override(state, UITheme.button_style(variant, state))
    var font_color := UITheme.color("color-bg" if active else "color-text")
    for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
        button.add_theme_color_override(state, font_color)
    button.add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
