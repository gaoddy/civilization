class_name GalleryCategoryPanel
extends Control
## 手机图鉴的分类面板：遮罩 + 两列分类列表，覆盖网格但不挤压布局。
##
## 只呈现分类与计数并发出选择；当前筛选与面板开关由页面决定。

signal taxon_selected(root_id: String)

var _scrim: ColorRect
var _panel: PanelContainer
var _toc: GalleryToc


func _init() -> void:
    name = "GalleryCategoryPanel"
    set_anchors_preset(Control.PRESET_FULL_RECT)
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    visible = false

    _scrim = ColorRect.new()
    _scrim.name = "GalleryCategoryScrim"
    _scrim.set_anchors_preset(Control.PRESET_FULL_RECT)
    _scrim.color = UITheme.alpha("color-bg", 0.62)
    _scrim.mouse_filter = Control.MOUSE_FILTER_STOP
    _scrim.gui_input.connect(_on_scrim_input)
    add_child(_scrim)

    _panel = PanelContainer.new()
    _panel.name = "GalleryCategoryBody"
    _panel.mouse_filter = Control.MOUSE_FILTER_STOP
    _panel.custom_minimum_size = Vector2(UITheme.metric("gallery-category-panel-width"), 0)
    _panel.add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
    add_child(_panel)

    var box := VBoxContainer.new()
    box.name = "GalleryCategoryList"
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    _panel.add_child(box)

    var title := UIControls.label(tr("ui.gallery.category_panel_title"), UITheme.font_size("font-body"), false)
    title.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    box.add_child(title)

    _toc = GalleryToc.new(true)
    _toc.name = "GalleryCategoryToc"
    _toc.taxon_selected.connect(func(root_id: String) -> void:
        close()
        taxon_selected.emit(root_id)
    )
    box.add_child(_toc)


func is_open() -> bool:
    return visible


## 以 `anchor` 为面板左上角，用当前分类与计数重建并显示。
func open(anchor: Vector2, categories: Array, counts: Dictionary, selected: String) -> void:
    _toc.set_state(categories, counts, selected, [], {}, "")
    _panel.position = anchor
    _panel.reset_size()
    visible = true


func close() -> void:
    visible = false


func _on_scrim_input(event: InputEvent) -> void:
    if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
        close()
        accept_event()
