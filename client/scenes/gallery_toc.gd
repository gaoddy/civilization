class_name GalleryToc
extends VBoxContainer
## 图鉴分类目录：顶层分类的单选列表，首项为「全部」，选中项下展开随从 / 效果二级筛选。
##
## 计数由页面按其它两维筛选换算后传入；本组件只呈现与发出选择。

signal taxon_selected(root_id: String)
signal kind_selected(kind: String)


func _init() -> void:
    name = "GalleryToc"
    var token := "gallery-toc-width"
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        token = "gallery-toc-width-compact"
    custom_minimum_size = Vector2(UITheme.metric(token), 0)
    add_theme_constant_override("separation", UITheme.space("space-xs"))


## `categories` 为 `{id, name}` 条目；`counts` 以分类 id 为键，空串表示全部。
## `kinds` 为二级筛选条目；`kind_counts` 以二级 id 为键，空串表示全部。
## 二级筛选只挂在当前选中的顶层分类下。
func set_state(
    categories: Array,
    counts: Dictionary,
    selected: String,
    kinds: Array,
    kind_counts: Dictionary,
    selected_kind: String,
) -> void:
    UI.clear(self)
    add_child(UI.section_title(tr("ui.gallery.toc_title")))
    var group := ButtonGroup.new()
    group.allow_unpress = false
    _add_item("", tr("ui.gallery.toc_all"), int(counts.get("", 0)), selected == "", group, 0)
    if selected == "":
        _add_kinds(kinds, kind_counts, selected_kind)
    for item in categories:
        var id := str(item.get("id", ""))
        var label := "%s %d" % [str(item.get("name", id)), int(counts.get(id, 0))]
        _add_item(id, label, int(counts.get(id, 0)), selected == id, group, 0)
        if selected == id:
            _add_kinds(kinds, kind_counts, selected_kind)


func _add_kinds(kinds: Array, counts: Dictionary, selected: String) -> void:
    var group := ButtonGroup.new()
    group.allow_unpress = false
    var indent := UITheme.space("space-2xl")
    for item in kinds:
        var id := str(item.get("id", ""))
        var label := "%s %d" % [str(item.get("name", id)), int(counts.get(id, 0))]
        _add_item(id, label, int(counts.get(id, 0)), selected == id, group, indent, true)


func _add_item(
    id: String,
    text: String,
    count: int,
    selected: bool,
    group: ButtonGroup,
    indent: int,
    is_kind := false,
) -> void:
    var button := Button.new()
    if is_kind:
        button.name = "KindAll" if id == "" else "Kind%s" % id.capitalize()
    else:
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
        button.add_theme_stylebox_override(state, _indent(UITheme.button_style("ghost", state), indent))
    button.add_theme_stylebox_override("pressed", _selected_style(indent))
    button.add_theme_stylebox_override("hover_pressed", _selected_style(indent))
    button.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    button.add_theme_color_override("font_hover_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_pressed_color", UITheme.color("color-accent"))
    button.add_theme_color_override("font_hover_pressed_color", UITheme.color("color-accent"))
    button.add_theme_color_override("font_focus_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
    button.pressed.connect(func() -> void:
        if is_kind:
            kind_selected.emit(id)
        else:
            taxon_selected.emit(id)
    )
    add_child(button)


func _indent(style: StyleBoxFlat, indent: int) -> StyleBoxFlat:
    if indent > 0:
        style.content_margin_left += indent
    return style


func _selected_style(indent: int) -> StyleBoxFlat:
    var box := UITheme.panel_style("accent-row")
    box.set_corner_radius_all(UITheme.radius("radius-sm"))
    return _indent(box, indent)
