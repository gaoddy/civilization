class_name GameHandRow
extends Control
## 贴底手牌行：高 146，无面板无标题，整行居中于舞台宽度。
##
## 槽位按 `card-width - hand-card-overlap` 步进重叠；最后一张补齐整卡宽度，
## 十张总宽正好 832。手牌仍只支持拖拽出牌，点击不产生意图。
## 底部 `card-height` 带同时是商店卡的购买落点：拖入时描边高亮，松手交给页面。

signal target_dropped(payload: Dictionary, target: Dictionary)
signal shop_dropped(payload: Dictionary)

var _scroll: Control
var _center: CenterContainer
var _row: HBoxContainer
var _band: HandDrop
var _band_hovering := false
var _watching_shop_drag := false


func _init() -> void:
    name = "HandStrip"
    custom_minimum_size = Vector2(0, UITheme.stage_metric("card-height"))
    mouse_filter = Control.MOUSE_FILTER_IGNORE

    # 购买落点只占底部卡带，同时承担悬停描边；放在最底层，不参与布局。
    _band = HandDrop.new()
    _band.name = "HandDropBand"
    _band.add_theme_stylebox_override("panel", UITheme.panel_style("bare"))
    _band.anchor_left = 0.0
    _band.anchor_right = 1.0
    _band.anchor_top = 1.0
    _band.anchor_bottom = 1.0
    _band.offset_top = -float(UITheme.stage_metric("card-height"))
    _band.offset_bottom = 0.0
    _band.shop_dropped.connect(_on_card_shop_dropped)
    add_child(_band)
    # 只在商店卡拖拽期间逐帧更新购买带高亮。
    set_process(false)

    # 十张手牌总宽 832，小于舞台宽度；用普通 Control 承载，避免滚动容器
    # 在内容小于视口时改变子节点宽度而破坏居中。
    _scroll = Control.new()
    _scroll.name = "HandScroll"
    _scroll.set_anchors_preset(Control.PRESET_FULL_RECT)
    _scroll.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(_scroll)

    # 舞台有富余高度时先落在手牌上方的留白里，手牌仍贴底居中。
    var stack := VBoxContainer.new()
    stack.name = "HandStack"
    stack.set_anchors_preset(Control.PRESET_FULL_RECT)
    stack.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_theme_constant_override("separation", 0)
    _scroll.add_child(stack)

    var top_space := Control.new()
    top_space.name = "HandTopSpace"
    top_space.size_flags_vertical = Control.SIZE_EXPAND_FILL
    top_space.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_child(top_space)

    _center = CenterContainer.new()
    _center.name = "HandCenter"
    _center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_child(_center)

    _row = HBoxContainer.new()
    _row.name = "HandRow"
    _row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _row.add_theme_constant_override("separation", 0)
    _center.add_child(_row)


func refresh(state: GameUiState) -> void:
    UI.clear(_row)
    var hand: Array = state.self_state.get("hand", [])
    if hand.is_empty():
        _row.add_child(_empty_hint())
        return
    var card_step := UITheme.stage_metric("card-width") - UITheme.stage_metric("hand-card-overlap")
    var last_index := hand.size() - 1
    for position in hand.size():
        var card: Dictionary = hand[position]
        var index := int(card.get("index", position))
        var entry := _build_card(card, state)
        var slot := HandSlot.new()
        slot.name = "HandSlot%d" % index
        slot.setup(entry, float(card_step))
        if position == last_index:
            slot.custom_minimum_size.x = UITheme.stage_metric("card-width")
        # 需要目标的牌可以把另一张手牌当落点，信号由作为目标的卡发出：
        # 每张手牌都要转发，不能只连接效果牌自身。
        entry.target_dropped.connect(_on_target_dropped)
        entry.shop_dropped.connect(_on_card_shop_dropped)
        _row.add_child(slot)


func _empty_hint() -> Label:
    var label := UI.empty_hint(tr("ui.game.hand.empty"))
    label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    # 空手牌时提示文本不能截断商店卡的落点。
    label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return label


func _build_card(card: Dictionary, state: GameUiState) -> HandCard:
    var index := int(card.get("index", 0))
    var card_id := str(card.get("cardId", ""))
    var playable := bool(card.get("playable", false))
    var entry_data := CardDB.card(card_id)
    var card_kind := "minion" if str(entry_data.get("card_type", "")) == "minion" else "effect"
    var node := HandCard.new()
    node.name = "HandCard%d" % index
    node.custom_minimum_size = Vector2(
        UITheme.stage_metric("card-width"), UITheme.stage_metric("card-height")
    )
    node.add_theme_stylebox_override("panel", UITheme.panel_style("raised-flush"))
    node.tooltip_text = CardDB.display_text(card_id)
    if card.get("disabledReason") != null:
        node.tooltip_text = UI.error_text(str(card.get("disabledReason")))

    # 随从牌显示服务端给出的真实身材（含成长等全部层数），效果牌不显示身材。
    var golden := bool(card.get("golden", false))
    var reward_level: Variant = card.get("rewardLevel")
    node.add_child(HandCardFace.build(card, self))
    if reward_level != null:
        node.tooltip_text = tr("ui.game.reward_card.text") % int(reward_level)

    node.payload = {
        "epoch": state.epoch,
        "handIndex": index,
        "instanceId": str(card.get("instanceId", "")),
        "cardId": card_id,
        "cardKind": card_kind,
        "golden": golden,
        "requiresTarget": bool(card.get("requiresTarget", false)),
        # 服务端下发的合法目标；效果牌拖到候选上松手才提交。
        # 必须深拷贝，避免拖拽数据与视图数组互相污染。
        "targets": Array(card.get("targetCandidates", [])).duplicate(true),
    }
    # 瞄准层的候选命中依赖这组元数据：手牌按实例编号定位。
    node.set_meta("target_zone", "hand")
    node.set_meta("target_id", str(card.get("instanceId", "")))
    node.drag_allowed = state.can_drag(playable)
    if not node.drag_allowed:
        UI.apply_dim(node)
    return node


func _on_target_dropped(payload: Dictionary, target: Dictionary) -> void:
    target_dropped.emit(payload, target)


func _on_card_shop_dropped(payload: Dictionary) -> void:
    shop_dropped.emit(payload)


func _notification(what: int) -> void:
    if what == NOTIFICATION_DRAG_BEGIN:
        var payload = get_viewport().gui_get_drag_data()
        if HandDrop.accepts_shop(payload):
            _watching_shop_drag = true
            set_process(true)
    elif what == NOTIFICATION_DRAG_END:
        _stop_shop_watch()


## 拖拽期间逐帧复查指针是否在卡带内：卡片自身会接住落点，靠 `_can_drop_data`
## 无法覆盖悬停在卡片上的情况。
func _process(_delta: float) -> void:
    var viewport := get_viewport()
    if (
        viewport == null
        or not viewport.gui_is_dragging()
        or not HandDrop.accepts_shop(viewport.gui_get_drag_data())
    ):
        _stop_shop_watch()
        return
    var rect := get_global_rect()
    var band := Rect2(
        rect.position.x,
        rect.end.y - float(UITheme.stage_metric("card-height")),
        rect.size.x,
        float(UITheme.stage_metric("card-height"))
    )
    _set_band_hover(band.has_point(get_global_mouse_position()))


func _stop_shop_watch() -> void:
    if not _watching_shop_drag:
        return
    _watching_shop_drag = false
    set_process(false)
    _set_band_hover(false)


func _set_band_hover(hovering: bool) -> void:
    if hovering == _band_hovering:
        return
    _band_hovering = hovering
    _band.add_theme_stylebox_override(
        "panel", UITheme.panel_style("accent-bare" if hovering else "bare")
    )
