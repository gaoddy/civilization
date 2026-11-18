class_name GalleryToc
extends VBoxContainer
## 图鉴分类目录：界级分类的单选列表，首项为「全部」。
##
## 计数由页面按当前时代筛选换算后传入；本组件只呈现与发出选择。

signal taxon_selected(root_id: String)


func _init() -> void:
    name = "GalleryToc"
    custom_minimum_size = Vector2(UITheme.metric("gallery-toc-width"), 0)
    add_theme_constant_override("separation", UITheme.space("space-xs"))


## `roots` 为界级分类 id 数组；`counts` 以分类 id 为键，空串表示全部。
func set_state(roots: Array, counts: Dictionary, selected: String) -> void:
    UI.clear(self)
    add_child(UI.section_title(tr("ui.gallery.toc_title")))
    var group := ButtonGroup.new()
    group.allow_unpress = false
    _add_item("", tr("ui.gallery.toc_all"), int(counts.get("", 0)), selected == "", group)
    for root in roots:
        var id := str(root)
        var label := "%s %d" % [CardDB.taxon_name(id), int(counts.get(id, 0))]
        _add_item(id, label, int(counts.get(id, 0)), selected == id, group)


func _add_item(id: String, text: String, count: int, selected: bool, group: ButtonGroup) -> void:
    var button := Button.new()
    button.name = "TaxonAll" if id == "" else "Taxon%s" % id
    button.text = text
    button.alignment = HORIZONTAL_ALIGNMENT_LEFT
    button.toggle_mode = true
    button.button_group = group
    button.button_pressed = selected
    button.disabled = count == 0 and not selected
    button.focus_mode = Control.FOCUS_ALL
    button.custom_minimum_size = Vector2(0, 28)
    button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    button.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        button.add_theme_stylebox_override(state, UITheme.button_style("ghost", state))
    button.add_theme_stylebox_override("pressed", _selected_style())
    button.add_theme_stylebox_override("hover_pressed", _selected_style())
    button.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    button.add_theme_color_override("font_hover_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_pressed_color", UITheme.color("color-accent"))
    button.add_theme_color_override("font_hover_pressed_color", UITheme.color("color-accent"))
    button.add_theme_color_override("font_focus_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
    button.pressed.connect(func() -> void: taxon_selected.emit(id))
    add_child(button)


func _selected_style() -> StyleBoxFlat:
    var box := UITheme.panel_style("accent-row")
    box.set_corner_radius_all(UITheme.radius("radius-sm"))
    return box
