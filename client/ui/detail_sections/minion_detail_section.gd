class_name MinionDetailSection
extends RefCounted
## 棋盘随从详情：名称、科学分类、玩法、关联分类与攻击/生命的构成分层。

## 来源层展示顺序与色点；基础与合计不在来源层内。
const SOURCE_LAYERS := ["aura", "permanent", "turn"]
const LAYER_DOT := {
    "aura": "blue-400",
    "permanent": "gold-400",
    "turn": "green-400",
}


## 面板宽度：来源超过 8 行且当前档位有加宽空间时取 dense，否则取 base。
static func panel_width(minion: Dictionary) -> int:
    var stats: Dictionary = minion.get("stats", {})
    var sources := _sources(stats)
    var base := UITheme.stage_metric("detail-minion-width")
    if sources.size() > 8:
        var dense := UITheme.stage_metric("detail-minion-width-dense")
        if dense > base:
            return dense
    return base


static func render(
    box: VBoxContainer,
    minion: Dictionary,
    kit: DetailKit,
    panel_width: int,
) -> void:
    var card_id := str(minion.get("cardId", ""))
    box.add_child(kit.name_label(CardDB.display_name(card_id)))
    var entry := CardDB.card(card_id)
    kit.append_classification(box, entry)
    kit.append_association(box, entry)
    var skill_text := CardDB.display_text(card_id)
    if skill_text != "":
        box.add_child(kit.text_block(skill_text))
    kit.append_gained_keywords(box, minion)
    var stats: Dictionary = minion.get("stats", {})
    if typeof(stats) != TYPE_DICTIONARY or stats.is_empty():
        box.add_child(kit.row(TranslationServer.translate("ui.game.detail.current"), "%d / %d" % [
            int(minion.get("attack", 0)),
            int(minion.get("health", 0)),
        ]))
        return
    _append_breakdown(box, stats, kit, panel_width)


## 数值构成：表头 → 基础 → 光环 → 永久 → 本回合 → 合计；来源按层挂在层小计之后。
static func _append_breakdown(
    box: VBoxContainer,
    stats: Dictionary,
    kit: DetailKit,
    panel_width: int,
) -> void:
    var attack: Dictionary = CardText.stat_layers(stats.get("attack"))
    var health: Dictionary = CardText.stat_layers(stats.get("health"))
    var grouped := _group_sources(_sources(stats))
    var two_columns := panel_width > UITheme.stage_metric("detail-minion-width")

    var header := kit.stat_header_row()
    header.name = "DetailStatHeader"
    box.add_child(header)

    var initial_a := int(attack.get("initial", 0))
    var initial_h := int(health.get("initial", 0))
    var base_row := kit.layer_row(
        TranslationServer.translate("ui.game.detail.stat.base"),
        "",
        "%d" % initial_a,
        "%d" % initial_h,
    )
    base_row.name = "DetailStatBase"
    box.add_child(base_row)

    var total_a := initial_a
    var total_h := initial_h
    for layer in SOURCE_LAYERS:
        var a := int(attack.get(layer, 0))
        var h := int(health.get(layer, 0))
        total_a += a
        total_h += h
        var zero := a == 0 and h == 0
        var dot_color := "snow-500" if zero else str(LAYER_DOT[layer])
        var layer_row := kit.layer_row(
            TranslationServer.translate("ui.game.detail.stat.%s" % layer),
            dot_color,
            CardText.signed(a),
            CardText.signed(h),
        )
        layer_row.name = "DetailStatLayer_%s" % layer
        layer_row.set_meta("layer", layer)
        box.add_child(layer_row)
        if zero:
            continue
        var layer_sources: Array = grouped.get(layer, [])
        _append_sources(box, layer_sources, kit, layer, two_columns, panel_width)

    box.add_child(kit.total_row(
        TranslationServer.translate("ui.game.detail.stat.total"),
        "%d" % total_a,
        "%d" % total_h,
    ))


## 来源行：单列直接追加；双列时每层放进两个 VBox，左列取 ceil(n/2)。
static func _append_sources(
    box: VBoxContainer,
    rows: Array,
    kit: DetailKit,
    layer: String,
    two_columns: bool,
    panel_width: int,
) -> void:
    if rows.is_empty():
        return
    var name_width := _source_name_width(panel_width, two_columns)
    if not two_columns:
        for index in rows.size():
            box.add_child(_source_row(rows[index], kit, "%s_%d" % [layer, index], name_width))
        return
    var columns := HBoxContainer.new()
    columns.name = "DetailStatSourceColumns_%s" % layer
    columns.mouse_filter = Control.MOUSE_FILTER_IGNORE
    columns.add_theme_constant_override("separation", UITheme.space("space-md"))
    columns.set_meta("layer", layer)
    var left := _source_column()
    var right := _source_column()
    columns.add_child(left)
    columns.add_child(right)
    var split := int(ceil(float(rows.size()) / 2.0))
    for index in rows.size():
        var target: VBoxContainer = left if index < split else right
        target.add_child(_source_row(
            rows[index], kit, "%s_%d" % [layer, index], name_width
        ))
    box.add_child(columns)


static func _source_column() -> VBoxContainer:
    var column := VBoxContainer.new()
    column.name = "DetailStatSourceColumn"
    column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    column.mouse_filter = Control.MOUSE_FILTER_IGNORE
    column.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    return column


static func _source_row(
    source: Dictionary,
    kit: DetailKit,
    suffix: String,
    name_width: int,
) -> HBoxContainer:
    var row := kit.source_row(
        _source_name(source),
        CardText.signed(int(source.get("attack", 0))),
        CardText.signed(int(source.get("health", 0))),
    )
    row.name = "DetailStatSourceRow_%s" % suffix
    # 来源名按可用列宽预先定宽，避免首帧按 0 宽换行撑高面板。
    var name_label := row.find_child("DetailStatSourceName", true, false) as Label
    if name_label != null:
        name_label.custom_minimum_size = Vector2(maxi(1, name_width), 0)
    return row


## 来源名称列宽：面板内容宽扣掉缩进、两个数值列与三处列间距。
static func _source_name_width(panel_width: int, two_columns: bool) -> int:
    var content := panel_width - UITheme.space("space-lg") * 2
    if two_columns:
        content = int((content - UITheme.space("space-md")) / 2)
    return maxi(
        1,
        content
            - UITheme.space("space-sm")
            - DetailKit.STAT_VALUE_WIDTH * 2
            - UITheme.space("space-md") * 3,
    )


static func _sources(stats: Dictionary) -> Array:
    var raw = stats.get("sources", [])
    return raw if raw is Array else []


static func _group_sources(sources: Array) -> Dictionary:
    var grouped := {"aura": [], "permanent": [], "turn": []}
    for raw in sources:
        if typeof(raw) != TYPE_DICTIONARY:
            continue
        var layer := str(raw.get("layer", ""))
        if grouped.has(layer):
            grouped[layer].append(raw)
    return grouped


## 来源名：卡来源为「技能 · 卡名」，缺一取另一个；非卡来源取枚举文案。
static func _source_name(source: Dictionary) -> String:
    var kind := str(source.get("kind", ""))
    if kind != "card":
        var label := UIWording.enum_text("stat_source_kind", kind)
        return label if label != "" else TranslationServer.translate("ui.game.detail.source.unknown")
    var skill_id := CardText.optional_text(source.get("skillId"))
    var card_id := CardText.optional_text(source.get("cardId"))
    var skill := CardDB.skill_name(skill_id)
    var card := _card_name(card_id)
    if skill != "" and card != "":
        return "%s · %s" % [skill, card]
    if skill != "":
        return skill
    if card != "":
        return card
    return TranslationServer.translate("ui.game.detail.source.unknown")


## 卡名：缺翻译时 `display_name` 会回退 id，此时按「无法解析」返回空串。
static func _card_name(card_id: String) -> String:
    if card_id == "":
        return ""
    var display := CardDB.display_name(card_id)
    return "" if display == card_id else display
