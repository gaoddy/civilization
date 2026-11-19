class_name CardFaceBase
extends RefCounted
## 卡面共享绘制：封面、首字水印、角标行、信息区、冻结蒙层与就地改身材。
##
## 只构造通用部件，不决定场景显示哪些元素；商店、手牌、棋盘与图鉴的卡面
## 组件各自组合本文件的构造入口。

## 角标行在胶囊高度之外留的余量，避免文字贴边被裁切。
const CORNER_BADGE_EXTRA_HEIGHT := 2
## 冰晶细线的不透明度与条数；角度固定 45°，间距取主题令牌。
const FREEZE_CRYSTAL_ALPHA := 0.55
const FREEZE_CRYSTAL_LINES := 10


## 卡面根与封面：玩法色铺满整卡；有美术资源时叠卡面纹理，无图或加载失败时
## 保留纯色封面与首字水印。所有子节点忽略鼠标，点击与悬停由外层卡片决定。
static func create_face(card_id: String, show_desc: bool) -> Control:
    var face_node := Control.new()
    face_node.name = "CardFace"
    face_node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    face_node.size_flags_vertical = Control.SIZE_EXPAND_FILL

    var entry := CardDB.card(card_id)
    var cover := ColorRect.new()
    cover.name = "Cover"
    cover.color = UITheme.playstyle_cover(str(entry.get("playstyle", "")))
    cover.set_anchors_preset(Control.PRESET_FULL_RECT)
    cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.add_child(cover)

    # 有美术资源时在其上叠卡面纹理；无图或加载失败保留上面的纯色封面回退。
    var cover_path := CardDB.art_path(card_id, "card_face")
    if cover_path != "" and ResourceLoader.exists(cover_path):
        var art := ResourceLoader.load(cover_path) as Texture2D
        if art != null:
            var cover_art := TextureRect.new()
            cover_art.name = "CoverArt"
            cover_art.texture = art
            cover_art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
            cover_art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
            cover_art.set_anchors_preset(Control.PRESET_FULL_RECT)
            cover_art.mouse_filter = Control.MOUSE_FILTER_IGNORE
            face_node.add_child(cover_art)

    # 有卡面纹理时不叠首字水印，避免与立绘重叠。
    var has_cover_art := face_node.get_node_or_null("CoverArt") != null
    if not has_cover_art:
        var desc_text := CardDB.display_text(card_id) if show_desc else ""
        var info_height := UITheme.stage_metric(
            "card-info-height" if desc_text != "" else "card-info-compact-height"
        )
        var watermark := UI.label(CardText.initial(card_id), UITheme.font_size("font-hero"), false)
        watermark.name = "CoverMark"
        watermark.set_anchors_preset(Control.PRESET_FULL_RECT)
        watermark.offset_bottom = -info_height
        watermark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
        watermark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
        watermark.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
        watermark.mouse_filter = Control.MOUSE_FILTER_IGNORE
        face_node.add_child(watermark)
    return face_node


## 左上费用、金色/奖励角标与右上时代的角标行；窄卡上左右贴边、互不重叠。
## 潜行状态标识放在右侧时代角标之前，与金色角标分列两端。全部为空时不建行。
static func add_corner_badges(
    face_node: Control,
    cost_text: String,
    era_text: String,
    badge_text: String,
    stealth := false,
) -> void:
    if cost_text == "" and era_text == "" and badge_text == "" and not stealth:
        return
    var margin := UITheme.space("space-xs")
    var row := HBoxContainer.new()
    row.name = "CardCorners"
    row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    row.set_anchors_preset(Control.PRESET_TOP_WIDE)
    row.offset_left = margin
    row.offset_right = -margin
    row.offset_top = margin
    row.offset_bottom = margin + _badge_height()
    face_node.add_child(row)

    if cost_text != "":
        var cost := _corner_badge(cost_text, "accent")
        cost.name = "CostBadge"
        row.add_child(cost)

    if badge_text != "":
        var badge := _corner_badge(badge_text, "accent")
        badge.name = "CardBadge"
        row.add_child(badge)

    var spacer := Control.new()
    spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_child(spacer)

    if stealth:
        var stealth_badge := _corner_badge(
            TranslationServer.translate("ui.game.card.stealth_glyph"), "info"
        )
        stealth_badge.name = "StealthBadge"
        row.add_child(stealth_badge)

    if era_text != "":
        var era := _corner_badge(era_text, "info")
        era.name = "EraBadge"
        row.add_child(era)


## 底部信息区：介绍（最多两行）、居中名称与「攻击 · 类型 · 生命」。
## `show_desc` 为假时不渲染介绍，信息区退回紧凑高度（商店与棋盘卡在手机上
## 放不下两行介绍）。
static func add_info(
    face_node: Control,
    card_id: String,
    attack_text: String,
    health_text: String,
    type_text: String,
    show_desc: bool,
) -> void:
    var desc_text := CardDB.display_text(card_id) if show_desc else ""
    var info_height := UITheme.stage_metric(
        "card-info-height" if desc_text != "" else "card-info-compact-height"
    )
    var info := UI.card_panel("cover-info")
    info.name = "CardInfo"
    info.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
    info.offset_top = -info_height
    info.offset_bottom = 0
    var info_box := VBoxContainer.new()
    info_box.name = "CardInfoBox"
    info_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info_box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    info_box.alignment = BoxContainer.ALIGNMENT_END
    info.add_child(info_box)

    if desc_text != "":
        # 固定高度的承载框隔离 Label 最小尺寸，保证信息区不被长介绍顶高；
        # 介绍最多两行，超长省略。
        var desc_box := Control.new()
        desc_box.name = "CardDescBox"
        desc_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
        desc_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
        var desc_label := UI.label(desc_text, UITheme.font_size("font-caption"))
        desc_label.name = "CardDesc"
        desc_label.set_anchors_preset(Control.PRESET_FULL_RECT)
        desc_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
        desc_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
        desc_label.max_lines_visible = 2
        desc_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
        desc_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
        desc_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
        desc_box.add_child(desc_label)
        info_box.add_child(desc_box)

    var name_label := UI.label(CardDB.display_name(card_id), UITheme.font_size("font-body-sm"), false)
    name_label.name = "CardName"
    name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info_box.add_child(name_label)

    var stats_row := HBoxContainer.new()
    stats_row.name = "CardStats"
    stats_row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stats_row.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    info_box.add_child(stats_row)

    var attack_label := UI.label(attack_text, UITheme.font_size("font-title"), false)
    attack_label.name = "CardAttack"
    attack_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stats_row.add_child(attack_label)

    var type_label := UI.label(type_text, UITheme.font_size("font-caption"), false)
    type_label.name = "CardType"
    type_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    type_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    type_label.max_lines_visible = 1
    type_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    type_label.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    type_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stats_row.add_child(type_label)

    var health_label := UI.label(health_text, UITheme.font_size("font-title"), false)
    health_label.name = "CardHealth"
    health_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stats_row.add_child(health_label)
    face_node.add_child(info)


## 冻结表现：整卡半透明蒙层 + 固定角度与间距的冰晶细线 + 右下角「冻」字。
static func add_freeze(face_node: Control) -> void:
    var overlay := ColorRect.new()
    overlay.name = "FreezeOverlay"
    overlay.color = UITheme.color("color-info")
    # self_modulate 只压暗蒙层自身，不向下继承到冰晶与「冻」字。
    overlay.self_modulate.a = UITheme.opacity("freeze-overlay-alpha")
    overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
    overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.add_child(overlay)

    var crystals := Control.new()
    crystals.name = "FreezeCrystals"
    crystals.mouse_filter = Control.MOUSE_FILTER_IGNORE
    crystals.clip_contents = true
    crystals.set_anchors_preset(Control.PRESET_FULL_RECT)
    overlay.add_child(crystals)

    # 斜向细线：角度与间距由令牌固定，不随卡片尺寸变化。
    var gap := UITheme.metric("freeze-crystal-gap")
    var line_color := UITheme.alpha("color-info", FREEZE_CRYSTAL_ALPHA)
    for index in FREEZE_CRYSTAL_LINES:
        var line := ColorRect.new()
        line.color = line_color
        line.size = Vector2(gap * 2, 1)
        line.position = Vector2(
            -gap + (index % 2) * gap / 2.0,
            -gap + index * gap,
        )
        line.rotation = deg_to_rad(45.0)
        line.mouse_filter = Control.MOUSE_FILTER_IGNORE
        crystals.add_child(line)

    var glyph := UI.label(
        TranslationServer.translate("ui.game.card.freeze_glyph"),
        UITheme.font_size("font-caption"),
        false,
    )
    glyph.name = "FreezeGlyph"
    glyph.add_theme_color_override("font_color", UITheme.color("color-info"))
    glyph.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
    glyph.grow_horizontal = Control.GROW_DIRECTION_BEGIN
    glyph.grow_vertical = Control.GROW_DIRECTION_BEGIN
    var edge := UITheme.space("space-xs")
    glyph.offset_left = -edge
    glyph.offset_top = -edge
    glyph.offset_right = -edge
    glyph.offset_bottom = -edge
    glyph.mouse_filter = Control.MOUSE_FILTER_IGNORE
    overlay.add_child(glyph)


## 战斗演示的就地身材更新：按节点名改文本，不重建整张卡。
static func set_stats(face_node: Control, attack_text: String, health_text: String) -> void:
    var attack_label := face_node.find_child("CardAttack", true, false) as Label
    if attack_label != null:
        attack_label.text = attack_text
    var health_label := face_node.find_child("CardHealth", true, false) as Label
    if health_label != null:
        health_label.text = health_text


## 窄卡角标：比胶囊更紧凑，避免两角标在 112 宽的商店卡上相撞。
static func _corner_badge(text: String, variant: String) -> Label:
    var badge := UI.pill_label(text, variant, "font-caption")
    var style := UITheme.pill_style(variant)
    style.content_margin_left = UITheme.space("space-xs")
    style.content_margin_right = UITheme.space("space-xs")
    badge.add_theme_stylebox_override("normal", style)
    badge.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return badge


static func _badge_height() -> int:
    return UITheme.font_size("font-caption") + UITheme.space("space-2xs") * 2 + CORNER_BADGE_EXTRA_HEIGHT
