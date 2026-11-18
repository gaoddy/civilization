class_name GalleryGrid
extends VBoxContainer
## 图鉴卡面网格与分页：每页固定张数，只构建当前页的卡片。
##
## 选中状态由页面传入；本组件发出点击卡牌，不读取筛选条件。

signal card_selected(card_id: String)

var _ids: Array = []
var _selected := ""
var _page := 0
var _tiles: Array = []

var _scroll: ScrollContainer
var _grid: GridContainer
var _empty: Label
var _pager: HBoxContainer
var _prev: Button
var _next: Button
var _page_label: Label


func _init() -> void:
    name = "GalleryGrid"
    add_theme_constant_override("separation", UITheme.space("space-sm"))

    _scroll = ScrollContainer.new()
    _scroll.name = "GalleryScroll"
    _scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    _scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
    add_child(_scroll)

    _grid = GridContainer.new()
    _grid.name = "GalleryGridRows"
    _grid.columns = UITheme.metric("gallery-grid-columns")
    _grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _grid.add_theme_constant_override("h_separation", UITheme.space("space-lg"))
    _grid.add_theme_constant_override("v_separation", UITheme.space("space-lg"))
    _scroll.add_child(_grid)

    _empty = UI.empty_hint(tr("ui.gallery.empty"))
    _empty.name = "GalleryEmpty"
    _empty.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    _empty.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _empty.visible = false
    add_child(_empty)

    _pager = HBoxContainer.new()
    _pager.name = "GalleryPager"
    _pager.alignment = BoxContainer.ALIGNMENT_CENTER
    _pager.add_theme_constant_override("separation", UITheme.space("space-md"))
    _prev = UI.styled_button(tr("ui.gallery.prev"), "default", 28)
    _prev.name = "GalleryPrev"
    _prev.custom_minimum_size = Vector2(72, 28)
    _prev.pressed.connect(func() -> void: _change_page(-1))
    _pager.add_child(_prev)
    _page_label = UI.label("", UITheme.font_size("font-body-sm"), false)
    _page_label.name = "GalleryPageLabel"
    _page_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    _page_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    _pager.add_child(_page_label)
    _next = UI.styled_button(tr("ui.gallery.next"), "default", 28)
    _next.name = "GalleryNext"
    _next.custom_minimum_size = Vector2(72, 28)
    _next.pressed.connect(func() -> void: _change_page(1))
    _pager.add_child(_next)
    add_child(_pager)


## 用新的卡牌集合重建第一页；`selected_id` 不在集合内时页面会选中首张。
func set_cards(ids: Array, selected_id: String) -> void:
    _ids = ids.duplicate()
    _selected = selected_id
    _page = 0
    _rebuild()


## 只更新选中描边，不重建当前页。
func select(selected_id: String) -> void:
    _selected = selected_id
    for tile in _tiles:
        if tile != null and is_instance_valid(tile):
            tile.set_selected(str(tile.card_id) == selected_id)


func _change_page(delta: int) -> void:
    _page += delta
    _rebuild()


func _rebuild() -> void:
    UI.clear(_grid)
    _tiles.clear()
    var total := _ids.size()
    var page_size := maxi(1, UITheme.metric("gallery-page-size"))
    var pages := int(ceil(float(total) / float(page_size))) if total > 0 else 0
    _page = clampi(_page, 0, maxi(pages - 1, 0))

    _scroll.visible = total > 0
    _empty.visible = total == 0
    _pager.visible = total > 0
    if total == 0:
        return

    var start := _page * page_size
    var stop := mini(start + page_size, total)
    for index in range(start, stop):
        var id := str(_ids[index])
        var tile := GalleryCardTile.new(id, id == _selected)
        tile.card_selected.connect(func(card_id: String) -> void: card_selected.emit(card_id))
        _grid.add_child(tile)
        _tiles.append(tile)

    _page_label.text = tr("ui.gallery.page") % [_page + 1, pages]
    _prev.disabled = _page <= 0
    _next.disabled = _page >= pages - 1
    _scroll.scroll_vertical = 0
