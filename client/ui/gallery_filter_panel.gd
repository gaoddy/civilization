class_name GalleryFilterPanel
extends PopupLayer
## 图鉴筛选面板：挂在筛选触发器下沿的条目列表，条目显示名称与计数。
##
## 只呈现条目并发出选择；筛选状态由页面持有。点击面板外或按 Esc 由页面收起。

signal item_selected(id: String)

var _panel: PanelContainer
var _list: VBoxContainer


func _init() -> void:
    # 下拉不需要压暗页面，透明遮罩只负责接收「点外部关闭」。
    super(PopupLayer.Placement.ANCHORED, 0.0, UITheme.color("color-bg"))
    name = "GalleryFilterPanel"
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    dismiss_on_scrim = true
    edge_margin = UITheme.space("space-md")

    _panel = PanelContainer.new()
    _panel.name = "GalleryFilterBody"
    _panel.mouse_filter = Control.MOUSE_FILTER_STOP
    _panel.custom_minimum_size = Vector2(UITheme.stage_metric("gallery-filter-panel-width"), 0)
    _panel.add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
    set_content(_panel)

    _list = VBoxContainer.new()
    _list.name = "GalleryFilterList"
    _list.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _panel.add_child(_list)


## 以 `anchor` 为面板左上角重建并显示；`prefix` 用于条目节点名，例如 `Chapter`。
func open_at(
    anchor: Vector2, items: Array, counts: Dictionary, selected: String, prefix: String
) -> void:
    UIControls.clear(_list)
    for item in items:
        var id := str(item.get("id", ""))
        _add_item(
            id,
            str(item.get("name", id)),
            int(counts.get(id, 0)),
            selected == id,
            prefix,
        )
    open_layer_at(anchor)


func close() -> void:
    close_layer()


func is_open() -> bool:
    return is_layer_open()


func _add_item(id: String, text: String, count: int, selected: bool, prefix: String) -> void:
    var button := UIControls.button("")
    button.name = prefix + ("All" if id == "" else id.capitalize())
    button.toggle_mode = true
    button.button_pressed = selected
    button.disabled = count == 0 and not selected
    button.focus_mode = Control.FOCUS_ALL
    button.custom_minimum_size = Vector2(0, 30)
    button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        button.add_theme_stylebox_override(state, UITheme.button_style("ghost", state))
    var selected_style := UITheme.panel_style("accent-row")
    selected_style.set_corner_radius_all(UITheme.radius("radius-sm"))
    button.add_theme_stylebox_override("pressed", selected_style)
    button.add_theme_stylebox_override("hover_pressed", selected_style)

    # 名称左、计数右：标签铺在按钮上，不参与按钮最小尺寸。
    var row := HBoxContainer.new()
    row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    row.add_theme_constant_override("separation", UITheme.space("space-sm"))
    row.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    row.offset_left = UITheme.space("space-md-plus")
    row.offset_right = -UITheme.space("space-md-plus")
    button.add_child(row)

    var name_label := UIControls.label(text, UITheme.font_size("font-body-sm"), false)
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_label.add_theme_color_override(
        "font_color",
        UITheme.color("color-accent" if selected else ("color-text-muted" if count == 0 else "color-text-dim")),
    )
    row.add_child(name_label)

    var count_label := UIControls.label(str(count), UITheme.font_size("font-caption"), false)
    count_label.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    row.add_child(count_label)

    button.pressed.connect(func() -> void:
        close()
        item_selected.emit(id)
    )
    _list.add_child(button)
