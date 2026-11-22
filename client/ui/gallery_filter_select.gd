class_name GalleryFilterSelect
extends Button
## 图鉴筛选触发器：显示「维度 · 当前值」，点击由页面打开筛选面板。
##
## 只呈现当前值与激活态，不保存筛选条件；条目、计数与选中项由页面给出。

var _dimension := ""


## `node_name` 为节点名；`dimension` 是维度名，例如「时代」。
func _init(node_name: String, dimension: String) -> void:
    name = node_name
    _dimension = dimension
    focus_mode = Control.FOCUS_ALL
    custom_minimum_size = Vector2(0, UITheme.metric("toolbar-button-height"))
    add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    for state in ["normal", "hover", "pressed", "hover_pressed", "focus", "disabled"]:
        add_theme_stylebox_override(state, UITheme.pill_button_style("neutral", state))
    _refresh("", false)


## `value` 为当前筛选展示名；`active` 表示已选非「全部」的筛选项。
func set_state(value: String, active: bool) -> void:
    _refresh(value, active)


func _refresh(value: String, active: bool) -> void:
    text = "%s · %s ▾" % [_dimension, value]
    var font_color := UITheme.color("color-accent" if active else "color-text")
    add_theme_color_override("font_color", font_color)
    add_theme_color_override("font_hover_color", UITheme.color("color-accent"))
    add_theme_color_override("font_pressed_color", UITheme.color("color-accent"))
    add_theme_color_override("font_focus_color", font_color)
