class_name GalleryDetail
extends ScrollContainer
## 图鉴详情内容：首行左侧封面、右侧卡名与元信息，效果在封面下方逐行展示。
##
## 只读展示；数据全部来自 `CardDB` 的只读展示表。外层抽屉的尺寸、遮罩与
## 关闭方式由 `GalleryDetailLayer` 提供。

var _box: VBoxContainer


func _init() -> void:
    name = "GalleryDetail"
    horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
    size_flags_horizontal = Control.SIZE_EXPAND_FILL
    size_flags_vertical = Control.SIZE_EXPAND_FILL

    var margin := MarginContainer.new()
    margin.name = "GalleryDetailMargin"
    margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    margin.size_flags_vertical = Control.SIZE_EXPAND_FILL
    var padding := (
        UITheme.space("space-md")
        if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
        else UITheme.space("space-xl")
    )
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, padding)
    add_child(margin)

    _box = VBoxContainer.new()
    _box.name = "GalleryDetailBody"
    _box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    margin.add_child(_box)


## 空 id 显示占位提示；否则重建详情内容。
func show_card(card_id: String) -> void:
    _show(card_id, false)


## 英雄详情：只展示身份与技能，不显示科学分类、时代、费用与初始数值。
func show_hero(card_id: String) -> void:
    _show(card_id, true)


func _show(card_id: String, hero: bool) -> void:
    UIControls.clear(_box)
    if card_id == "":
        _box.alignment = BoxContainer.ALIGNMENT_CENTER
        _box.add_child(UIControls.empty_hint(tr("ui.gallery.detail_empty")))
        return
    _box.alignment = BoxContainer.ALIGNMENT_BEGIN

    var entry := CardDB.hero(card_id) if hero else CardDB.card(card_id)
    _box.add_child(_build_header(card_id, entry, hero))

    if hero:
        _add_divider()
        SkillList.build(_box, entry.get("skills", []))
        return

    var effect := CardDB.display_text(card_id)
    if effect != "":
        _add_divider()
        _box.add_child(UIControls.section_title(tr("ui.gallery.effect")))
        var index := 0
        for line in effect.split("\n"):
            if line.strip_edges() == "":
                continue
            var effect_label := UIControls.label(line, UITheme.font_size("font-body-sm"))
            effect_label.name = "GalleryDetailEffectLine%d" % index
            effect_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
            _box.add_child(effect_label)
            index += 1


## 首行：左侧固定尺寸封面，右侧卡名与信息列。英雄不显示类型、时代、费用与攻防。
func _build_header(card_id: String, entry: Dictionary, hero: bool) -> Control:
    var header := HBoxContainer.new()
    header.name = "GalleryDetailHeader"
    var gap := (
        UITheme.space("space-md")
        if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
        else UITheme.space("space-lg")
    )
    header.add_theme_constant_override("separation", gap)
    header.add_child(_build_preview(card_id, entry, hero))

    var info := VBoxContainer.new()
    info.name = "GalleryDetailInfo"
    info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    info.size_flags_vertical = Control.SIZE_EXPAND_FILL
    info.add_theme_constant_override("separation", UITheme.space("space-sm"))
    header.add_child(info)

    var name_label := UIControls.label(CardDB.display_name(card_id), UITheme.font_size("font-title-lg"))
    name_label.name = "GalleryDetailName"
    info.add_child(name_label)

    if hero:
        var introduction := CardDB.hero_introduction(card_id)
        if introduction != "":
            var intro := UIControls.label(introduction, UITheme.font_size("font-body-sm"))
            intro.name = "GalleryDetailIntroduction"
            intro.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
            info.add_child(intro)
    else:
        info.add_child(_build_meta(card_id, entry))

    _add_row(info, tr("ui.game.detail.playstyle"), _playstyle_text(entry))
    var associations := _association_text(entry)
    if associations != "":
        _add_row(info, tr("ui.game.detail.association"), associations)
    if not hero:
        _add_row(info, tr("ui.game.detail.taxonomy"), _taxonomy_text(entry))
    return header


## 元信息一行：类型 · 时代 · 费用 · 攻/血，全部写成文字，不额外引入图标。
func _build_meta(card_id: String, entry: Dictionary) -> Control:
    var parts: Array[String] = []
    parts.append(_type_text(card_id, entry))
    var era_id := str(entry.get("era", ""))
    if era_id != "":
        parts.append(UIWording.enum_text("era", era_id))
    parts.append(tr("ui.game.detail.cost_value") % CardDB.display_cost(card_id))
    if entry.get("attack") != null or entry.get("health") != null:
        parts.append(
            tr("ui.gallery.stats_labeled") % [
                CardText.stat_text(entry.get("attack")),
                CardText.stat_text(entry.get("health")),
            ],
        )
    var meta := UIControls.label(" · ".join(parts), UITheme.font_size("font-body-sm"))
    meta.name = "GalleryDetailMeta"
    meta.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    return meta


## 封面按卡面比例固定：桌面 160×209、手机 112×146；卡框复用 `CardFrame`，
## 无美术时用玩法色与首字占位。
func _build_preview(card_id: String, entry: Dictionary, hero: bool) -> Control:
    var width := UITheme.stage_metric("gallery-preview-width")
    var size := Vector2(width, UITheme.card_face_height(width))
    var inset := UITheme.stage_metric("card-cover-inset")

    var holder := Control.new()
    holder.name = "GalleryDetailArtFrame"
    holder.custom_minimum_size = size
    holder.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
    holder.mouse_filter = Control.MOUSE_FILTER_IGNORE

    # 封面底色与卡面一致：玩法色压暗，用圆角面板避免方形色块盖住四角圆弧。
    var cover := Panel.new()
    cover.name = "GalleryDetailCover"
    var style := StyleBoxFlat.new()
    style.bg_color = UITheme.playstyle_cover(CardDB.primary_playstyle(entry))
    style.set_corner_radius_all(UITheme.radius("radius-md"))
    style.set_content_margin_all(0)
    cover.add_theme_stylebox_override("panel", style)
    cover.set_anchors_preset(Control.PRESET_FULL_RECT)
    cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
    holder.add_child(cover)

    var texture := _preview_texture(card_id)
    if texture != null:
        var art := TextureRect.new()
        art.name = "GalleryDetailArt"
        art.texture = texture
        art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
        art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
        art.set_anchors_preset(Control.PRESET_FULL_RECT)
        art.offset_left = inset
        art.offset_top = inset
        art.offset_right = -inset
        art.offset_bottom = -inset
        art.mouse_filter = Control.MOUSE_FILTER_IGNORE
        holder.add_child(art)
    else:
        var placeholder := UIControls.label(CardText.initial(card_id), UITheme.font_size("font-hero"), false)
        placeholder.name = "GalleryDetailArtPlaceholder"
        placeholder.set_anchors_preset(Control.PRESET_FULL_RECT)
        placeholder.offset_left = inset
        placeholder.offset_top = inset
        placeholder.offset_right = -inset
        placeholder.offset_bottom = -inset
        placeholder.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
        placeholder.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
        placeholder.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
        placeholder.mouse_filter = Control.MOUSE_FILTER_IGNORE
        holder.add_child(placeholder)

    # 公共卡框压在封面之上：描边与星级和网格、对局卡面一致；英雄没有星级。
    var level := 0 if hero else int(entry.get("level", 1))
    holder.add_child(CardFrame.create(level))
    return holder


func _preview_texture(card_id: String) -> Texture2D:
    return UIArt.texture(CardDB.art_path(card_id, "card_face"))


func _type_text(_card_id: String, entry: Dictionary) -> String:
    return UIWording.enum_text("card_type", str(entry.get("card_type", "")))


func _taxonomy_text(entry: Dictionary) -> String:
    var path := CardDB.taxon_path(str(entry.get("taxon", "")))
    return " · ".join(path) if not path.is_empty() else tr("ui.game.detail.undetermined")


## 玩法行展示全部 `play:` 归属，按固定玩法顺序用「、」分隔。
func _playstyle_text(entry: Dictionary) -> String:
    return CardText.playstyle_text(entry)


## 关联分类行只展示 prey / host，格式为「捕食：…；寄生：…」。
func _association_text(entry: Dictionary) -> String:
    return CardText.association_text(entry)


## 分隔线：1px `color-border`，把首行信息与下方效果区隔开。
func _add_divider() -> void:
    var margin := MarginContainer.new()
    margin.name = "GalleryDetailDivider"
    margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
    margin.add_theme_constant_override("margin_top", UITheme.space("space-sm"))
    margin.add_theme_constant_override("margin_bottom", UITheme.space("space-sm"))
    var line := ColorRect.new()
    line.color = UITheme.color("color-border")
    line.custom_minimum_size = Vector2(0, 1)
    line.mouse_filter = Control.MOUSE_FILTER_IGNORE
    margin.add_child(line)
    _box.add_child(margin)


func _add_row(parent: VBoxContainer, key_text: String, value_text: String) -> void:
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-sm"))
    row.add_child(_key_label(key_text))
    var value := _value_label(value_text)
    value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_child(value)
    parent.add_child(row)


## 属性行标题：桌面固定宽 64、手机 52，`font-caption`，`color-text-muted`，顶部对齐。
func _key_label(text: String) -> Label:
    var key := UIControls.label(text, UITheme.font_size("font-caption"), false)
    var key_width := 52 if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE else 64
    key.custom_minimum_size = Vector2(key_width, 0)
    key.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    key.vertical_alignment = VERTICAL_ALIGNMENT_TOP
    return key


func _value_label(text: String) -> Label:
    return UIControls.label(text, UITheme.font_size("font-body-sm"))
