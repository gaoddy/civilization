class_name GameHandRow
extends Control
## 贴底手牌行：高 146，无面板无标题，整行居中于舞台宽度。
##
## 槽位按 `card-width - hand-card-overlap` 步进重叠；最后一张补齐整卡宽度，
## 十张总宽正好 832。手牌仍只支持拖拽出牌，点击不产生意图。

signal target_dropped(payload: Dictionary, target: Dictionary)

var _scroll: Control
var _center: CenterContainer
var _row: HBoxContainer


func _init() -> void:
    name = "HandStrip"
    custom_minimum_size = Vector2(0, UITheme.stage_metric("card-height"))
    mouse_filter = Control.MOUSE_FILTER_IGNORE

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
        if bool(card.get("requiresTarget", false)):
            entry.target_dropped.connect(_on_target_dropped)
        _row.add_child(slot)


func _empty_hint() -> Label:
    var label := UI.empty_hint(tr("ui.game.hand.empty"))
    label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
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
        "targets": Array(card.get("targetCandidates", [])),
    }
    node.drag_allowed = state.can_drag(playable)
    if not node.drag_allowed:
        UI.apply_dim(node)
    return node


func _on_target_dropped(payload: Dictionary, target: Dictionary) -> void:
    target_dropped.emit(payload, target)
