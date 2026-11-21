class_name GalleryDetail
extends PanelContainer
## 图鉴常驻详情：选中卡的卡面美术图与类型、分类、玩法、数值、技能文本。
##
## 只读展示；数据全部来自 `CardDB` 的只读展示表。

var _scroll: ScrollContainer
var _box: VBoxContainer


func _init() -> void:
    name = "GalleryDetail"
    var token := "gallery-detail-width"
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        token = "gallery-detail-width-compact"
    custom_minimum_size = Vector2(UITheme.metric(token), 0)
    size_flags_vertical = Control.SIZE_EXPAND_FILL
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))

    _scroll = ScrollContainer.new()
    _scroll.name = "GalleryDetailScroll"
    _scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    _scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
    add_child(_scroll)

    _box = VBoxContainer.new()
    _box.name = "GalleryDetailBody"
    _box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    _scroll.add_child(_box)


## 空 id 显示占位提示；否则重建详情内容。
func show_card(card_id: String) -> void:
    _show(card_id, false)


## 英雄详情：只展示身份与技能，不显示科学分类、时代、费用与初始数值。
func show_hero(card_id: String) -> void:
    _show(card_id, true)


func _show(card_id: String, hero: bool) -> void:
    UI.clear(_box)
    if card_id == "":
        _box.alignment = BoxContainer.ALIGNMENT_CENTER
        _box.add_child(UI.empty_hint(tr("ui.gallery.detail_empty")))
        return
    _box.alignment = BoxContainer.ALIGNMENT_BEGIN

    var entry := CardDB.hero(card_id) if hero else CardDB.card(card_id)
    _box.add_child(_build_preview(card_id, entry, hero))

    var name_label := UI.label(CardDB.display_name(card_id), UITheme.font_size("font-title-lg"))
    name_label.name = "GalleryDetailName"
    name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    # 卡名行：名称居中占满，右侧放时代象形；英雄条目没有 era，自然不建象形。
    var name_row := HBoxContainer.new()
    name_row.name = "GalleryDetailNameRow"
    name_row.add_theme_constant_override("separation", UITheme.space("space-xs"))
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_row.add_child(name_label)
    var name_era := str(entry.get("era", ""))
    if name_era != "":
        var glyph := EraGlyph.create(name_era, UITheme.metric("era-glyph-size-detail"))
        glyph.name = "GalleryDetailEraGlyph"
        name_row.add_child(glyph)
    _box.add_child(name_row)

    if hero:
        var introduction := CardDB.hero_introduction(card_id)
        if introduction != "":
            var intro := UI.label(introduction, UITheme.font_size("font-body-sm"))
            intro.name = "GalleryDetailIntroduction"
            intro.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
            intro.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
            _box.add_child(intro)
        _add_row(tr("ui.game.detail.type"), tr("ui.game.detail.hero"))
        _add_row(tr("ui.game.detail.playstyle"), _playstyle_text(entry))
        var hero_associations := _association_text(entry)
        if hero_associations != "":
            _add_row(tr("ui.game.detail.association"), hero_associations)
        UI.skill_list(_box, entry.get("skills", []))
        return

    var era_id := str(entry.get("era", ""))
    _add_row(tr("ui.game.detail.type"), _type_text(card_id, entry))
    _add_row(tr("ui.game.detail.taxonomy"), _taxonomy_text(entry))
    _add_row(tr("ui.game.detail.playstyle"), _playstyle_text(entry))
    var associations := _association_text(entry)
    if associations != "":
        _add_row(tr("ui.game.detail.association"), associations)
    if era_id != "":
        _add_row(tr("ui.game.detail.era"), UI.enum_text("era", era_id))
    _add_row(tr("ui.game.detail.cost"), tr("ui.game.detail.cost_value") % CardDB.display_cost(card_id))
    if entry.get("attack") != null or entry.get("health") != null:
        _add_row(
            tr("ui.game.detail.initial_stats"),
            tr("ui.gallery.stats") % [
                CardText.stat_text(entry.get("attack")),
                CardText.stat_text(entry.get("health")),
            ],
        )
    var skills := _skill_names(entry)
    if skills != "":
        _add_row(tr("ui.gallery.skills"), skills)
    var effect := CardDB.display_text(card_id)
    if effect != "":
        _box.add_child(UI.section_title(tr("ui.gallery.effect")))
        var effect_label := UI.label(effect, UITheme.font_size("font-body-sm"))
        effect_label.name = "GalleryDetailEffect"
        effect_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
        _box.add_child(effect_label)


## 预览区显示美术大图：普通卡用卡面，英雄用立绘；无美术时用玩法色与首字占位，
## 不再现场拼装卡面组件。
func _build_preview(card_id: String, entry: Dictionary, hero := false) -> Control:
    var holder := CenterContainer.new()
    holder.name = "GalleryDetailPreview"
    holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
    var size := Vector2(
        _preview_metric("gallery-preview-width"),
        _preview_metric("gallery-preview-height"),
    )

    var frame := PanelContainer.new()
    frame.name = "GalleryDetailArtFrame"
    frame.custom_minimum_size = size
    frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
    var style := StyleBoxFlat.new()
    style.bg_color = UITheme.playstyle_cover(CardDB.primary_playstyle(entry))
    style.set_corner_radius_all(UITheme.radius("radius-md"))
    style.set_content_margin_all(UITheme.metric("card-cover-inset"))
    frame.add_theme_stylebox_override("panel", style)
    holder.add_child(frame)

    var texture := _preview_texture(card_id, hero)
    if texture != null:
        var art := TextureRect.new()
        art.name = "GalleryDetailArt"
        art.texture = texture
        art.custom_minimum_size = size
        art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
        art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
        art.mouse_filter = Control.MOUSE_FILTER_IGNORE
        frame.add_child(art)
    else:
        var placeholder := UI.label(CardText.initial(card_id), UITheme.font_size("font-hero"), false)
        placeholder.name = "GalleryDetailArtPlaceholder"
        placeholder.custom_minimum_size = size
        placeholder.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
        placeholder.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
        placeholder.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
        placeholder.mouse_filter = Control.MOUSE_FILTER_IGNORE
        frame.add_child(placeholder)
    return holder


func _preview_texture(card_id: String, hero: bool) -> Texture2D:
    return UI.art_texture(CardDB.art_path(card_id, "portrait" if hero else "card_face"))


func _preview_metric(name: String) -> int:
    var token := name
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        token = name + "-compact"
    return UITheme.metric(token)


func _type_text(_card_id: String, entry: Dictionary) -> String:
    return UI.enum_text("card_type", str(entry.get("card_type", "")))


func _taxonomy_text(entry: Dictionary) -> String:
    var path := CardDB.taxon_path(str(entry.get("taxon", "")))
    return " · ".join(path) if not path.is_empty() else tr("ui.game.detail.undetermined")


## 玩法行展示全部 `play:` 归属，按固定玩法顺序用「、」分隔。
func _playstyle_text(entry: Dictionary) -> String:
    return CardText.playstyle_text(entry)


## 关联分类行只展示 prey / host，格式为「捕食：…；寄生：…」。
func _association_text(entry: Dictionary) -> String:
    return CardText.association_text(entry)


func _skill_names(entry: Dictionary) -> String:
    var names: Array = []
    for skill_id in entry.get("skills", []):
        var name := CardDB.skill_name(str(skill_id))
        if name != "" and not names.has(name):
            names.append(name)
    return "、".join(names)


func _add_row(key_text: String, value_text: String) -> void:
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-sm"))
    var key := UI.label(key_text, UITheme.font_size("font-caption"), false)
    key.custom_minimum_size = Vector2(64, 0)
    key.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    key.vertical_alignment = VERTICAL_ALIGNMENT_TOP
    row.add_child(key)
    var value := UI.label(value_text, UITheme.font_size("font-body-sm"))
    value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_child(value)
    _box.add_child(row)
