class_name CardGallery
extends Control
## 卡牌图鉴：时代章节 + 分类目录 + 卡面网格 + 常驻详情。
##
## 大厅与对局共用的业务组件。卡牌集合、标题、返回文案与副标题由页面传入；
## 本组件不读取房间或对局状态，筛选与选中只影响自身展示。Esc 与返回按钮
## 发出 `closed`，由页面决定是返回大厅还是收起覆盖层。

signal closed

const ERA_IDS := [
    "prehistoric",
    "primitive",
    "classical",
    "feudal",
    "industrial",
    "modern",
    "future",
]

var _ids: Array = []
var _era := 0
var _taxon := ""
var _selected := ""

var _title: Label
var _subtitle: Label
var _count: Label
var _close_button: Button
var _chapter_bar: GalleryChapterBar
var _toc: GalleryToc
var _grid: GalleryGrid
var _detail: GalleryDetail


func _init() -> void:
    name = "CardGallery"
    set_anchors_preset(Control.PRESET_FULL_RECT)
    mouse_filter = Control.MOUSE_FILTER_STOP

    var background := ColorRect.new()
    background.name = "GalleryBackground"
    background.color = UITheme.color("color-bg")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    background.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(background)

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-2xl"))
    add_child(margin)

    var root := VBoxContainer.new()
    root.add_theme_constant_override("separation", UITheme.space("space-md"))
    margin.add_child(root)

    var header := HBoxContainer.new()
    header.name = "GalleryHeader"
    header.add_theme_constant_override("separation", UITheme.space("space-sm"))
    root.add_child(header)

    _title = UI.label("", UITheme.font_size("font-display"), false)
    _title.name = "GalleryTitle"
    header.add_child(_title)

    _subtitle = UI.pill_label("", "accent", "font-caption")
    _subtitle.name = "GallerySubtitle"
    _subtitle.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    header.add_child(_subtitle)

    _count = UI.label("", UITheme.font_size("font-body-sm"), false)
    _count.name = "GalleryCount"
    _count.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    _count.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    header.add_child(_count)

    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    header.add_child(spacer)

    _close_button = UI.styled_button("", "default", 32)
    _close_button.name = "GalleryClose"
    _close_button.pressed.connect(_on_close)
    header.add_child(_close_button)

    _chapter_bar = GalleryChapterBar.new()
    _chapter_bar.chapter_selected.connect(_on_chapter_selected)
    root.add_child(_chapter_bar)

    var body := HBoxContainer.new()
    body.name = "GalleryBody"
    body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override("separation", UITheme.space("space-lg"))
    root.add_child(body)

    _toc = GalleryToc.new()
    _toc.taxon_selected.connect(_on_taxon_selected)
    body.add_child(_toc)

    _grid = GalleryGrid.new()
    _grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _grid.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _grid.card_selected.connect(_on_card_selected)
    body.add_child(_grid)

    _detail = GalleryDetail.new()
    body.add_child(_detail)


## 载入卡牌集合与页面文案；重置筛选并选中首张。
func set_catalog(card_ids: Array, title: String, close_text: String, subtitle: String) -> void:
    _ids = _sorted_ids(card_ids)
    _title.text = title
    _close_button.text = close_text
    _subtitle.text = subtitle
    _era = 0
    _taxon = ""
    _selected = ""
    _rebuild()


func _unhandled_input(event: InputEvent) -> void:
    if not is_visible_in_tree():
        return
    if event.is_action_pressed("ui_cancel"):
        closed.emit()
        get_viewport().set_input_as_handled()


func _rebuild() -> void:
    _chapter_bar.set_state(_count_by_era(), _era)
    _toc.set_state(_roots(), _count_by_taxon(), _taxon)
    var filtered := _filtered_ids()
    if filtered.is_empty():
        _selected = ""
    elif not filtered.has(_selected):
        _selected = str(filtered[0])
    _count.text = tr("ui.gallery.count") % filtered.size()
    _grid.set_cards(filtered, _selected)
    _detail.show_card(_selected)


func _on_chapter_selected(era: int) -> void:
    _era = era
    _rebuild()


func _on_taxon_selected(root_id: String) -> void:
    _taxon = root_id
    _rebuild()


func _on_card_selected(card_id: String) -> void:
    _selected = card_id
    _grid.select(card_id)
    _detail.show_card(card_id)


func _on_close() -> void:
    closed.emit()


func _filtered_ids() -> Array:
    var out: Array = []
    for id in _ids:
        var entry := CardDB.card(str(id))
        if not _matches_era(entry) or not _matches_taxon(entry):
            continue
        out.append(str(id))
    return out


func _count_by_era() -> Dictionary:
    var counts := {}
    for id in _ids:
        var entry := CardDB.card(str(id))
        if not _matches_taxon(entry):
            continue
        counts[0] = int(counts.get(0, 0)) + 1
        var era := _era_index(str(entry.get("era", "")))
        if era <= ERA_IDS.size():
            counts[era] = int(counts.get(era, 0)) + 1
    return counts


func _count_by_taxon() -> Dictionary:
    var counts := {}
    for id in _ids:
        var entry := CardDB.card(str(id))
        if not _matches_era(entry):
            continue
        counts[""] = int(counts.get("", 0)) + 1
        var root := _root_taxon(str(entry.get("taxon", "")))
        if root != "":
            counts[root] = int(counts.get(root, 0)) + 1
    return counts


func _matches_era(entry: Dictionary) -> bool:
    if _era == 0:
        return true
    return _era_index(str(entry.get("era", ""))) == _era


func _matches_taxon(entry: Dictionary) -> bool:
    if _taxon == "":
        return true
    return _root_taxon(str(entry.get("taxon", ""))) == _taxon


func _roots() -> Array:
    var out: Array = []
    for id in CardDB.taxonomy:
        var node: Dictionary = CardDB.taxonomy[id]
        if node.get("parent") == null:
            out.append(str(id))
    return out


func _root_taxon(taxon_id: String) -> String:
    var current := taxon_id
    var guard := 0
    while current != "" and CardDB.taxonomy.has(current) and guard < 8:
        var parent: Variant = CardDB.taxonomy[current].get("parent")
        if parent == null or str(parent) == "":
            return current
        current = str(parent)
        guard += 1
    return current


func _era_index(era_id: String) -> int:
    var index := ERA_IDS.find(era_id)
    return index + 1 if index >= 0 else ERA_IDS.size() + 1


func _sorted_ids(card_ids: Array) -> Array:
    var out: Array = card_ids.duplicate()
    out.sort_custom(func(a: Variant, b: Variant) -> bool:
        var a_entry := CardDB.card(str(a))
        var b_entry := CardDB.card(str(b))
        var a_era := _era_index(str(a_entry.get("era", "")))
        var b_era := _era_index(str(b_entry.get("era", "")))
        if a_era != b_era:
            return a_era < b_era
        return CardDB.display_name(str(a)) < CardDB.display_name(str(b)))
    return out
