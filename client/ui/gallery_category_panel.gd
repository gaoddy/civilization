class_name GalleryCategoryPanel
extends PopupLayer
## 手机图鉴的分类面板：遮罩 + 两列分类列表，覆盖网格但不挤压布局。
##
## 只呈现分类与计数并发出选择；当前筛选与面板开关由页面决定。遮罩、锚点定位
## 与点击遮罩关闭由 `PopupLayer` 承担。

signal taxon_selected(root_id: String)

var _panel: PanelContainer
var _toc: GalleryToc


func _init() -> void:
    super(PopupLayer.Placement.ANCHORED, UITheme.opacity("scrim-alpha-gallery"), UITheme.color("color-bg"))
    name = "GalleryCategoryPanel"
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    dismiss_on_scrim = true
    # 锚点由工具栏位置换算，保持原有贴边位置；越界时仍由外壳夹紧。
    edge_margin = 0

    _panel = PanelContainer.new()
    _panel.name = "GalleryCategoryBody"
    _panel.mouse_filter = Control.MOUSE_FILTER_STOP
    _panel.custom_minimum_size = Vector2(UITheme.stage_metric("gallery-category-panel-width"), 0)
    _panel.add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
    set_content(_panel)

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


## 以 `anchor` 为面板左上角，用当前分类与计数重建并显示。
func open(anchor: Vector2, categories: Array, counts: Dictionary, selected: String) -> void:
    _toc.set_state(categories, counts, selected, [], {}, "")
    open_layer_at(anchor)


func close() -> void:
    close_layer()


func is_open() -> bool:
    return is_layer_open()
