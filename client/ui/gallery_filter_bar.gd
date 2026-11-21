class_name GalleryFilterBar
extends HFlowContainer
## 图鉴顶部筛选栏：横向居中的单选按钮组，供时代章节与英雄分类共用。
##
## 条目、计数与选中项都由页面给出；本组件只呈现与发出选择，不推导分类。

signal item_selected(id: String)

var _button_prefix := ""


## `node_name` 与 `button_prefix` 决定节点名，例如 `GalleryChapters` + `Chapter`。
func _init(node_name: String, button_prefix: String) -> void:
    name = node_name
    _button_prefix = button_prefix
    alignment = FlowContainer.ALIGNMENT_CENTER
    add_theme_constant_override("h_separation", UITheme.space("space-xs"))
    add_theme_constant_override("v_separation", UITheme.space("space-xs"))


## `items` 为 `{id, name}` 条目；`counts` 以 id 为键，空串表示「全部」。
func set_state(items: Array, counts: Dictionary, selected: String) -> void:
    UIControls.clear(self)
    var group := ButtonGroup.new()
    group.allow_unpress = false
    for item in items:
        var id := str(item.get("id", ""))
        var count := int(counts.get(id, 0))
        var text := "%s\n%d" % [str(item.get("name", id)), count]
        _add_item(id, text, count, selected == id, group)


func _add_item(id: String, text: String, count: int, selected: bool, group: ButtonGroup) -> void:
    var button := UIControls.button("")
    button.name = _button_prefix + ("All" if id == "" else id.capitalize())
    button.text = text
    button.toggle_mode = true
    button.button_group = group
    button.button_pressed = selected
    button.disabled = count == 0 and not selected
    button.focus_mode = Control.FOCUS_ALL
    button.custom_minimum_size = Vector2(0, 44)
    button.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        button.add_theme_stylebox_override(state, UITheme.button_style("default", state))
    button.add_theme_stylebox_override("pressed", UITheme.button_style("primary", "normal"))
    button.add_theme_stylebox_override("hover_pressed", UITheme.button_style("primary", "hover"))
    button.add_theme_color_override("font_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_hover_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_pressed_color", UITheme.color("color-bg"))
    button.add_theme_color_override("font_hover_pressed_color", UITheme.color("color-bg"))
    button.add_theme_color_override("font_focus_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
    button.pressed.connect(func() -> void: item_selected.emit(id))
    add_child(button)
