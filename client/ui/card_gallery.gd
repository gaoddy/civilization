class_name CardGallery
extends Control
## 卡牌图鉴：随从 / 英雄视图 + 顶部筛选下拉 + 卡面网格 + 详情浮层。
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

## 随从与英雄两套视图：随从视图按时代与玩法筛选，英雄视图按英雄玩法筛选。
const MODE_MINION := "minion"
const MODE_HERO := "hero"

## 选中顶层分类后展开的二级筛选：随从与效果牌。
const KIND_ALL := ""
const KIND_MINION := "minion"
const KIND_EFFECT := "effect"

var _ids: Array = []
var _hero_ids: Array = []
var _mode := MODE_MINION
var _era := 0
var _taxon := ""
var _kind := KIND_ALL
var _hero_taxon := ""
var _selected := ""

var _mode_bar: GalleryModeBar
var _era_select: GalleryFilterSelect
var _play_select: GalleryFilterSelect
var _kind_select: GalleryFilterSelect
var _count_label: Label
var _close_button: Button
var _grid: GalleryGrid
var _filter_layer: GalleryFilterPanel
var _detail_layer: GalleryDetailLayer
var _background: Control
var _filter_dimension := ""


## 页面已提供整屏背景时关闭组件背景，避免两层背景缩放不一致。
func set_background_visible(value: bool) -> void:
    if _background != null:
        _background.visible = value


func _init() -> void:
    name = "CardGallery"
    set_anchors_preset(Control.PRESET_FULL_RECT)
    mouse_filter = Control.MOUSE_FILTER_STOP

    _background = UIControls.background()
    _background.name = "GalleryBackground"
    add_child(_background)

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
    var header_height := _header_height()
    header.custom_minimum_size = Vector2(0, header_height)
    header.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
    root.add_child(header)

    _mode_bar = GalleryModeBar.new()
    _mode_bar.mode_selected.connect(_on_mode_selected)
    header.add_child(_mode_bar)

    _era_select = GalleryFilterSelect.new("GalleryEraSelect", tr("ui.game.detail.era"))
    _era_select.pressed.connect(func() -> void: _open_filter("era", _era_select))
    header.add_child(_era_select)

    _play_select = GalleryFilterSelect.new("GalleryPlaySelect", tr("ui.game.detail.playstyle"))
    _play_select.pressed.connect(func() -> void: _open_filter("play", _play_select))
    header.add_child(_play_select)

    _kind_select = GalleryFilterSelect.new("GalleryKindSelect", tr("ui.gallery.filter_kind"))
    _kind_select.pressed.connect(func() -> void: _open_filter("kind", _kind_select))
    header.add_child(_kind_select)

    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    header.add_child(spacer)

    _count_label = UIControls.label("", UITheme.font_size("font-caption"), false)
    _count_label.name = "GalleryCount"
    _count_label.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    header.add_child(_count_label)

    _close_button = UIControls.compact_button("")
    _close_button.name = "GalleryClose"
    _close_button.pressed.connect(_on_close)
    header.add_child(_close_button)

    _grid = GalleryGrid.new()
    _grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _grid.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _grid.card_selected.connect(_on_card_selected)
    root.add_child(_grid)

    _filter_layer = GalleryFilterPanel.new()
    _filter_layer.item_selected.connect(_on_filter_selected)
    add_child(_filter_layer)

    _detail_layer = GalleryDetailLayer.new(UITheme.space("space-2xl") + header_height)
    add_child(_detail_layer)


## 头部高度跟随工具行次级按钮：桌面 26；触控端抬到次要操作命中高度。
static func _header_height() -> int:
    return UIControls.compact_button_height()


## 载入卡牌集合与页面文案；重置视图、筛选并选中首张。
##
## `hero_ids` 为 `null` 时取全部英雄卡；本局图鉴传入按启用法则过滤后的英雄，
## 传空数组表示本局没有可用英雄（英雄视图不可进入）。
func set_catalog(card_ids: Array, close_text: String, hero_ids: Variant = null) -> void:
    _ids = _sorted_ids(card_ids)
    _hero_ids = _sorted_hero_ids(CardDB.heroes_array() if hero_ids == null else hero_ids)
    _close_button.text = close_text
    _mode = MODE_MINION
    _era = 0
    _taxon = ""
    _kind = KIND_ALL
    _hero_taxon = ""
    _selected = ""
    _filter_layer.close()
    _detail_layer.close()
    _rebuild()


func _unhandled_input(event: InputEvent) -> void:
    if not is_visible_in_tree():
        return
    if event.is_action_pressed("ui_cancel"):
        if _filter_layer.is_open():
            _filter_layer.close()
        elif _detail_layer.is_open():
            _detail_layer.close()
        else:
            closed.emit()
        get_viewport().set_input_as_handled()


func _rebuild() -> void:
    if _mode == MODE_HERO and _hero_ids.is_empty():
        _mode = MODE_MINION
    var hero := _mode == MODE_HERO
    _mode_bar.set_state(_mode, not _hero_ids.is_empty())
    _era_select.visible = not hero
    _kind_select.visible = not hero
    if hero:
        _play_select.set_state(_hero_taxon_name(), _hero_taxon != "")
    else:
        _era_select.set_state(_era_name(), _era != 0)
        _play_select.set_state(_taxon_name(), _taxon != "")
        _kind_select.set_state(_kind_name(), _kind != KIND_ALL)

    var filtered := _filtered_ids()
    if filtered.is_empty():
        _selected = ""
    elif not filtered.has(_selected):
        _selected = str(filtered[0])
    _grid.set_cards(filtered, _selected, hero)
    _count_label.text = tr("ui.gallery.result_count") % filtered.size()
    _detail_layer.close()


func _on_mode_selected(mode: String) -> void:
    if mode == _mode or (mode == MODE_HERO and _hero_ids.is_empty()):
        return
    _filter_layer.close()
    _detail_layer.close()
    _mode = mode
    _rebuild()


func _on_filter_selected(id: String) -> void:
    match _filter_dimension:
        "era":
            _era = 0 if id == "" else int(id)
        "play":
            if _mode == MODE_HERO:
                _hero_taxon = id
            else:
                _taxon = id
                _kind = KIND_ALL
        "kind":
            _kind = id
    _rebuild()


func _on_card_selected(card_id: String) -> void:
    _selected = card_id
    _grid.select(card_id)
    _detail_layer.open_card(card_id, _mode == MODE_HERO)


func _on_close() -> void:
    closed.emit()


## 打开某个筛选维度面板；再次点击同一维度则收起。
func _open_filter(dimension: String, source: Control) -> void:
    if _filter_layer.is_open() and _filter_dimension == dimension:
        _filter_layer.close()
        return
    _filter_dimension = dimension
    var items: Array
    var counts: Dictionary
    var selected := ""
    var prefix := ""
    match dimension:
        "era":
            items = _chapter_items()
            counts = _count_by_era()
            selected = "" if _era == 0 else str(_era)
            prefix = "Chapter"
        "play":
            if _mode == MODE_HERO:
                items = _hero_category_items()
                counts = _count_by_hero_category()
                selected = _hero_taxon
                prefix = "HeroCategory"
            else:
                items = _play_items()
                counts = _count_by_category()
                selected = _taxon
                prefix = "Taxon"
        _:
            items = _kinds()
            counts = _count_by_kind()
            selected = _kind
            prefix = "Kind"
    var rect := source.get_global_rect()
    var origin := _filter_layer.get_global_rect().position
    var anchor := Vector2(
        rect.position.x - origin.x,
        rect.end.y - origin.y + UITheme.space("space-xs"),
    )
    _filter_layer.open_at(anchor, items, counts, selected, prefix)


func _filtered_ids() -> Array:
    if _mode == MODE_HERO:
        var heroes: Array = []
        for id in _hero_ids:
            var entry := CardDB.hero(str(id))
            if _hero_taxon == "" or CardDB.play_categories(entry).has(_hero_taxon):
                heroes.append(str(id))
        return heroes
    var out: Array = []
    for id in _ids:
        var entry := CardDB.card(str(id))
        if not _matches_era(entry) or not _matches_category(entry) or not _matches_kind(entry):
            continue
        out.append(str(id))
    return out


func _chapter_items() -> Array:
    var out: Array = [{"id": "", "name": tr("ui.gallery.chapter_all")}]
    for era in range(1, ERA_IDS.size() + 1):
        out.append({"id": str(era), "name": UIWording.enum_text("era", ERA_IDS[era - 1])})
    return out


func _play_items() -> Array:
    var out: Array = [{"id": "", "name": tr("ui.gallery.toc_all")}]
    out.append_array(_categories())
    return out


func _hero_category_items() -> Array:
    return _play_items()


func _count_by_era() -> Dictionary:
    var counts := {}
    for id in _ids:
        var entry := CardDB.card(str(id))
        if not _matches_category(entry) or not _matches_kind(entry):
            continue
        counts[""] = int(counts.get("", 0)) + 1
        var era := _era_index(str(entry.get("era", "")))
        if era <= ERA_IDS.size():
            counts[str(era)] = int(counts.get(str(era), 0)) + 1
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


## 英雄按全部 `play:` 归属计数；「全部」按英雄张数去重。
func _count_by_hero_category() -> Dictionary:
    var counts := {"": _hero_ids.size()}
    for id in _hero_ids:
        for playstyle in CardDB.play_categories(CardDB.hero(str(id))):
            counts[playstyle] = int(counts.get(playstyle, 0)) + 1
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


## 一张卡可同时属于多条 `play:` 归属。
func _category_keys(entry: Dictionary) -> Array:
    return CardDB.play_categories(entry)


func _is_effect(entry: Dictionary) -> bool:
    return str(entry.get("card_type", "")) == "effect"


func _kind_bucket(entry: Dictionary) -> String:
    return KIND_EFFECT if _is_effect(entry) else KIND_MINION


func _categories() -> Array:
    var out: Array = []
    for id in CardDB.playstyles:
        out.append({"id": str(id), "name": CardDB.playstyle_name(str(id))})
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


func _era_name() -> String:
    if _era == 0:
        return tr("ui.gallery.chapter_all")
    return UIWording.enum_text("era", ERA_IDS[_era - 1])


func _taxon_name() -> String:
    if _taxon == "":
        return tr("ui.gallery.toc_all")
    return CardDB.playstyle_name(_taxon)


func _hero_taxon_name() -> String:
    if _hero_taxon == "":
        return tr("ui.gallery.toc_all")
    return CardDB.playstyle_name(_hero_taxon)


func _kind_name() -> String:
    if _kind == KIND_ALL:
        return tr("ui.gallery.kind_all")
    if _kind == KIND_EFFECT:
        return tr("ui.gallery.kind_effect")
    return tr("ui.gallery.kind_minion")


## 英雄按展示名排序，与普通卡的「时代 → 名称」顺序区分开。
func _sorted_hero_ids(hero_ids: Array) -> Array:
    var out: Array = []
    for id in hero_ids:
        if CardDB.heroes.has(str(id)):
            out.append(str(id))
    out.sort_custom(func(a: String, b: String) -> bool:
        var a_name := CardDB.display_name(a)
        var b_name := CardDB.display_name(b)
        if a_name != b_name:
            return a_name < b_name
        return a < b)
    return out


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
