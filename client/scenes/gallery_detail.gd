class_name GalleryDetail
extends PanelContainer
## 图鉴常驻详情：选中卡的大卡面与类型、分类、玩法、数值、技能文本。
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
    UI.clear(_box)
    if card_id == "":
        _box.alignment = BoxContainer.ALIGNMENT_CENTER
        _box.add_child(UI.empty_hint(tr("ui.gallery.detail_empty")))
        return
    _box.alignment = BoxContainer.ALIGNMENT_BEGIN

    var entry := CardDB.card(card_id)
    var card := UI.card_panel("raised-tight")
    card.custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
    card.mouse_filter = Control.MOUSE_FILTER_IGNORE
    var era_id := str(entry.get("era", ""))
    card.add_child(GameCardFace.face(
        card_id,
        GameCardFace.stat_text(entry.get("attack")),
        GameCardFace.stat_text(entry.get("health")),
        GameCardFace.playstyle_text(entry),
        tr("ui.game.detail.hero") if bool(entry.get("is_hero", false)) else "",
        str(CardDB.display_cost(card_id)),
        UI.enum_text("era", era_id) if era_id != "" else "",
    ))
    var holder := CenterContainer.new()
    holder.add_child(card)
    _box.add_child(holder)

    var name_label := UI.label(CardDB.display_name(card_id), UITheme.font_size("font-title-lg"))
    name_label.name = "GalleryDetailName"
    name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _box.add_child(name_label)

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
                GameCardFace.stat_text(entry.get("attack")),
                GameCardFace.stat_text(entry.get("health")),
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


func _type_text(card_id: String, entry: Dictionary) -> String:
    if CardDB.tokens.has(card_id):
        return tr("ui.gallery.type_token")
    var text := UI.enum_text("card_type", str(entry.get("card_type", "")))
    if bool(entry.get("is_hero", false)):
        text = "%s · %s" % [text, tr("ui.game.detail.hero")]
    return text


func _taxonomy_text(entry: Dictionary) -> String:
    var path := CardDB.taxon_path(str(entry.get("taxon", "")))
    return " · ".join(path) if not path.is_empty() else tr("ui.game.detail.undetermined")


func _playstyle_text(entry: Dictionary) -> String:
    var name := CardDB.playstyle_name(str(entry.get("playstyle", "")))
    return name if name != "" else tr("ui.game.detail.undetermined")


func _association_text(entry: Dictionary) -> String:
    var names: Array = []
    var specialty: Variant = entry.get("specialty", [])
    if specialty is Array:
        for item in specialty:
            if not (item is Dictionary) or str(item.get("kind", "")) != "play":
                continue
            var name := CardDB.playstyle_name(str(item.get("value", "")))
            if name != "" and not names.has(name):
                names.append(name)
    return "、".join(names)


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
