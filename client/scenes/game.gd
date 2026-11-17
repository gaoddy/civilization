extends Control
## 对局：按酒馆战棋结构呈现服务端 PlayerView 与 affordances，操作只发协议意图。
##
## 布局：左栏积分榜 — 中区（招募：商店、我方棋盘、手牌；战斗：双方随从、
## 结果横幅）— 右栏（场景身份、回合与倒计时、我的英雄与操作）— 状态行（含断线提示）。

## 每次 `_send_intent` 先发一条，供冒烟与调试断言意图内容。
signal intent_sent(intent: Dictionary)

## 服务端演示节奏：单条战斗动作的播放时长（与 room-service 的 PACE_MS 一致）。
const COMBAT_PACE_MS := 320

## 服务端演示节奏：结果横幅停留（与 room-service 的 TAIL_MS 一致）。
const COMBAT_TAIL_MS := 2000

# 页面级状态与门槛。
var _intent_counter := 0
var _target_requests: Dictionary = {}
var _target_intent: Dictionary = {}
var _target_title := "选择目标"
var _target_required := false
var _elapsed := 0.0
var _deadline_expired := false
var _view_stale := false
## 每次 `_refresh` 递增；拖拽数据带上它，落点回调据此判断拖拽是否已过期。
var _view_epoch := 0
## 队首待选是目标待选：该座位除选目标外不能操作，手牌不可拖。
var _target_pending := false

## 已回报「演示播完」的回合；同一回合只回报一次。
var _combat_ack_turn := -1

# 状态行：错误与断线提示共用。
var _status: Label
## 状态行当前承载的是连接提示：恢复连接时据此清空，避免误清错误文案。
var _connection_notice := false

# 左栏积分榜。
var _players_panel: GamePlayersRail

# 中区 · 招募与战斗。
var _recruit_view: GameRecruitView
var _combat_view: GameCombatView
var _hand_row: HFlowContainer

# 右栏。
var _side_panel: GameSidePanel

var _overlay: Control

# 随从详情：覆盖全屏但不拦截输入，层级位于选择浮层之上。
var _detail: GameDetailLayer


func _ready() -> void:
    _build()
    GameStore.view_changed.connect(_refresh)
    GameStore.error_received.connect(_on_error)
    GameStore.game_over.connect(_on_game_over)
    Net.connected.connect(_refresh_connection)
    Net.disconnected.connect(_refresh_connection)
    _refresh()
    _update_timer()


# ── 构建 ──────────────────────────────────────────────────────────────────

func _build() -> void:
    var background := ColorRect.new()
    background.color = UITheme.color("color-bg")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(background)

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-xl"))
    add_child(margin)

    var root := VBoxContainer.new()
    root.add_theme_constant_override("separation", UITheme.space("space-md"))
    margin.add_child(root)

    var body := HBoxContainer.new()
    body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override("separation", UITheme.space("space-lg"))
    root.add_child(body)

    _players_panel = GamePlayersRail.new()
    body.add_child(_players_panel)
    body.add_child(_build_center())
    _side_panel = GameSidePanel.new()
    _side_panel.hero_power_pressed.connect(_on_hero_power)
    _side_panel.end_recruit_pressed.connect(_on_end_recruit)
    body.add_child(_side_panel)

    _status = UI.label("", UITheme.font_size("font-body-sm"))
    _status.name = "StatusLabel"
    _status.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    root.add_child(_status)

    _overlay = Control.new()
    _overlay.name = "ChooserOverlay"
    _overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
    _overlay.visible = false
    add_child(_overlay)

    _detail = GameDetailLayer.new()
    add_child(_detail)
    _players_panel.detail = _detail
    _recruit_view.detail = _detail
    _combat_view.detail = _detail


func _build_center() -> Control:
    var center := VBoxContainer.new()
    center.name = "CenterPanel"
    center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    center.size_flags_vertical = Control.SIZE_EXPAND_FILL
    center.add_theme_constant_override("separation", UITheme.space("space-md"))
    _recruit_view = GameRecruitView.new()
    _recruit_view.buy.connect(_on_buy)
    _recruit_view.reroll_pressed.connect(_on_reroll)
    _recruit_view.upgrade_pressed.connect(_on_upgrade)
    _recruit_view.freeze_pressed.connect(_on_freeze)
    _recruit_view.board_dropped.connect(_on_board_dropped)
    _recruit_view.sell_dropped.connect(_on_sell_dropped)
    _recruit_view.sell_hover.connect(_on_sell_hover)
    center.add_child(_recruit_view)

    _combat_view = GameCombatView.new()
    center.add_child(_combat_view)

    center.add_child(_build_hand_panel())
    return center


func _build_hand_panel() -> Control:
    var hand_panel := UI.card_panel("surface")
    hand_panel.name = "HandPanel"
    var hand_box := VBoxContainer.new()
    hand_box.add_theme_constant_override("separation", UITheme.space("space-xs"))
    hand_panel.add_child(hand_box)
    hand_box.add_child(UI.section_title("手牌"))

    var hand_scroll := ScrollContainer.new()
    hand_scroll.custom_minimum_size = Vector2(0, UITheme.metric("hand-card-height") + UITheme.space("space-md"))
    hand_scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    hand_box.add_child(hand_scroll)
    _hand_row = _flow_row("HandRow")
    hand_scroll.add_child(_hand_row)
    return hand_panel


func _flow_row(node_name: String) -> HFlowContainer:
    var row := HFlowContainer.new()
    row.name = node_name
    row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_theme_constant_override("h_separation", UITheme.space("space-sm"))
    row.add_theme_constant_override("v_separation", UITheme.space("space-sm"))
    return row


# ── 刷新 ──────────────────────────────────────────────────────────────────

func _process(delta: float) -> void:
    _elapsed += delta
    if _elapsed >= 0.25:
        _elapsed = 0.0
        _update_timer()
    if _combat_view.playing():
        _combat_view.advance(delta)
    _report_combat_played()


func _refresh() -> void:
    _detail.reset()
    var view: Dictionary = GameStore.view
    if view.is_empty():
        _status.text = "等待对局状态…"
        return

    var self_state: Dictionary = view.get("self", {})
    var recruited := bool(self_state.get("recruited", false))
    var phase := str(view.get("phase", GameStore.phase))
    _view_stale = int(view.get("turn", 0)) != GameStore.turn
    _view_epoch += 1
    var pending: Array = self_state.get("pendingChoices", [])
    _target_pending = not pending.is_empty() and str(pending[0].get("kind", "")) == "target"
    var state := _build_state(view, self_state, phase, recruited)

    _players_panel.refresh(state)

    var in_combat := phase == Protocol.PHASE_COMBAT or phase == Protocol.PHASE_SETTLE
    _set_combat_mode(in_combat)
    if in_combat:
        _combat_view.refresh(state)
    else:
        _combat_view.reset()
        _combat_ack_turn = -1
        _recruit_view.refresh(state)
    _refresh_hand(self_state, phase)

    _side_panel.refresh(state)
    _refresh_overlay(self_state, recruited)
    _refresh_connection()
    _prune_target_requests()


## 组装本次刷新传给各视图的快照；视图只读，不回写页面状态。
func _build_state(
    view: Dictionary,
    self_state: Dictionary,
    phase: String,
    recruited: bool,
) -> GameUiState:
    var state := GameUiState.new()
    state.view = view
    state.self_state = self_state
    state.phase = phase
    state.recruited = recruited
    state.stale = _view_stale
    state.expired = _deadline_expired
    state.target_pending = _target_pending
    state.connected = Net.is_open()
    state.epoch = _view_epoch
    state.can_play = _can_play_now()
    state.board_slots = maxi(int(view.get("boardSlots", 7)), 1)
    state.hp_cap = maxi(int(view.get("hpCap", 40)), 1)
    return state


static func drag_gate(
    phase: String,
    stale: bool,
    expired: bool,
    connected: bool,
    target_pending: bool,
    playable: bool
) -> bool:
    return (
        phase == Protocol.PHASE_RECRUIT
        and not stale
        and not expired
        and connected
        and not target_pending
        and playable
    )


## 手牌：只支持拖拽出牌，点击不产生意图。
func _refresh_hand(self_state: Dictionary, phase: String) -> void:
    UI.clear(_hand_row)
    var hand: Array = self_state.get("hand", [])
    if hand.is_empty():
        _hand_row.add_child(UI.empty_hint("手牌为空"))
        return
    for card in hand:
        var index := int(card.get("index", 0))
        var card_id := str(card.get("cardId", ""))
        var playable := bool(card.get("playable", false))
        var card_kind := "minion" if str(CardDB.card(card_id).get("card_type", "")) == "随从" else "effect"
        var entry := HandCard.new()
        entry.name = "HandCard%d" % index
        entry.custom_minimum_size = Vector2(UITheme.metric("hand-card-width"), UITheme.metric("hand-card-height"))
        entry.tooltip_text = CardDB.display_text(card_id)
        if card.get("disabledReason") != null:
            entry.tooltip_text = UI.error_text(str(card.get("disabledReason")))

        var box := VBoxContainer.new()
        box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
        box.mouse_filter = Control.MOUSE_FILTER_IGNORE
        entry.add_child(box)
        var name_label := UI.label(CardDB.display_name(card_id), UITheme.font_size("font-body-sm"), false)
        name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
        name_label.max_lines_visible = 2
        name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
        box.add_child(name_label)
        var text_label := UI.label(CardDB.display_text(card_id), UITheme.font_size("font-caption"), false)
        text_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
        text_label.max_lines_visible = 4
        text_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
        text_label.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
        box.add_child(text_label)

        entry.payload = {
            "epoch": _view_epoch,
            "handIndex": index,
            "cardId": card_id,
            "cardKind": card_kind,
            "requiresTarget": bool(card.get("requiresTarget", false)),
        }
        entry.drag_allowed = drag_gate(
            phase,
            _view_stale,
            _deadline_expired,
            Net.is_open(),
            _target_pending,
            playable
        )
        if not entry.drag_allowed:
            UI.apply_dim(entry)
        _hand_row.add_child(entry)


func _set_combat_mode(in_combat: bool) -> void:
    _recruit_view.visible = not in_combat
    _combat_view.visible = in_combat
    _side_panel.set_combat_mode(in_combat)


## 演示播完后向服务端回报一次；断线时不发送也不重试（服务端把断线视为已回报）。
func _report_combat_played() -> void:
    if GameStore.phase != Protocol.PHASE_COMBAT:
        return
    if not GameStore.combat_is_current() or not _combat_view.finished():
        return
    if _combat_ack_turn == GameStore.turn:
        return
    _combat_ack_turn = GameStore.turn
    if not Net.is_open():
        return
    Net.send(Protocol.MSG_COMBAT_PLAYED, {})


func _refresh_connection() -> void:
    if _status == null:
        return
    var disconnected := not Net.is_open() and not GameStore.view.is_empty()
    if disconnected:
        # 有连接句柄但未就绪 = 正在重连（dim）；句柄为空 = 断线（danger）。
        var reconnecting := Net.socket != null
        _status.text = "正在重连…" if reconnecting else "未连接 · 正在重连"
        _status.add_theme_color_override(
            "font_color",
            UITheme.color("color-text-dim") if reconnecting else UITheme.color("color-danger"),
        )
        _connection_notice = true
        return
    if _connection_notice:
        _status.text = ""
        # 恢复到初始化时的基线颜色；只删除覆盖会丢掉 dim 基线。
        _status.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
        _connection_notice = false


## 倒计时由右栏维护；页面按「是否存在截止时间」决定清零，避免把已过期
## （秒数为 0）误判成没有截止时间。
func _update_timer() -> void:
    if _side_panel == null:
        return
    if not _side_panel.has_deadline():
        _set_deadline_expired(false)
        return
    _set_deadline_expired(_side_panel.seconds_left() <= 0)


func _set_deadline_expired(value: bool) -> void:
    if _deadline_expired == value:
        return
    _deadline_expired = value
    _refresh()


# ── 弹层 ──────────────────────────────────────────────────────────────────

func _refresh_overlay(self_state: Dictionary, recruited: bool) -> void:
    UI.clear(_overlay)
    var choices: Array = self_state.get("pendingChoices", [])
    var phase := str(GameStore.view.get("phase", GameStore.phase))
    # 待选只在招募阶段可操作：跨回合保留的发现/进化待选不能盖在战斗演示上。
    if _view_stale or _deadline_expired or recruited or phase != Protocol.PHASE_RECRUIT:
        _target_intent = {}
        _overlay.visible = false
    else:
        var head: Dictionary = choices[0] if not choices.is_empty() else {}
        if str(head.get("kind", "")) == "target":
            # 服务端目标待选优先：丢弃英雄技能的客户端目标态，改为选随从结算战吼。
            _target_intent = {}
            _build_target_pending_overlay(head, self_state)
        elif not _target_intent.is_empty():
            _build_target_overlay(self_state)
        elif not choices.is_empty():
            _build_choice_overlay(choices[0])
        else:
            _overlay.visible = false
    # 浮层可见性只在本函数收口；打开浮层时同步遮住详情层。
    _detail.blocked = _overlay.visible


func _overlay_frame(title: String, parent: VBoxContainer) -> void:
    _overlay.visible = true
    _detail.hide_panel()
    var shade := ColorRect.new()
    shade.color = UITheme.alpha("color-bg", 0.72)
    shade.set_anchors_preset(Control.PRESET_FULL_RECT)
    _overlay.add_child(shade)
    var center := CenterContainer.new()
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    _overlay.add_child(center)
    var panel := UI.card_panel("raised")
    panel.custom_minimum_size = Vector2(420, 0)
    center.add_child(panel)
    var margin := MarginContainer.new()
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-xl"))
    panel.add_child(margin)
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    margin.add_child(box)
    box.add_child(UI.label(title, UITheme.font_size("font-display"), false))
    for child in parent.get_children():
        parent.remove_child(child)
        box.add_child(child)


func _build_choice_overlay(choice: Dictionary) -> void:
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    var kind := str(choice.get("kind", ""))
    if kind == "discover":
        box.add_child(UI.label("发现：选择一张卡牌", UITheme.font_size("font-body")))
        var index := 0
        for card_id in choice.get("candidates", []):
            var button := UI.styled_button("%s\n%s" % [
                CardDB.display_name(str(card_id)),
                CardDB.display_text(str(card_id)),
            ], "slot", 64)
            button.pressed.connect(_on_choose.bind(index))
            box.add_child(button)
            index += 1
    else:
        box.add_child(UI.label("进化：选择一项", UITheme.font_size("font-body")))
        var index := 0
        for option in choice.get("options", []):
            var button := UI.styled_button(str(option), "slot", 44)
            button.pressed.connect(_on_choose.bind(index))
            box.add_child(button)
            index += 1
    _overlay_frame("待选择", box)


## 战吼目标待选：只列我方棋盘随从，没有取消与跳过（必须选一个）。
func _build_target_pending_overlay(choice: Dictionary, self_state: Dictionary) -> void:
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    box.add_child(UI.label("选择我方一个随从结算战吼", UITheme.font_size("font-body")))
    var source_id := str(choice.get("sourceMinionId", ""))
    for minion in self_state.get("board", []):
        var minion_id := str(minion.get("minionId", ""))
        var prefix := "★ " if minion_id == source_id else ""
        var button := UI.styled_button("%s%s  %d/%d" % [
            prefix,
            CardDB.display_name(str(minion.get("cardId", ""))),
            int(minion.get("attack", 0)),
            int(minion.get("health", 0)),
        ], "slot", 40)
        if minion_id == source_id:
            button.add_theme_color_override("font_color", UITheme.color("color-accent"))
        button.pressed.connect(_on_choose_target.bind(minion_id))
        box.add_child(button)
    _overlay_frame("战吼目标", box)


func _build_target_overlay(self_state: Dictionary) -> void:
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    box.add_child(UI.label(_target_title, UITheme.font_size("font-body")))
    if not _target_required:
        var skip := UI.styled_button("不指定目标", "default", 36)
        skip.pressed.connect(_on_target_skip)
        box.add_child(skip)
    for minion in self_state.get("board", []):
        var button := UI.styled_button("%s  %d/%d" % [
            CardDB.display_name(str(minion.get("cardId", ""))),
            int(minion.get("attack", 0)),
            int(minion.get("health", 0)),
        ], "slot", 40)
        button.pressed.connect(_on_target_chosen.bind(str(minion.get("minionId", ""))))
        box.add_child(button)
    var cancel := UI.styled_button("取消", "default", 36)
    cancel.pressed.connect(_on_target_cancel)
    box.add_child(cancel)
    _overlay_frame("选择目标", box)


# ── 意图 ──────────────────────────────────────────────────────────────────

func _send_intent(intent: Dictionary) -> String:
    intent_sent.emit(intent)
    _intent_counter += 1
    var intent_id := Net.send(Protocol.MSG_INTENT, {
        "clientIntentId": "ui-%d-%d" % [Time.get_ticks_usec(), _intent_counter],
        "intent": intent,
    })
    if intent_id == "":
        _status.text = "未连接，操作未发出"
    return intent_id


func _remember_target_request(intent_id: String, intent: Dictionary) -> void:
    if intent_id == "":
        return
    if _target_requests.size() >= 32:
        _target_requests.clear()
    _target_requests[intent_id] = {"intent": intent, "turn": GameStore.turn}


func _prune_target_requests() -> void:
    for intent_id in _target_requests.keys():
        var entry: Dictionary = _target_requests[intent_id]
        if int(entry.get("turn", 0)) != GameStore.turn:
            _target_requests.erase(intent_id)


func _on_choose_target(minion_id: String) -> void:
    if _deadline_expired or _view_stale:
        return
    if str(GameStore.view.get("phase", GameStore.phase)) != Protocol.PHASE_RECRUIT:
        return
    _send_intent({"kind": Protocol.INTENT_CHOOSE_TARGET, "minionId": minion_id})


func _on_buy(index: int) -> void:
    # 买入只入手牌，不涉及目标，因此不登记补选目标。
    _send_intent({"kind": Protocol.INTENT_BUY_MINION, "slot": index})


## 棋盘落点回调：出牌的唯一入口。拖拽途中视图已重建时按取消处理。
##
## 拖拽期间状态可能已经变化（视图重建、换回合、开战、断线、进入目标待选），
## 落点必须重新过一遍与拖拽起始同一组门控，任一不满足都不构造意图。
func _on_board_dropped(data: Dictionary, position: int, target_minion_id: String) -> void:
    if int(data.get("epoch", -1)) != _view_epoch:
        return
    if not _can_play_now():
        return
    if data.has("minionId"):
        var from_index := int(data.get("fromIndex", 0))
        if position == from_index:
            return
        _send_intent({
            "kind": Protocol.INTENT_REARRANGE_BOARD,
            "fromIndex": from_index,
            "toIndex": position,
        })
        return
    var intent := {"kind": Protocol.INTENT_PLAY_CARD, "handIndex": int(data.get("handIndex", 0))}
    if str(data.get("cardKind", "")) == "minion":
        intent["position"] = position
    elif target_minion_id != "":
        intent["targetMinionId"] = target_minion_id
    _remember_target_request(_send_intent(intent), intent)


## 落点门控的实时复查：与 `drag_gate` 同源，但用手牌之外的状态（是否已结束招募）。
static func drop_gate(
    phase: String,
    stale: bool,
    expired: bool,
    connected: bool,
    target_pending: bool,
    recruited: bool
) -> bool:
    return (
        phase == Protocol.PHASE_RECRUIT
        and not stale
        and not expired
        and connected
        and not target_pending
        and not recruited
    )


## 落点门控的实时复查：拖拽期间状态可能已经变化，落点按当前页面状态重判。
func _can_play_now() -> bool:
    return drop_gate(
        str(GameStore.view.get("phase", GameStore.phase)),
        _view_stale,
        _deadline_expired,
        Net.is_open(),
        _target_pending,
        bool(GameStore.view.get("self", {}).get("recruited", false))
    )


func _on_sell_hover(hovering: bool, refund: int) -> void:
    _recruit_view.set_sell_hint(hovering, refund)


func _on_sell_dropped(payload: Dictionary) -> void:
    if int(payload.get("epoch", -1)) != _view_epoch:
        return
    if not _can_play_now():
        return
    _send_intent({
        "kind": Protocol.INTENT_SELL_MINION,
        "minionId": str(payload.get("minionId", "")),
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
    if _deadline_expired or str(GameStore.view.get("phase", GameStore.phase)) != Protocol.PHASE_RECRUIT:
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


# ── 错误与结算 ────────────────────────────────────────────────────────────

func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    var intent_id := str(GameStore.last_error.get("clientIntentId", ""))
    _status.text = UI.error_text(code)
    _status.tooltip_text = str(GameStore.last_error.get("message", ""))
    if code == Protocol.ERROR_NO_VALID_TARGET and _target_requests.has(intent_id):
        var entry: Dictionary = _target_requests[intent_id]
        _target_intent = entry.get("intent", {})
        _target_requests.erase(intent_id)
        _target_title = "该操作需要目标"
        _target_required = true
        _refresh_overlay_now()
        return
    match UI.error_route(code):
        "lobby":
            GameStore.reset_room()
            UI.navigate(self, "res://scenes/lobby.tscn")
        "login":
            GameStore.reset_session()
            UI.navigate(self, "res://scenes/login.tscn")


func _on_game_over() -> void:
    UI.navigate(self, "res://scenes/result.tscn")
