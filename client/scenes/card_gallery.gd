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
## 独立英雄分区：英雄卡只在开局选人中出场，不参与时代与随从/效果二级筛选。
const CATEGORY_HERO := "hero"

## 选中顶层分类后展开的二级筛选：随从与效果牌。
const KIND_ALL := ""
const KIND_MINION := "minion"
const KIND_EFFECT := "effect"

var _ids: Array = []
var _hero_ids: Array = []
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
##
## `hero_ids` 为 `null` 时取全部英雄卡；本局图鉴传入按启用法则过滤后的英雄，
## 传空数组表示本局没有可用英雄（英雄分区显示空态）。
func set_catalog(card_ids: Array, title: String, close_text: String, hero_ids: Variant = null) -> void:
    _ids = _sorted_ids(card_ids)
    _hero_ids = _sorted_hero_ids(CardDB.heroes_array() if hero_ids == null else hero_ids)
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
    var in_hero := _taxon == CATEGORY_HERO
    _chapter_bar.visible = not in_hero
    if not in_hero:
        _chapter_bar.set_state(_count_by_era(), _era)
    _toc.set_state(
        _categories(),
        _count_by_category(),
        _taxon,
        [] if in_hero else _kinds(),
        {} if in_hero else _count_by_kind(),
        _kind,
    )
    var filtered := _filtered_ids()
    if filtered.is_empty():
        _selected = ""
    elif not filtered.has(_selected):
        _selected = str(filtered[0])
    _grid.set_cards(filtered, _selected, in_hero)
    if in_hero:
        _detail.show_hero(_selected)
    else:
        _detail.show_card(_selected)


func _on_chapter_selected(era: int) -> void:
    _era = era
    _rebuild()


func _on_taxon_selected(root_id: String) -> void:
    _taxon = root_id
    _kind = KIND_ALL
    # 英雄没有时代维度：进入英雄分区时清掉时代筛选，避免切回普通分区后无结果。
    if root_id == CATEGORY_HERO:
        _era = 0
    _rebuild()


func _on_kind_selected(kind: String) -> void:
    _kind = kind
    _rebuild()


func _on_card_selected(card_id: String) -> void:
    _selected = card_id
    _grid.select(card_id)
    if _taxon == CATEGORY_HERO:
        _detail.show_hero(card_id)
    else:
        _detail.show_card(card_id)


func _on_close() -> void:
    closed.emit()


func _filtered_ids() -> Array:
    if _taxon == CATEGORY_HERO:
        return _hero_ids.duplicate()
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
    if not _hero_ids.is_empty():
        counts[CATEGORY_HERO] = _hero_ids.size()
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
    if not _hero_ids.is_empty():
        out.append({"id": CATEGORY_HERO, "name": tr("ui.gallery.category_hero")})
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
