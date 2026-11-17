class_name GameCardFace
extends RefCounted
## 卡面与卡牌文案的共享构造：封面、名称与「攻击 · 类型 · 生命」信息区。
##
## 只构造控件，不持有页面状态；商店、棋盘与战斗只读卡都从这里取卡面。

## 统一卡面：封面铺满整卡，底部信息区依次为居中名称与「攻击 · 类型 · 生命」。
## 所有子节点忽略鼠标，点击与悬停由外层卡片决定。
static func face(
    card_id: String,
    attack_text: String,
    health_text: String,
    type_text: String,
    badge_text := "",
) -> Control:
    var entry := CardDB.card(card_id)
    var face_node := Control.new()
    face_node.name = "CardFace"
    face_node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    face_node.size_flags_vertical = Control.SIZE_EXPAND_FILL

    var cover := ColorRect.new()
    cover.name = "Cover"
    cover.color = UITheme.tribe_cover(str(entry.get("tribe", "")))
    cover.set_anchors_preset(Control.PRESET_FULL_RECT)
    cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.add_child(cover)

    var watermark := UI.label(initial(card_id), UITheme.font_size("font-hero"), false)
    watermark.name = "CoverMark"
    watermark.set_anchors_preset(Control.PRESET_FULL_RECT)
    watermark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    watermark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    watermark.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
    watermark.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.add_child(watermark)

    if badge_text != "":
        var badge := UI.pill_label(badge_text, "accent", "font-caption")
        badge.name = "CardBadge"
        badge.set_anchors_preset(Control.PRESET_TOP_LEFT)
        badge.position = Vector2(UITheme.space("space-xs"), UITheme.space("space-xs"))
        badge.mouse_filter = Control.MOUSE_FILTER_IGNORE
        face_node.add_child(badge)

    var info := UI.card_panel("cover-info")
    info.name = "CardInfo"
    info.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
    info.offset_top = -UITheme.metric("card-info-height")
    info.offset_bottom = 0
    var info_box := VBoxContainer.new()
    info_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info_box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    info.add_child(info_box)

    var name_label := UI.label(CardDB.display_name(card_id), UITheme.font_size("font-body-sm"), false)
    name_label.name = "CardName"
    name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info_box.add_child(name_label)

    var stats_row := HBoxContainer.new()
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
    return face_node


## 战斗演示的只读随从卡：不带拖拽数据，也不可交互。
static func read_only_minion_card(minion: Dictionary) -> Control:
    var minion_index := int(minion.get("index", 0))
    var panel := BoardCard.new()
    panel.add_theme_stylebox_override("panel", UITheme.panel_style("raised-tight"))
    panel.name = "Board" + ("Minion%d" % minion_index)
    panel.set_meta("minion_id", str(minion.get("minionId", "")))
    panel.set_meta("minion_index", minion_index)
    panel.custom_minimum_size = Vector2(UITheme.metric("minion-card-width"), UITheme.metric("minion-card-height"))

    var card_id := str(minion.get("cardId", ""))
    var entry := CardDB.card(card_id)
    panel.add_child(face(
        card_id,
        stat_text(minion.get("attack")),
        stat_text(minion.get("health")),
        tribe_text(entry),
        "★" if bool(minion.get("golden", false)) else "",
    ))
    return panel


## 卡面封面的首字水印；空 id 回退问号。
static func initial(card_id: String) -> String:
    var display := CardDB.display_name(card_id)
    return display.substr(0, 1) if display != "" else "?"


## 卡面类型行：卡面宽度有限，只放部落；完整类型放在详情面板。
static func tribe_text(entry: Dictionary) -> String:
    var tribe := str(entry.get("tribe", ""))
    return tribe if tribe != "" else str(entry.get("card_type", "—"))


## 详情类型行：卡牌类别与部落。
static func type_text(entry: Dictionary) -> String:
    var card_type := str(entry.get("card_type", ""))
    var tribe := str(entry.get("tribe", ""))
    if card_type == "":
        return tribe if tribe != "" else "—"
    if tribe == "" or tribe == card_type:
        return card_type
    return "%s·%s" % [card_type, tribe]


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
    return "时代 %s" % CardDB.era_name(era)


## 可空字段转文本：缺失统一为空串。
static func optional_text(value: Variant) -> String:
    if value == null:
        return ""
    return str(value)
