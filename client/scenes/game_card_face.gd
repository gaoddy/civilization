class_name GameCardFace
extends RefCounted
## 卡面与卡牌文案的共享构造：封面、名称与「攻击 · 类型 · 生命」信息区。
##
## 只构造控件，不持有页面状态；商店、棋盘与战斗只读卡都从这里取卡面。

## 角标行在胶囊高度之外留的余量，避免文字贴边被裁切。
const CORNER_BADGE_EXTRA_HEIGHT := 2
## 冰晶细线的不透明度与条数；角度固定 45°，间距取主题令牌。
const FREEZE_CRYSTAL_ALPHA := 0.55
const FREEZE_CRYSTAL_LINES := 10

## 统一卡面：封面铺满整卡，左上费用、右上时代，底部信息区依次为介绍（最多
## 两行）、居中名称与「攻击 · 类型 · 生命」。`cost_text`/`era_text` 为空时
## 不渲染角标；`stealth` 为真时在角标行右侧追加潜行状态标识；`frozen` 为真时
## 盖整卡蒙层、叠冰晶并保留右下「冻」字。
## 所有子节点忽略鼠标，点击与悬停由外层卡片决定。
static func face(
    card_id: String,
    attack_text: String,
    health_text: String,
    type_text: String,
    badge_text := "",
    cost_text := "",
    era_text := "",
    frozen := false,
    stealth := false,
) -> Control:
    var entry := CardDB.card(card_id)
    var desc_text := CardDB.display_text(card_id)
    var info_height := UITheme.metric(
        "card-info-height" if desc_text != "" else "card-info-compact-height"
    )
    var face_node := Control.new()
    face_node.name = "CardFace"
    face_node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    face_node.size_flags_vertical = Control.SIZE_EXPAND_FILL

    var cover := ColorRect.new()
    cover.name = "Cover"
    cover.color = UITheme.playstyle_cover(str(entry.get("playstyle", "")))
    cover.set_anchors_preset(Control.PRESET_FULL_RECT)
    cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.add_child(cover)

    var watermark := UI.label(initial(card_id), UITheme.font_size("font-hero"), false)
    watermark.name = "CoverMark"
    watermark.set_anchors_preset(Control.PRESET_FULL_RECT)
    watermark.offset_bottom = -info_height
    watermark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    watermark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    watermark.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
    watermark.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.add_child(watermark)

    if cost_text != "" or era_text != "" or badge_text != "" or stealth:
        _add_corner_badges(face_node, cost_text, era_text, badge_text, stealth)

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

    if frozen:
        _add_freeze_overlay(face_node)
    return face_node


## 左上费用、金色/奖励角标与右上时代的角标行；窄卡上左右贴边、互不重叠。
## 潜行状态标识放在右侧时代角标之前，与金色角标分列两端。
static func _add_corner_badges(
    face_node: Control,
    cost_text: String,
    era_text: String,
    badge_text: String,
    stealth := false,
) -> void:
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


## 冻结表现：整卡半透明蒙层 + 固定角度与间距的冰晶细线 + 右下角「冻」字。
static func _add_freeze_overlay(face_node: Control) -> void:
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


## 战斗演示的只读随从卡：不带拖拽数据，也不可交互。
static func read_only_minion_card(minion: Dictionary) -> Control:
    var minion_index := int(minion.get("index", 0))
    var panel := BoardCard.new()
    panel.add_theme_stylebox_override("panel", UITheme.panel_style("raised-tight"))
    panel.name = "Board" + ("Minion%d" % minion_index)
    panel.set_meta("minion_id", str(minion.get("minionId", "")))
    panel.set_meta("minion_index", minion_index)
    panel.custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))

    var card_id := str(minion.get("cardId", ""))
    var entry := CardDB.card(card_id)
    panel.add_child(face(
        card_id,
        stat_text(minion.get("attack")),
        stat_text(minion.get("health")),
        playstyle_text(entry),
        "★" if bool(minion.get("golden", false)) else "",
        "",
        "",
        false,
        bool(minion.get("stealth", false)),
    ))
    return panel


## 卡面封面的首字水印；空 id 回退问号。
static func initial(card_id: String) -> String:
    var display := CardDB.display_name(card_id)
    return display.substr(0, 1) if display != "" else "?"


## 卡面类型行：卡面宽度有限，只放玩法；完整类型与分类放在详情面板。
static func playstyle_text(entry: Dictionary) -> String:
    var playstyle := str(entry.get("playstyle", ""))
    var text := UI.enum_text("playstyle", playstyle) if playstyle != "" else ""
    if text != "" and text != playstyle:
        return text
    return TranslationServer.translate("ui.game.detail.undetermined")


## 详情类型行：只显示卡牌类别，玩法与分类由详情单独成行。
static func type_text(entry: Dictionary) -> String:
    var card_type := str(entry.get("card_type", ""))
    return UI.enum_text("card_type", card_type) if card_type != "" else "—"


## 卡面数值：服务端或卡表缺失时用破折号占位。
static func stat_text(value: Variant) -> String:
    if value == null:
        return "—"
    return str(int(value))


static func stat_layers(value: Variant) -> Dictionary:
    if typeof(value) == TYPE_DICTIONARY:
        return value
    return {}


static func signed(value: int) -> String:
    return "+%d" % value if value >= 0 else str(value)


## 受击行以「损失量」呈现：层和高于当前量为负数，复生超过层和为正数。
static func damage_text(value: int) -> String:
    if value > 0:
        return "-%d" % value
    if value < 0:
        return "+%d" % -value
    return "0"


## 统一时代文案：顶栏、积分榜、属性行与战斗条都显示具体时代名。
static func era_text(era: int) -> String:
    return TranslationServer.translate("ui.game.era_format") % CardDB.era_name(era)


## 可空字段转文本：缺失统一为空串。
static func optional_text(value: Variant) -> String:
    if value == null:
        return ""
    return str(value)
