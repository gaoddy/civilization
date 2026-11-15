extends Control
## 对局：只读服务端 PlayerView 与 affordances，操作只发协议意图。

const PHASE_TEXT := {
    Protocol.PHASE_RECRUIT: "招募",
    Protocol.PHASE_COMBAT: "战斗",
    Protocol.PHASE_SETTLE: "结算",
    Protocol.PHASE_GAME_OVER: "结束",
}

var _turn_label: Label
var _timer_label: Label
var _stats_label: Label
var _status: Label
var _shop_row: HBoxContainer
var _hand_row: HBoxContainer
var _board_box: VBoxContainer
var _players_box: VBoxContainer
var _log_text: RichTextLabel
var _hero_power_button: Button
var _reroll_button: Button
var _upgrade_button: Button
var _freeze_button: Button
var _end_button: Button
var _overlay: Control

var _intent_counter := 0
var _target_requests: Dictionary = {}
var _target_intent: Dictionary = {}
var _target_title := "选择目标"
var _target_required := false
var _elapsed := 0.0
var _deadline_expired := false
var _view_stale := false


func _ready() -> void:
    _build()
    GameStore.view_changed.connect(_refresh)
    GameStore.error_received.connect(_on_error)
    GameStore.game_over.connect(_on_game_over)
    Net.connected.connect(_refresh_connection)
    Net.disconnected.connect(_refresh_connection)
    _refresh()
    _update_timer()


func _build() -> void:
    var background := ColorRect.new()
    background.color = Color("10141c")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(background)

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, 16)
    add_child(margin)

    var root := VBoxContainer.new()
    root.add_theme_constant_override("separation", 8)
    margin.add_child(root)

    var top := HBoxContainer.new()
    top.add_theme_constant_override("separation", 18)
    _turn_label = UI.label("", 20)
    top.add_child(_turn_label)
    _timer_label = UI.label("—", 20)
    top.add_child(_timer_label)
    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    top.add_child(spacer)
    _stats_label = UI.label("", 20)
    top.add_child(_stats_label)
    root.add_child(top)

    var actions := HBoxContainer.new()
    actions.add_theme_constant_override("separation", 8)
    _reroll_button = UI.button("刷新")
    _reroll_button.name = "RerollButton"
    _reroll_button.pressed.connect(_on_reroll)
    actions.add_child(_reroll_button)
    _upgrade_button = UI.button("升时代")
    _upgrade_button.pressed.connect(_on_upgrade)
    actions.add_child(_upgrade_button)
    _freeze_button = UI.button("冻结商店")
    _freeze_button.name = "FreezeButton"
    _freeze_button.pressed.connect(_on_freeze)
    actions.add_child(_freeze_button)
    _hero_power_button = UI.button("英雄技能")
    _hero_power_button.name = "HeroPowerButton"
    _hero_power_button.pressed.connect(_on_hero_power)
    actions.add_child(_hero_power_button)
    var action_spacer := Control.new()
    action_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    actions.add_child(action_spacer)
    _end_button = UI.button("结束招募")
    _end_button.name = "EndButton"
    _end_button.pressed.connect(_on_end_recruit)
    actions.add_child(_end_button)
    root.add_child(actions)
    root.add_child(HSeparator.new())

    var body := HBoxContainer.new()
    body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override("separation", 16)
    root.add_child(body)

    var left := VBoxContainer.new()
    left.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    left.add_theme_constant_override("separation", 6)
    left.add_child(UI.label("商店", 18))
    _shop_row = HBoxContainer.new()
    _shop_row.add_theme_constant_override("separation", 6)
    left.add_child(_shop_row)
    left.add_child(UI.label("手牌", 18))
    _hand_row = HBoxContainer.new()
    _hand_row.add_theme_constant_override("separation", 6)
    left.add_child(_hand_row)
    left.add_child(UI.label("棋盘", 18))
    _board_box = VBoxContainer.new()
    _board_box.add_theme_constant_override("separation", 4)
    left.add_child(_board_box)
    body.add_child(left)

    var right := VBoxContainer.new()
    right.custom_minimum_size = Vector2(340, 0)
    right.add_theme_constant_override("separation", 6)
    right.add_child(UI.label("玩家", 18))
    _players_box = VBoxContainer.new()
    _players_box.add_theme_constant_override("separation", 3)
    right.add_child(_players_box)
    right.add_child(UI.label("战报与事件", 18))
    _log_text = RichTextLabel.new()
    _log_text.fit_content = false
    _log_text.scroll_active = true
    _log_text.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _log_text.custom_minimum_size = Vector2(0, 220)
    right.add_child(_log_text)
    body.add_child(right)

    _status = UI.label("", 14)
    root.add_child(_status)

    _overlay = Control.new()
    _overlay.name = "ChooserOverlay"
    _overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
    _overlay.visible = false
    add_child(_overlay)


func _process(delta: float) -> void:
    _elapsed += delta
    if _elapsed >= 0.25:
        _elapsed = 0.0
        _update_timer()


func _refresh() -> void:
    var view: Dictionary = GameStore.view
    if view.is_empty():
        _status.text = "等待对局状态…"
        return

    var self_state: Dictionary = view.get("self", {})
    var recruited := bool(self_state.get("recruited", false))
    _view_stale = int(view.get("turn", 0)) != GameStore.turn
    _turn_label.text = "第 %d 回合 · %s" % [
        int(view.get("turn", 1)),
        PHASE_TEXT.get(str(view.get("phase", "")), str(view.get("phase", ""))),
    ]
    _stats_label.text = "生命 %d   时代 %d   金币 %d" % [
        int(self_state.get("hp", 0)),
        int(self_state.get("era", 1)),
        int(self_state.get("gold", 0)),
    ]

    _refresh_actions(self_state, recruited)
    _refresh_shop(self_state)
    _refresh_hand(self_state)
    _refresh_board(self_state, recruited)
    _refresh_players(view)
    _refresh_log()
    _refresh_overlay(self_state, recruited)
    _refresh_connection()


func _refresh_actions(self_state: Dictionary, recruited: bool) -> void:
    var aff: Dictionary = self_state.get("affordances", {})
    var phase := str(GameStore.view.get("phase", ""))
    _reroll_button.text = "刷新（%d 金）" % int(self_state.get("rerollCost", 0))
    _reroll_button.disabled = _view_stale or _deadline_expired or not bool(aff.get("canReroll", false))
    var upgrade_cost = self_state.get("upgradeCost")
    if upgrade_cost != null:
        _upgrade_button.text = "升时代（%d 金）" % int(upgrade_cost)
    else:
        _upgrade_button.text = "时代已满"
    _upgrade_button.disabled = _view_stale or _deadline_expired or not bool(aff.get("canUpgradeEra", false))
    _upgrade_button.tooltip_text = "" if aff.get("upgradeDisabledReason") == null else str(aff.get("upgradeDisabledReason"))
    var power: Dictionary = self_state.get("heroPower", {})
    _hero_power_button.text = "英雄技能（剩 %d）" % int(power.get("usesLeft", 0))
    _hero_power_button.disabled = _view_stale or _deadline_expired or not bool(aff.get("canUseHeroPower", false))
    _freeze_button.disabled = _view_stale or _deadline_expired or phase != Protocol.PHASE_RECRUIT or recruited
    _end_button.text = "已结束招募" if recruited else "结束招募"
    _end_button.disabled = _view_stale or _deadline_expired or not bool(aff.get("canEndRecruit", false))


func _refresh_shop(self_state: Dictionary) -> void:
    UI.clear(_shop_row)
    var aff: Dictionary = self_state.get("affordances", {})
    for slot in self_state.get("shop", []):
        var index := int(slot.get("index", 0))
        var card_id := _optional_text(slot.get("cardId"))
        var button := UI.button("")
        button.name = "ShopSlot%d" % index
        button.custom_minimum_size = Vector2(132, 92)
        if card_id == "":
            button.text = "空槽"
            button.disabled = true
        else:
            var frozen := "[冻] " if bool(slot.get("frozen", false)) else ""
            button.text = "%s%s\n%s 金" % [
                frozen,
                CardDB.display_name(card_id),
                CardDB.display_cost(card_id),
            ]
            var slot_aff := _find_by_index(aff.get("shopSlots", []), index)
            button.disabled = _view_stale or _deadline_expired or not bool(slot_aff.get("buyable", false))
            if slot_aff.get("disabledReason") != null:
                button.tooltip_text = str(slot_aff.get("disabledReason"))
            button.pressed.connect(_on_buy.bind(index))
        _shop_row.add_child(button)


func _refresh_hand(self_state: Dictionary) -> void:
    UI.clear(_hand_row)
    for card in self_state.get("hand", []):
        var index := int(card.get("index", 0))
        var card_id := str(card.get("cardId", ""))
        var button := UI.button("%s\n%s" % [CardDB.display_name(card_id), CardDB.display_text(card_id)])
        button.name = "HandCard%d" % index
        button.custom_minimum_size = Vector2(150, 80)
        button.disabled = _view_stale or _deadline_expired or not bool(card.get("playable", false))
        if card.get("disabledReason") != null:
            button.tooltip_text = str(card.get("disabledReason"))
        button.pressed.connect(_on_play.bind(index))
        _hand_row.add_child(button)


func _refresh_board(self_state: Dictionary, recruited: bool) -> void:
    UI.clear(_board_box)
    var phase := str(GameStore.view.get("phase", ""))
    var aff: Dictionary = self_state.get("affordances", {})
    for minion in self_state.get("board", []):
        var index := int(minion.get("index", 0))
        var row := HBoxContainer.new()
        row.name = "BoardMinion%d" % index
        row.add_theme_constant_override("separation", 6)
        var keywords := _join_cn(minion.get("keywords", []))
        var label := UI.label("%s  %d/%d  %s" % [
            CardDB.display_name(str(minion.get("cardId", ""))),
            int(minion.get("attack", 0)),
            int(minion.get("health", 0)),
            keywords,
        ], 15)
        label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        row.add_child(label)

        var minion_aff := _find_by_index(aff.get("boardMinions", []), index)
        var sell := UI.button("出售")
        sell.disabled = _view_stale or _deadline_expired or not bool(minion_aff.get("sellable", false))
        sell.pressed.connect(_on_sell.bind(str(minion.get("minionId", ""))))
        row.add_child(sell)

        var can_rearrange := not _view_stale and not _deadline_expired and phase == Protocol.PHASE_RECRUIT and not recruited
        var left := UI.button("←")
        left.disabled = not can_rearrange or index <= 0
        left.pressed.connect(_on_rearrange.bind(index, index - 1))
        row.add_child(left)

        var right := UI.button("→")
        right.disabled = not can_rearrange or index >= self_state.get("board", []).size() - 1
        right.pressed.connect(_on_rearrange.bind(index, index + 1))
        row.add_child(right)
        _board_box.add_child(row)


func _refresh_players(view: Dictionary) -> void:
    UI.clear(_players_box)
    for player in view.get("players", []):
        var marks := []
        if bool(player.get("alive", true)) == false:
            marks.append("淘汰")
        if bool(player.get("recruited", false)):
            marks.append("已招募")
        if player.get("placement") != null:
            marks.append("第 %d 名" % int(player.get("placement")))
        if str(player.get("playerId", "")) == GameStore.player_id:
            marks.append("我")
        var line := "%d号 %s%s  HP %d  时代 %d  随从 %d" % [
            int(player.get("seat", 0)),
            _optional_text(player.get("displayName")),
            "（%s）" % _join_cn(marks) if not marks.is_empty() else "",
            int(player.get("hp", 0)),
            int(player.get("era", 1)),
            int(player.get("boardCount", 0)),
        ]
        var label := UI.label(line, 14)
        if str(player.get("playerId", "")) == GameStore.player_id:
            label.modulate = Color("8fd3ff")
        _players_box.add_child(label)


func _refresh_log() -> void:
    var lines: Array = []
    if not GameStore.last_combat.is_empty():
        lines.append(_combat_text())
    var events: Array = GameStore.event_log
    var start := maxi(0, events.size() - 8)
    for index in range(start, events.size()):
        lines.append(_event_text(events[index]))
    _log_text.text = _join_lines(lines)


func _combat_text() -> String:
    var combat: Dictionary = GameStore.last_combat
    var winner = combat.get("winnerId")
    var result := "平局"
    if winner != null:
        result = "胜利" if str(winner) == GameStore.player_id else "失败"
    var mirror: String = "（镜像）" if bool(combat.get("mirror", false)) else ""
    return "[战斗] %s%s  伤害 %d  我方 %d vs 对手 %d\n" % [
        result,
        mirror,
        int(combat.get("damage", 0)),
        combat.get("ownBoard", []).size(),
        combat.get("opponentBoard", []).size(),
    ]


func _event_text(event: Dictionary) -> String:
    var kind := str(event.get("kind", ""))
    var parts := ["[%s]" % kind]
    if event.get("cardId") != null:
        parts.append(CardDB.display_name(str(event.get("cardId"))))
    if event.get("amount") != null:
        parts.append(str(int(event.get("amount"))))
    if event.get("place") != null:
        parts.append("第 %d 名" % int(event.get("place")))
    if event.get("playerId") != null:
        parts.append(GameStore.player_name(str(event.get("playerId"))))
    return " ".join(PackedStringArray(parts))


func _join_cn(values: Array) -> String:
    var parts := PackedStringArray()
    for value in values:
        parts.append(str(value))
    return "、".join(parts)


func _join_lines(values: Array) -> String:
    var parts := PackedStringArray()
    for value in values:
        parts.append(str(value))
    return "\n".join(parts)


func _refresh_overlay(self_state: Dictionary, recruited: bool) -> void:
    UI.clear(_overlay)
    var choices: Array = self_state.get("pendingChoices", [])
    if _view_stale or _deadline_expired or recruited:
        _target_intent = {}
        _overlay.visible = false
        return
    if not _target_intent.is_empty():
        _build_target_overlay(self_state)
    elif not choices.is_empty():
        _build_choice_overlay(choices[0])
    else:
        _overlay.visible = false


func _overlay_frame(title: String, parent: VBoxContainer) -> void:
    _overlay.visible = true
    var shade := ColorRect.new()
    shade.color = Color(0, 0, 0, 0.65)
    shade.set_anchors_preset(Control.PRESET_FULL_RECT)
    _overlay.add_child(shade)
    var center := CenterContainer.new()
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    _overlay.add_child(center)
    var panel := UI.panel()
    panel.custom_minimum_size = Vector2(420, 0)
    center.add_child(panel)
    var margin := MarginContainer.new()
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, 16)
    panel.add_child(margin)
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", 8)
    margin.add_child(box)
    box.add_child(UI.label(title, 20))
    for child in parent.get_children():
        parent.remove_child(child)
        box.add_child(child)


func _build_choice_overlay(choice: Dictionary) -> void:
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", 8)
    var kind := str(choice.get("kind", ""))
    if kind == "discover":
        box.add_child(UI.label("发现：选择一张卡牌", 16))
        var index := 0
        for card_id in choice.get("candidates", []):
            var button := UI.button("%s\n%s" % [
                CardDB.display_name(str(card_id)),
                CardDB.display_text(str(card_id)),
            ])
            button.pressed.connect(_on_choose.bind(index))
            box.add_child(button)
            index += 1
    else:
        box.add_child(UI.label("进化：选择一项", 16))
        var index := 0
        for option in choice.get("options", []):
            var button := UI.button(str(option))
            button.pressed.connect(_on_choose.bind(index))
            box.add_child(button)
            index += 1
    _overlay_frame("待选择", box)


func _build_target_overlay(self_state: Dictionary) -> void:
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", 8)
    box.add_child(UI.label(_target_title, 16))
    if not _target_required:
        var skip := UI.button("不指定目标")
        skip.pressed.connect(_on_target_skip)
        box.add_child(skip)
    for minion in self_state.get("board", []):
        var button := UI.button("%s  %d/%d" % [
            CardDB.display_name(str(minion.get("cardId", ""))),
            int(minion.get("attack", 0)),
            int(minion.get("health", 0)),
        ])
        button.pressed.connect(_on_target_chosen.bind(str(minion.get("minionId", ""))))
        box.add_child(button)
    var cancel := UI.button("取消")
    cancel.pressed.connect(_on_target_cancel)
    box.add_child(cancel)
    _overlay_frame("选择目标", box)


func _refresh_connection() -> void:
    if not Net.is_open() and not GameStore.view.is_empty():
        _status.text = "连接已断开，正在自动重连…"
    elif Net.is_open() and _status.text.begins_with("连接已断开"):
        _status.text = ""


func _update_timer() -> void:
    if _timer_label == null:
        return
    if GameStore.deadline == "":
        _timer_label.text = "—"
        _set_deadline_expired(false)
        return
    var deadline := _unix_from_rfc3339(GameStore.deadline)
    if deadline <= 0.0:
        _timer_label.text = "—"
        _set_deadline_expired(false)
        return
    var left := int(ceil(deadline - Time.get_unix_time_from_system()))
    _timer_label.text = "剩余 %d 秒" % maxi(left, 0)
    _set_deadline_expired(left <= 0)


func _set_deadline_expired(value: bool) -> void:
    if _deadline_expired == value:
        return
    _deadline_expired = value
    _refresh()


func _unix_from_rfc3339(value: String) -> float:
    var text := value
    if text.ends_with("Z"):
        text = text.substr(0, text.length() - 1)
    var dot := text.find(".")
    if dot >= 0:
        text = text.substr(0, dot)
    return Time.get_unix_time_from_datetime_string(text)


func _find_by_index(items: Array, index: int) -> Dictionary:
    for item in items:
        if int(item.get("index", -1)) == index:
            return item
    return {}


func _optional_text(value: Variant) -> String:
    if value == null:
        return ""
    return str(value)


func _send_intent(intent: Dictionary) -> String:
    _intent_counter += 1
    return Net.send(Protocol.MSG_INTENT, {
        "clientIntentId": "ui-%d-%d" % [Time.get_ticks_usec(), _intent_counter],
        "intent": intent,
    })


func _on_buy(index: int) -> void:
    var intent := {"kind": Protocol.INTENT_BUY_MINION, "slot": index}
    _target_requests[_send_intent(intent)] = intent


func _on_play(hand_index: int) -> void:
    var intent := {"kind": Protocol.INTENT_PLAY_CARD, "handIndex": hand_index}
    _target_requests[_send_intent(intent)] = intent


func _on_sell(minion_id: String) -> void:
    _send_intent({"kind": Protocol.INTENT_SELL_MINION, "minionId": minion_id})


func _on_rearrange(from_index: int, to_index: int) -> void:
    if to_index < 0:
        return
    _send_intent({
        "kind": Protocol.INTENT_REARRANGE_BOARD,
        "fromIndex": from_index,
        "toIndex": to_index,
    })


func _on_reroll() -> void:
    _send_intent({"kind": Protocol.INTENT_REROLL_SHOP})


func _on_upgrade() -> void:
    _send_intent({"kind": Protocol.INTENT_UPGRADE_ERA})


func _on_freeze() -> void:
    _send_intent({"kind": Protocol.INTENT_FREEZE_SHOP, "slot": null})


func _on_hero_power() -> void:
    var power: Dictionary = GameStore.view.get("self", {}).get("heroPower", {})
    var skill_id := str(power.get("skillId", ""))
    if skill_id == "":
        return
    _request_optional_target(
        {"kind": Protocol.INTENT_USE_HERO_POWER, "skillId": skill_id},
        "英雄技能目标（可跳过）",
    )


func _request_optional_target(intent: Dictionary, title: String) -> void:
    var self_state: Dictionary = GameStore.view.get("self", {})
    if Array(self_state.get("board", [])).is_empty():
        _send_intent(intent)
        return
    _target_intent = intent
    _target_title = title
    _target_required = false
    _refresh_overlay_now()


func _on_end_recruit() -> void:
    _send_intent({"kind": Protocol.INTENT_END_RECRUIT})


func _on_choose(choice_index: int) -> void:
    if _deadline_expired:
        return
    _send_intent({"kind": Protocol.INTENT_CHOOSE_DISCOVER, "choice": choice_index})


func _on_target_chosen(minion_id: String) -> void:
    if _target_intent.is_empty():
        return
    var intent := _target_intent.duplicate()
    intent["targetMinionId"] = minion_id
    _target_intent = {}
    _send_intent(intent)
    _refresh_overlay_now()


func _on_target_skip() -> void:
    if _target_intent.is_empty():
        return
    var intent := _target_intent
    _target_intent = {}
    _send_intent(intent)
    _refresh_overlay_now()


func _on_target_cancel() -> void:
    _target_intent = {}
    _refresh_overlay_now()


func _refresh_overlay_now() -> void:
    var self_state: Dictionary = GameStore.view.get("self", {})
    _refresh_overlay(self_state, bool(self_state.get("recruited", false)))


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    var intent_id := str(GameStore.last_error.get("clientIntentId", ""))
    _status.text = "%s：%s" % [code, str(GameStore.last_error.get("message", ""))]
    if code == Protocol.ERROR_NO_VALID_TARGET and _target_requests.has(intent_id):
        _target_intent = _target_requests[intent_id]
        _target_requests.erase(intent_id)
        _target_title = "该操作需要目标"
        _target_required = true
        _refresh_overlay_now()


func _on_game_over() -> void:
    get_tree().change_scene_to_file("res://scenes/result.tscn")
