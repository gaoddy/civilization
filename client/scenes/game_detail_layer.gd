class_name GameDetailLayer
extends Control
## 随从详情浮层：覆盖全屏但不拦截输入，层级位于内容与战报之上、选择浮层之下。
##
## 卡片通过 `track()` 登记，本层用 `_input` 做跨子控件的矩形命中判断；
## 模态浮层打开时由页面把 `blocked` 置真，本层直接隐藏且不再响应悬停。

const DETAIL_ROWS := ["initial", "permanent", "turn", "aura", "damage"]
## 科学分类完整路径与多值字段的连接符。
const TAXON_PATH_SEPARATOR := " › "
const LIST_SEPARATOR := " / "
## 详情面板的挂载方向：商店卡在左右，棋盘卡在上下。
const DETAIL_SIDES := {
    "shop": ["right", "left"],
    "seat": ["right", "left"],
    "minion": ["bottom", "top"],
    "seller": ["bottom", "top"],
    "hero": ["left", "right"],
}

## 模态浮层打开时置真：详情层会盖住浮层内容，直接隐藏且不再响应悬停。
var blocked := false
## 左栏战报浮层打开时置真：战报优先，悬停详情不再抢占画面。
var report_open := false

var _panel: PanelContainer
var _cards: Array = []
var _hover_card: Control = null


func _init() -> void:
    name = "MinionDetailLayer"
    set_anchors_preset(Control.PRESET_FULL_RECT)
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    visible = false

    _panel = UI.card_panel("raised")
    _panel.name = "MinionDetailPanel"
    _panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _panel.custom_minimum_size = Vector2(UITheme.metric("detail-panel-width"), 0)
    _panel.visible = false
    add_child(_panel)


## 卡片统一挂详情：悬停进入/离开、鼠标左键按下/松开、触摸按下/松开。
## `data.detail_kind` 决定详情内容（shop / minion）与面板挂载方向。
## 卡内控件全部 IGNORE 鼠标，祖先悬停通知会被子控件截断，
## 因此真正的矩形命中判断放在 `_input`，这里只保留就近的进入/离开兜底。
func track(card: Control, data: Dictionary) -> void:
    # 重建时旧行已 queue_free；登记新行前裁掉已释放的条目，避免无限累积。
    _cards = _cards.filter(func(entry): return is_instance_valid(entry))
    card.set_meta("detail_data", data)
    _cards.append(card)
    # 常驻节点每轮刷新都会重新登记：信号只连一次，避免重复连接告警。
    if card.has_meta("detail_bound"):
        return
    card.set_meta("detail_bound", true)
    card.mouse_entered.connect(_show_detail.bind(data, card))
    card.gui_input.connect(_on_card_gui_input.bind(data, card))


## 每次刷新前清空登记并隐藏当前面板。
func reset() -> void:
    hide_panel()
    _cards.clear()


## 隐藏详情层与面板，并清空当前悬停卡。
func hide_panel() -> void:
    if _panel != null:
        _panel.visible = false
    visible = false
    _hover_card = null


func _on_card_gui_input(event: InputEvent, data: Dictionary, card: Control) -> void:
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        if event.pressed:
            _show_detail(data, card)
        else:
            _update_detail_hover(card.get_global_rect().position + event.position)
    elif event is InputEventScreenTouch:
        if event.pressed:
            _show_detail(data, card)
        else:
            _update_detail_hover(card.get_global_rect().position + event.position)


## 跨子控件的真实命中判断：指针落在卡片矩形内即显示，移出才隐藏。
func _input(event: InputEvent) -> void:
    if event is InputEventMouseMotion:
        _update_detail_hover(_canvas_point(event.position))
    elif event is InputEventScreenTouch:
        _update_detail_hover(_canvas_point(event.position))
    elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        _update_detail_hover(_canvas_point(event.position))


## 视口坐标转为画布坐标，与 get_global_rect() 同一坐标系。
func _canvas_point(viewport_point: Vector2) -> Vector2:
    return get_viewport().get_canvas_transform().affine_inverse() * viewport_point


func _update_detail_hover(point: Vector2) -> void:
    # 模态浮层打开时详情层会盖住浮层内容，直接隐藏且不再响应悬停。
    if blocked or report_open:
        hide_panel()
        return
    var found: Control = null
    for card in _cards:
        if is_instance_valid(card) and card.is_visible_in_tree() and card.get_global_rect().has_point(point):
            found = card
            break
    if found == null:
        if _hover_card != null:
            hide_panel()
        return
    if found == _hover_card and _panel != null and _panel.visible:
        return
    _show_detail(found.get_meta("detail_data", {}), found)


func _show_detail(data: Dictionary, anchor: Control) -> void:
    if blocked or report_open:
        return
    if _panel == null or anchor == null or not is_instance_valid(anchor):
        return
    var kind := str(data.get("detail_kind", "minion"))
    # 同一张卡已在展示：只校正位置，避免每次按下/移入都重建整块面板。
    # `reset()` 会先隐藏面板，因此数据更新仍会触发重建。
    if anchor == _hover_card and _panel.visible:
        _place_detail(anchor.get_global_rect(), kind)
        return
    if not is_instance_valid(_hover_card) or _hover_card != anchor:
        _hover_card = anchor
    UI.clear(_panel)
    var box := VBoxContainer.new()
    box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _panel.add_child(box)

    if kind == "shop":
        _render_shop_detail(box, data)
    elif kind == "seat":
        _render_seat_detail(box, data.get("player", {}), int(data.get("hpCap", 0)))
    elif kind == "seller":
        _render_seller_detail(box)
    elif kind == "hero":
        _render_hero_detail(box, data.get("heroId"))
    else:
        _render_minion_detail(box, data.get("minion", {}))

    visible = true
    _panel.visible = true
    _place_detail(anchor.get_global_rect(), kind)


## 商店卡详情：类型、科学分类、玩法、关联、食物链、费用、原始数值、时代、
## 技能文本与不可购买原因。
func _render_shop_detail(box: VBoxContainer, data: Dictionary) -> void:
    var card_id := str(data.get("cardId", ""))
    var entry := CardDB.card(card_id)
    box.add_child(_detail_name(CardDB.display_name(card_id)))
    box.add_child(_detail_row(tr("ui.game.detail.type"), GameCardFace.type_text(entry)))
    _append_classification(box, entry)
    _append_association(box, entry)
    _append_food_chain(box, entry)
    box.add_child(_detail_row(tr("ui.game.detail.cost"), tr("ui.game.detail.cost_value") % CardDB.display_cost(card_id)))
    box.add_child(_detail_row(tr("ui.game.detail.initial_stats"), "%s / %s" % [
        GameCardFace.stat_text(entry.get("attack")),
        GameCardFace.stat_text(entry.get("health")),
    ]))
    var era := str(entry.get("era", ""))
    if era != "":
        box.add_child(_detail_row(tr("ui.game.detail.era"), UI.enum_text("era", era)))
    var skill_text := CardDB.display_text(card_id)
    if skill_text != "":
        box.add_child(_detail_text_block(skill_text))
    if data.get("disabledReason") != null:
        var reason := UI.label(UI.error_text(str(data.get("disabledReason"))), UITheme.font_size("font-body-sm"))
        reason.name = "DetailBlocked"
        reason.add_theme_color_override("font_color", UITheme.color("color-danger"))
        box.add_child(reason)


## 座位详情：名字、英雄、生命（当前/上限）、时代、随从数与状态。
func _render_seat_detail(box: VBoxContainer, player: Dictionary, hp_cap: int) -> void:
    box.add_child(_detail_name(GameCardFace.optional_text(player.get("displayName"))))
    box.add_child(_detail_row(tr("ui.game.detail.hero"), CardDB.display_name(GameCardFace.optional_text(player.get("heroCardId")))))
    box.add_child(_detail_row(tr("ui.game.detail.hp"), "%d/%d" % [int(player.get("hp", 0)), maxi(hp_cap, 0)]))
    box.add_child(_detail_row(tr("ui.game.detail.era"), CardDB.era_name(int(player.get("era", 1)))))
    box.add_child(_detail_row(tr("ui.game.detail.minions"), "%d" % int(player.get("boardCount", 0))))
    var alive := bool(player.get("alive", true))
    box.add_child(_detail_row(
        tr("ui.game.detail.status"),
        tr("ui.game.detail.alive") if alive else tr("ui.game.detail.eliminated"),
    ))
    if alive:
        box.add_child(_detail_row(
            tr("ui.game.detail.recruit"),
            tr("ui.game.detail.recruited") if bool(player.get("recruited", false)) else tr("ui.game.detail.not_recruited"),
        ))


## 售货员详情：营业状态与可执行的商店操作，替代常驻文字。
func _render_seller_detail(box: VBoxContainer) -> void:
    box.add_child(_detail_name(tr("ui.game.detail.seller")))
    box.add_child(_detail_row(tr("ui.game.detail.status"), tr("ui.game.detail.shop_open")))
    var hint := UI.label(
        tr("ui.game.detail.shop_hint"),
        UITheme.font_size("font-body-sm"),
    )
    hint.custom_minimum_size = Vector2(_detail_text_width(), 0)
    hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    hint.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    box.add_child(hint)


## 我方英雄详情：科学分类、玩法、擅长玩法、时代与英雄技能，替代常驻技能文本。
func _render_hero_detail(box: VBoxContainer, hero_id: Variant) -> void:
    var id := "" if hero_id == null else str(hero_id)
    box.add_child(_detail_name(CardDB.display_name(id)))
    var entry := CardDB.card(id)
    _append_classification(box, entry)
    _append_specialty(box, entry)
    var era := str(entry.get("era", ""))
    if era != "":
        box.add_child(_detail_row(tr("ui.game.detail.era"), UI.enum_text("era", era)))
    var skill_text := CardDB.display_text(id)
    if skill_text == "":
        return
    var heading := UI.label(tr("ui.game.detail.hero_power"), UITheme.font_size("font-caption"), false)
    heading.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    box.add_child(heading)
    box.add_child(_detail_text_block(skill_text))


## 棋盘随从详情：名称、科学分类、玩法、关联、食物链与攻击/生命的构成分层。
func _render_minion_detail(box: VBoxContainer, minion: Dictionary) -> void:
    var card_id := str(minion.get("cardId", ""))
    box.add_child(_detail_name(CardDB.display_name(card_id)))
    var entry := CardDB.card(card_id)
    _append_classification(box, entry)
    _append_association(box, entry)
    _append_food_chain(box, entry)
    var skill_text := CardDB.display_text(card_id)
    if skill_text != "":
        box.add_child(_detail_text_block(skill_text))
    var stats: Dictionary = minion.get("stats", {})
    if typeof(stats) != TYPE_DICTIONARY or stats.is_empty():
        box.add_child(_detail_row(tr("ui.game.detail.current"), "%d / %d" % [
            int(minion.get("attack", 0)),
            int(minion.get("health", 0)),
        ]))
        return
    var attack: Dictionary = GameCardFace.stat_layers(stats.get("attack"))
    var health: Dictionary = GameCardFace.stat_layers(stats.get("health"))
    for layer in DETAIL_ROWS:
        var a := int(attack.get(layer, 0))
        var h := int(health.get(layer, 0))
        if layer == "damage" and a == 0 and h == 0:
            continue
        var label := UI.enum_text("stat_layer", layer)
        if layer == "initial":
            box.add_child(_detail_row(label, "%d / %d" % [a, h]))
        elif layer == "damage":
            box.add_child(_detail_row(label, "%s / %s" % [
                GameCardFace.damage_text(a),
                GameCardFace.damage_text(h),
            ]))
        else:
            box.add_child(_detail_row(label, "%s / %s" % [
                GameCardFace.signed(a),
                GameCardFace.signed(h),
            ]))
    box.add_child(_detail_row(tr("ui.game.detail.current"), "%d / %d" % [
        int(attack.get("current", 0)),
        int(health.get("current", 0)),
    ]))


## 科学分类与玩法：分类路径缺失显示「未定」，卡表无该实体时整组跳过。
func _append_classification(box: VBoxContainer, entry: Dictionary) -> void:
    if entry.is_empty():
        return
    var path: Array = CardDB.taxon_path(str(entry.get("taxon", "")))
    var path_text := TAXON_PATH_SEPARATOR.join(PackedStringArray(path)) if not path.is_empty() else tr("ui.game.detail.undetermined")
    box.add_child(_detail_row(tr("ui.game.detail.taxonomy"), path_text))
    var playstyle := CardDB.playstyle_name(str(entry.get("playstyle", "")))
    if playstyle == "":
        playstyle = tr("ui.game.detail.undetermined")
    box.add_child(_detail_row(tr("ui.game.detail.playstyle"), playstyle))


## 关联玩法：仅中立非学者卡展示 `play:`，非中立或学者整行省略。
func _append_association(box: VBoxContainer, entry: Dictionary) -> void:
    if entry.is_empty():
        return
    if str(entry.get("playstyle", "")) != "neutral" or bool(entry.get("is_scholar", false)):
        return
    var names := _specialty_names(entry, "play")
    if names.is_empty():
        return
    box.add_child(_detail_row(
        tr("ui.game.detail.association"),
        LIST_SEPARATOR.join(PackedStringArray(names)),
    ))


## 英雄擅长玩法：取 `play:` 关联，空列表不显示该行。
func _append_specialty(box: VBoxContainer, entry: Dictionary) -> void:
    var names := _specialty_names(entry, "play")
    if names.is_empty():
        return
    box.add_child(_detail_row(
        tr("ui.game.detail.specialty"),
        LIST_SEPARATOR.join(PackedStringArray(names)),
    ))


## 食物链：捕食与寄生分别聚合，二者都为空时不显示该行。
func _append_food_chain(box: VBoxContainer, entry: Dictionary) -> void:
    if entry.is_empty():
        return
    var prey := _specialty_names(entry, "prey", true)
    var host := _specialty_names(entry, "host", true)
    if prey.is_empty() and host.is_empty():
        return
    var parts := PackedStringArray()
    if not prey.is_empty():
        parts.append("%s %s" % [
            tr("ui.game.detail.prey"),
            LIST_SEPARATOR.join(PackedStringArray(prey)),
        ])
    if not host.is_empty():
        parts.append("%s %s" % [
            tr("ui.game.detail.host"),
            LIST_SEPARATOR.join(PackedStringArray(host)),
        ])
    box.add_child(_detail_row(tr("ui.game.detail.food_chain"), " · ".join(parts)))


## 按关联类型取展示名；`taxon` 为真时走科学分类文本，否则走玩法文本。
func _specialty_names(entry: Dictionary, kind: String, taxon := false) -> Array:
    var names: Array = []
    var specialties = entry.get("specialty", [])
    if not (specialties is Array):
        return names
    for item in specialties:
        if not (item is Dictionary):
            continue
        if str(item.get("kind", "")) != kind:
            continue
        var value := str(item.get("value", ""))
        var name := CardDB.taxon_name(value) if taxon else CardDB.playstyle_name(value)
        if name != "":
            names.append(name)
    return names


func _detail_name(text: String) -> Label:
    var name_label := UI.label(text, UITheme.font_size("font-title"), false)
    name_label.name = "DetailName"
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return name_label


## 面板宽度扣除卡片内边距后的文本宽度，供技能文本换行。
func _detail_text_width() -> int:
    return UITheme.metric("detail-panel-width") - UITheme.space("space-lg") * 2


## 详情里的介绍/技能文本块：按面板宽度换行，颜色弱于正文。
func _detail_text_block(text: String) -> Label:
    var label := UI.label(text, UITheme.font_size("font-body-sm"))
    label.name = "DetailText"
    label.custom_minimum_size = Vector2(_detail_text_width(), 0)
    label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    return label


## 详情定位：商店卡先右后左，棋盘卡先下后上；夹紧视口且不覆盖当前卡片。
func _place_detail(card_rect: Rect2, kind: String) -> void:
    var viewport := get_viewport_rect().size
    var panel_size := _panel.get_combined_minimum_size()
    var margin := UITheme.space("space-xs")
    var gap := UITheme.space("space-sm")
    var fallback := Vector2.ZERO
    var has_fallback := false
    for side in DETAIL_SIDES.get(kind, DETAIL_SIDES["minion"]):
        var pos := Vector2.ZERO
        match side:
            "right":
                pos = Vector2(card_rect.end.x + gap, card_rect.position.y)
            "left":
                pos = Vector2(card_rect.position.x - panel_size.x - gap, card_rect.position.y)
            "bottom":
                pos = Vector2(card_rect.position.x, card_rect.end.y + gap)
            _:
                pos = Vector2(card_rect.position.x, card_rect.position.y - panel_size.y - gap)
        pos.x = clampf(pos.x, margin, maxf(margin, viewport.x - panel_size.x - margin))
        pos.y = clampf(pos.y, margin, maxf(margin, viewport.y - panel_size.y - margin))
        if not has_fallback:
            fallback = pos
            has_fallback = true
        if not Rect2(pos, panel_size).intersects(card_rect):
            _panel.position = pos
            return
    _panel.position = fallback


func _detail_row(prefix: String, value: String) -> HBoxContainer:
    var row := HBoxContainer.new()
    row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    var key := UI.label(prefix, UITheme.font_size("font-body-sm"), false)
    key.mouse_filter = Control.MOUSE_FILTER_IGNORE
    key.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    row.add_child(key)
    var spacer := Control.new()
    spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_child(spacer)
    var text := UI.label(value, UITheme.font_size("font-body-sm"), false)
    text.mouse_filter = Control.MOUSE_FILTER_IGNORE
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
    row.add_child(text)
    return row
