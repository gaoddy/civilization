class_name CardGallery
extends Control
## 卡牌图鉴：随从 / 英雄视图 + 顶部筛选 + 卡面网格 + 常驻详情。
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

var _title: Label
var _mode_bar: GalleryModeBar
var _chapter_bar: GalleryFilterBar
var _hero_bar: GalleryFilterBar
var _close_button: Button
var _toc: GalleryToc
var _grid: GalleryGrid
var _detail: GalleryDetail
var _background: Control
var _phone := false
var _toolbar: HBoxContainer
var _category_button: Button
var _kind_bar: GalleryFilterBar
var _category_panel: GalleryCategoryPanel


## 页面已提供整屏背景时关闭组件背景，避免两层背景缩放不一致。
func set_background_visible(value: bool) -> void:
    if _background != null:
        _background.visible = value


func _init() -> void:
    name = "CardGallery"
    _phone = ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
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
    root.add_child(header)

    _title = UIControls.label("", UITheme.font_size("font-display"), false)
    _title.name = "GalleryTitle"
    header.add_child(_title)

    _mode_bar = GalleryModeBar.new()
    _mode_bar.mode_selected.connect(_on_mode_selected)
    header.add_child(_mode_bar)

    _chapter_bar = GalleryFilterBar.new("GalleryChapters", "Chapter")
    _chapter_bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _chapter_bar.item_selected.connect(_on_chapter_selected)
    header.add_child(_chapter_bar)

    _hero_bar = GalleryFilterBar.new("GalleryHeroCategories", "HeroCategory")
    _hero_bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _hero_bar.item_selected.connect(_on_hero_taxon_selected)
    _hero_bar.visible = false
    header.add_child(_hero_bar)

    _close_button = UIControls.styled_button("", "default", 32)
    _close_button.name = "GalleryClose"
    _close_button.pressed.connect(_on_close)
    header.add_child(_close_button)

    var body := HBoxContainer.new()
    body.name = "GalleryBody"
    body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override("separation", UITheme.space("space-lg"))
    root.add_child(body)

    _grid = GalleryGrid.new()
    _grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _grid.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _grid.card_selected.connect(_on_card_selected)

    if _phone:
        # 手机把目录收进分类面板，主体上方只留分类按钮与二级筛选。
        var column := VBoxContainer.new()
        column.name = "GalleryGridColumn"
        column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        column.size_flags_vertical = Control.SIZE_EXPAND_FILL
        column.add_theme_constant_override("separation", UITheme.space("space-sm"))
        body.add_child(column)

        _toolbar = HBoxContainer.new()
        _toolbar.name = "GalleryToolbar"
        _toolbar.add_theme_constant_override("separation", UITheme.space("space-sm"))
        column.add_child(_toolbar)

        _category_button = UIControls.styled_button("", "default", 30)
        _category_button.name = "GalleryCategoryButton"
        _category_button.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
        _category_button.pressed.connect(_on_category_pressed)
        _toolbar.add_child(_category_button)

        _kind_bar = GalleryFilterBar.new("GalleryKinds", "GalleryKind")
        _kind_bar.alignment = FlowContainer.ALIGNMENT_BEGIN
        _kind_bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        _kind_bar.item_selected.connect(_on_kind_selected)
        _toolbar.add_child(_kind_bar)

        column.add_child(_grid)
    else:
        _toc = GalleryToc.new()
        _toc.taxon_selected.connect(_on_taxon_selected)
        _toc.kind_selected.connect(_on_kind_selected)
        body.add_child(_toc)
        body.add_child(_grid)

    _detail = GalleryDetail.new()
    body.add_child(_detail)

    if _phone:
        _category_panel = GalleryCategoryPanel.new()
        _category_panel.taxon_selected.connect(_on_taxon_selected)
        add_child(_category_panel)


## 载入卡牌集合与页面文案；重置视图、筛选并选中首张。
##
## `hero_ids` 为 `null` 时取全部英雄卡；本局图鉴传入按启用法则过滤后的英雄，
## 传空数组表示本局没有可用英雄（英雄视图不可进入）。
func set_catalog(card_ids: Array, title: String, close_text: String, hero_ids: Variant = null) -> void:
    _ids = _sorted_ids(card_ids)
    _hero_ids = _sorted_hero_ids(CardDB.heroes_array() if hero_ids == null else hero_ids)
    _title.text = title
    _close_button.text = close_text
    _mode = MODE_MINION
    _era = 0
    _taxon = ""
    _kind = KIND_ALL
    _hero_taxon = ""
    _selected = ""
    if _category_panel != null:
        _category_panel.close()
    _rebuild()


func _unhandled_input(event: InputEvent) -> void:
    if not is_visible_in_tree():
        return
    if event.is_action_pressed("ui_cancel"):
        if _category_panel != null and _category_panel.is_open():
            _category_panel.close()
        else:
            closed.emit()
        get_viewport().set_input_as_handled()


func _rebuild() -> void:
    if _mode == MODE_HERO and _hero_ids.is_empty():
        _mode = MODE_MINION
    var hero := _mode == MODE_HERO
    _mode_bar.set_state(_mode, not _hero_ids.is_empty())
    _chapter_bar.visible = not hero
    _hero_bar.visible = hero
    if _phone:
        _toolbar.visible = not hero
        if hero and _category_panel.is_open():
            _category_panel.close()
    else:
        _toc.visible = not hero
    if hero:
        _hero_bar.set_state(_hero_category_items(), _count_by_hero_category(), _hero_taxon)
    else:
        _chapter_bar.set_state(_chapter_items(), _count_by_era(), "" if _era == 0 else str(_era))
        if _phone:
            _kind_bar.set_state(_kinds(), _count_by_kind(), _kind)
            _category_button.text = tr("ui.gallery.category_filter") % _taxon_name()
        else:
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
    _grid.set_cards(filtered, _selected, hero)
    if hero:
        _detail.show_hero(_selected)
    else:
        _detail.show_card(_selected)


func _on_mode_selected(mode: String) -> void:
    if mode == _mode or (mode == MODE_HERO and _hero_ids.is_empty()):
        return
    _mode = mode
    _rebuild()


func _on_chapter_selected(era_id: String) -> void:
    _era = 0 if era_id == "" else int(era_id)
    _rebuild()


func _on_taxon_selected(root_id: String) -> void:
    _taxon = root_id
    _kind = KIND_ALL
    _rebuild()


## 手机分类按钮：在工具栏下沿展开或收起分类面板。
func _on_category_pressed() -> void:
    if _category_panel.is_open():
        _category_panel.close()
        return
    var origin := get_global_rect().position
    var toolbar_rect := _toolbar.get_global_rect()
    var anchor := Vector2(
        toolbar_rect.position.x - origin.x,
        toolbar_rect.end.y - origin.y + UITheme.space("space-xs"),
    )
    _category_panel.open(anchor, _categories(), _count_by_category(), _taxon)


func _taxon_name() -> String:
    if _taxon == "":
        return tr("ui.gallery.toc_all")
    return CardDB.playstyle_name(_taxon)


func _on_hero_taxon_selected(root_id: String) -> void:
    _hero_taxon = root_id
    _rebuild()


func _on_kind_selected(kind: String) -> void:
    _kind = kind
    _rebuild()


func _on_card_selected(card_id: String) -> void:
    _selected = card_id
    _grid.select(card_id)
    if _mode == MODE_HERO:
        _detail.show_hero(card_id)
    else:
        _detail.show_card(card_id)


func _on_close() -> void:
    closed.emit()


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


func _hero_category_items() -> Array:
    var out: Array = [{"id": "", "name": tr("ui.gallery.toc_all")}]
    out.append_array(_categories())
    return out


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
