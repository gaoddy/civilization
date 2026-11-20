class_name GalleryModeBar
extends HBoxContainer
## 图鉴视图切换：只显示当前视图的单个按钮，点击切到另一视图。
##
## 只发出选择；视图状态、可用性与筛选由页面决定。

signal mode_selected(mode: String)

const MODE_MINION := "minion"
const MODE_HERO := "hero"

var _mode := MODE_MINION
var _button: Button


func _init() -> void:
    name = "GalleryModeBar"
    _button = UI.styled_button(tr("ui.gallery.mode_minion"), "default", 30)
    _button.name = "ModeToggle"
    _button.custom_minimum_size = Vector2(56, 30)
    _button.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        _button.add_theme_stylebox_override(state, UITheme.button_style("primary", state))
    for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
        _button.add_theme_color_override(state, UITheme.color("color-bg"))
    _button.pressed.connect(_on_pressed)
    add_child(_button)


## `mode` 为当前视图；按钮显示当前视图名称，英雄集合为空时禁用。
func set_state(mode: String, hero_available: bool) -> void:
    _mode = mode
    _button.text = tr("ui.gallery.mode_hero" if mode == MODE_HERO else "ui.gallery.mode_minion")
    _button.disabled = not hero_available


func _on_pressed() -> void:
    mode_selected.emit(MODE_HERO if _mode == MODE_MINION else MODE_MINION)
