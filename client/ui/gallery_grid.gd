class_name GalleryGrid
extends VBoxContainer
## 图鉴卡面网格：6 列滚动网格，按批追加卡面，滚到接近底部时自动加载下一批。
##
## 选中状态由页面传入；本组件发出点击卡牌，不读取筛选条件。

signal card_selected(card_id: String)

var _ids: Array = []
var _selected := ""
var _hero := false
var _loaded := 0
var _tiles: Array = []
var _fill_serial := 0

var _scroll: ScrollContainer
var _body: VBoxContainer
var _grid: GridContainer
var _status: Label
var _empty: Label


func _init() -> void:
    name = "GalleryGrid"
    add_theme_constant_override("separation", UITheme.space("space-sm"))

    _scroll = ScrollContainer.new()
    _scroll.name = "GalleryScroll"
    _scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    _scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
    _scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _scroll.get_v_scroll_bar().value_changed.connect(_on_scroll_changed)
    add_child(_scroll)

    _body = VBoxContainer.new()
    _body.name = "GalleryScrollBody"
    _body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _body.add_theme_constant_override("separation", UITheme.space("space-sm"))
    _scroll.add_child(_body)

    _grid = GridContainer.new()
    _grid.name = "GalleryGridRows"
    _grid.columns = _columns()
    _grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    var gap := "space-md" if _is_phone() else "space-lg"
    _grid.add_theme_constant_override("h_separation", UITheme.space(gap))
    _grid.add_theme_constant_override("v_separation", UITheme.space(gap))
    _body.add_child(_grid)

    _status = UIControls.label("", UITheme.font_size("font-caption"), false)
    _status.name = "GalleryLoadStatus"
    _status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _status.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    _body.add_child(_status)

    _empty = UIControls.empty_hint(tr("ui.gallery.empty"))
    _empty.name = "GalleryEmpty"
    _empty.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    _empty.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _empty.visible = false
    add_child(_empty)


## 用新的卡牌集合重建首批；`selected_id` 不在集合内时页面会选中首张。
## `hero` 为真时卡面走英雄样式（无费用、时代与攻击/生命）。
func set_cards(ids: Array, selected_id: String, hero := false) -> void:
    _ids = ids.duplicate()
    _selected = selected_id
    _hero = hero
    _rebuild()


## 只更新选中描边，不重建已加载的卡面。
func select(selected_id: String) -> void:
    _selected = selected_id
    for tile in _tiles:
        if tile != null and is_instance_valid(tile):
            tile.set_selected(str(tile.card_id) == selected_id)


func _rebuild() -> void:
    UIControls.clear(_grid)
    _tiles.clear()
    _loaded = 0
    var total := _ids.size()
    _scroll.visible = total > 0
    _status.visible = total > 0
    _empty.visible = total == 0
    if total == 0:
        _status.text = ""
        return

    _load_next_batch()
    _queue_fill()


## 内容不足一屏时没有滚动条，用户无法触发加载；这里补足到可滚动或全部加载。
func _queue_fill() -> void:
    _fill_serial += 1
    var serial := _fill_serial
    _scroll.scroll_vertical = 0
    await get_tree().process_frame
    await get_tree().process_frame
    if serial != _fill_serial:
        return
    while _loaded < _ids.size() and _content_height() <= _scroll.size.y:
        _load_next_batch()
        await get_tree().process_frame


func _load_next_batch() -> void:
    if _loaded >= _ids.size():
        return
    var stop := mini(_loaded + _batch_size(), _ids.size())
    for index in range(_loaded, stop):
        var id := str(_ids[index])
        var tile := GalleryCardTile.new(id, id == _selected, _hero)
        tile.card_selected.connect(func(card_id: String) -> void: card_selected.emit(card_id))
        _grid.add_child(tile)
        _tiles.append(tile)
    _loaded = stop
    _update_status()


func _on_scroll_changed(_value: float) -> void:
    if _loaded >= _ids.size():
        return
    var bar := _scroll.get_v_scroll_bar()
    var threshold := float(UITheme.metric("card-height"))
    if bar.value >= bar.max_value - bar.page - threshold:
        _load_next_batch()


func _update_status() -> void:
    var stats := tr("ui.gallery.loaded_stats") % [_loaded, _ids.size()]
    var hint := tr(
        "ui.gallery.load_all_done" if _loaded >= _ids.size() else "ui.gallery.load_more_hint"
    )
    _status.text = "%s · %s" % [stats, hint]


func _content_height() -> float:
    return (
        _grid.get_combined_minimum_size().y
        + UITheme.space("space-sm")
        + _status.get_combined_minimum_size().y
    )


func _is_phone() -> bool:
    return ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE


func _columns() -> int:
    return UITheme.stage_metric("gallery-grid-columns")


func _batch_size() -> int:
    return maxi(1, UITheme.stage_metric("gallery-batch-size"))
