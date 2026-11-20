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

## 玩法之外追加的横切分类；效果牌不再作为顶层分类。
const CATEGORY_NEUTRAL := "neutral"

## 选中顶层分类后展开的二级筛选：随从与效果牌。
const KIND_ALL := ""
const KIND_MINION := "minion"
const KIND_EFFECT := "effect"

var _ids: Array = []
var _era := 0
var _taxon := ""
var _kind := KIND_ALL
var _selected := ""

var _title: Label
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

    _chapter_bar = GalleryChapterBar.new()
    _chapter_bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _chapter_bar.chapter_selected.connect(_on_chapter_selected)
    header.add_child(_chapter_bar)

    _close_button = UI.styled_button("", "default", 32)
    _close_button.name = "GalleryClose"
    _close_button.pressed.connect(_on_close)
    header.add_child(_close_button)

    var body := HBoxContainer.new()
    body.name = "GalleryBody"
    body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override("separation", UITheme.space("space-lg"))
    root.add_child(body)

    _toc = GalleryToc.new()
    _toc.taxon_selected.connect(_on_taxon_selected)
    _toc.kind_selected.connect(_on_kind_selected)
    body.add_child(_toc)

    _grid = GalleryGrid.new()
    _grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _grid.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _grid.card_selected.connect(_on_card_selected)
    body.add_child(_grid)

    _detail = GalleryDetail.new()
    body.add_child(_detail)


## 载入卡牌集合与页面文案；重置筛选并选中首张。
func set_catalog(card_ids: Array, title: String, close_text: String) -> void:
    _ids = _sorted_ids(card_ids)
    _title.text = title
    _close_button.text = close_text
    _era = 0
    _taxon = ""
    _kind = KIND_ALL
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
    _toc.set_state(
        _categories(),
        _count_by_category(),
        _taxon,
        _kinds(),
        _count_by_kind(),
        _kind,
    )
    var filtered := _filtered_ids()
    if filtered.is_empty():
        _selected = ""
    elif not filtered.has(_selected):
        _selected = str(filtered[0])
    _grid.set_cards(filtered, _selected)
    _detail.show_card(_selected)


func _on_chapter_selected(era: int) -> void:
    _era = era
    _rebuild()


func _on_taxon_selected(root_id: String) -> void:
    _taxon = root_id
    _kind = KIND_ALL
    _rebuild()


func _on_kind_selected(kind: String) -> void:
    _kind = kind
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
        if not _matches_era(entry) or not _matches_category(entry) or not _matches_kind(entry):
            continue
        out.append(str(id))
    return out


func _count_by_era() -> Dictionary:
    var counts := {}
    for id in _ids:
        var entry := CardDB.card(str(id))
        if not _matches_category(entry) or not _matches_kind(entry):
            continue
        counts[0] = int(counts.get(0, 0)) + 1
        var era := _era_index(str(entry.get("era", "")))
        if era <= ERA_IDS.size():
            counts[era] = int(counts.get(era, 0)) + 1
    return counts


func _count_by_category() -> Dictionary:
    var counts := {}
    for id in _ids:
        var entry := CardDB.card(str(id))
        if not _matches_era(entry) or not _matches_kind(entry):
            continue
        counts[""] = int(counts.get("", 0)) + 1
        for key in _category_keys(entry):
            counts[key] = int(counts.get(key, 0)) + 1
    return counts


func _count_by_kind() -> Dictionary:
    var counts := {}
    for id in _ids:
        var entry := CardDB.card(str(id))
        if not _matches_era(entry) or not _matches_category(entry):
            continue
        counts[KIND_ALL] = int(counts.get(KIND_ALL, 0)) + 1
        var kind := _kind_bucket(entry)
        counts[kind] = int(counts.get(kind, 0)) + 1
    return counts


func _matches_era(entry: Dictionary) -> bool:
    if _era == 0:
        return true
    return _era_index(str(entry.get("era", ""))) == _era


func _matches_category(entry: Dictionary) -> bool:
    if _taxon == "":
        return true
    return _category_keys(entry).has(_taxon)


func _matches_kind(entry: Dictionary) -> bool:
    if _kind == KIND_ALL:
        return true
    return _kind_bucket(entry) == _kind


## 一张卡可同时属于自身玩法、`play:` 关联玩法与「中立」横切分类。
func _category_keys(entry: Dictionary) -> Array:
    var keys: Array = []
    var playstyle := str(entry.get("playstyle", ""))
    if playstyle != "" and playstyle != CATEGORY_NEUTRAL:
        keys.append(playstyle)
    for item in entry.get("specialty", []):
        if str(item.get("kind", "")) != "play":
            continue
        var value := str(item.get("value", ""))
        if value != "" and not keys.has(value):
            keys.append(value)
    if playstyle == CATEGORY_NEUTRAL:
        keys.append(CATEGORY_NEUTRAL)
    return keys


func _is_effect(entry: Dictionary) -> bool:
    return str(entry.get("card_type", "")) == "effect"


func _kind_bucket(entry: Dictionary) -> String:
    return KIND_EFFECT if _is_effect(entry) else KIND_MINION


func _categories() -> Array:
    var out: Array = []
    for id in CardDB.playstyles:
        out.append({"id": str(id), "name": CardDB.playstyle_name(str(id))})
    out.append({"id": CATEGORY_NEUTRAL, "name": CardDB.playstyle_name("neutral")})
    return out


func _kinds() -> Array:
    return [
        {"id": KIND_ALL, "name": tr("ui.gallery.kind_all")},
        {"id": KIND_MINION, "name": tr("ui.gallery.kind_minion")},
        {"id": KIND_EFFECT, "name": tr("ui.gallery.kind_effect")},
    ]


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
