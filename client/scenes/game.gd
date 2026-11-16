extends Control
## 对局：按酒馆战棋结构呈现服务端 PlayerView 与 affordances，操作只发协议意图。
##
## 布局：顶栏（回合/计时/属性）— 左栏积分榜 — 中区（招募：商店、我方棋盘、手牌；
## 战斗：双方英雄与随从、结果横幅）— 右栏（我的英雄、英雄技能、结束招募、战报）— 状态行。

## 每次 `_send_intent` 先发一条，供冒烟与调试断言意图内容。
signal intent_sent(intent: Dictionary)

## 服务端演示节奏：单条战斗动作的播放时长（与 room-service 的 PACE_MS 一致）。
const COMBAT_PACE_MS := 320

## 服务端演示节奏：结果横幅停留（与 room-service 的 TAIL_MS 一致）。
const COMBAT_TAIL_MS := 2000

const PHASE_TEXT := {
    Protocol.PHASE_RECRUIT: "招募",
    Protocol.PHASE_COMBAT: "战斗",
    Protocol.PHASE_SETTLE: "结算",
    Protocol.PHASE_GAME_OVER: "结束",
}

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

# 顶栏。
var _turn_label: Label
var _timer_label: Label
var _hp_pill: Label
var _era_pill: Label
var _gold_pill: Label
var _conn_label: Label
var _status: Label

# 左栏积分榜。
var _players_rail: VBoxContainer

# 中区 · 招募。
var _recruit_panel: VBoxContainer
var _board_row: BoardDrop
var _board_hint: Label
var _shop_row: HFlowContainer
var _sell_drop: SellDrop
var _sell_hint: Label
var _hand_row: HFlowContainer
var _reroll_button: Button
var _upgrade_button: Button
var _freeze_button: Button

# 中区 · 战斗。
var _combat_root: Control
var _combat_body: VBoxContainer
var _combat_opponent_strip: HBoxContainer
var _combat_opponent_row: HFlowContainer
var _combat_banner: Label
var _combat_own_row: HFlowContainer
var _combat_self_strip: HBoxContainer
## 演示播放器（纯数据）与当前演示所属回合。
var _combat := CombatPlayback.new()
var _combat_turn := -1

# 右栏。
var _self_hero_box: VBoxContainer
var _hero_power_button: Button
var _end_button: Button
var _log_text: RichTextLabel

var _overlay: Control

# 随从详情：覆盖全屏但不拦截输入，层级位于选择浮层之上。
var _detail_layer: Control
var _detail_panel: PanelContainer
var _detail_cards: Array = []
var _hover_card: Control = null


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

    root.add_child(_build_header())

    var body := HBoxContainer.new()
    body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override("separation", UITheme.space("space-lg"))
    root.add_child(body)

    body.add_child(_build_players_rail())
    body.add_child(_build_center())
    body.add_child(_build_side())

    _status = UI.label("", UITheme.font_size("font-body-sm"))
    _status.name = "StatusLabel"
    _status.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    root.add_child(_status)

    _overlay = Control.new()
    _overlay.name = "ChooserOverlay"
    _overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
    _overlay.visible = false
    add_child(_overlay)

    _build_detail_layer()


func _build_detail_layer() -> void:
    _detail_layer = Control.new()
    _detail_layer.name = "MinionDetailLayer"
    _detail_layer.set_anchors_preset(Control.PRESET_FULL_RECT)
    _detail_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _detail_layer.visible = false
    add_child(_detail_layer)

    _detail_panel = UI.card_panel("raised")
    _detail_panel.name = "MinionDetailPanel"
    _detail_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _detail_panel.custom_minimum_size = Vector2(UITheme.metric("minion-card-width") + UITheme.space("space-2xl"), 0)
    _detail_panel.visible = false
    _detail_layer.add_child(_detail_panel)


func _build_header() -> Control:
    var panel := UI.card_panel("surface")
    panel.name = "Header"
    panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    panel.add_child(row)

    _turn_label = UI.label("", UITheme.font_size("font-title-lg"), false)
    _turn_label.name = "TurnLabel"
    row.add_child(_turn_label)

    _conn_label = UI.label("", UITheme.font_size("font-body-sm"), false)
    _conn_label.name = "ConnectionLabel"
    _conn_label.add_theme_color_override("font_color", UITheme.color("color-danger"))
    row.add_child(_conn_label)

    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_child(spacer)

    _hp_pill = UI.pill_label("", "danger", "font-title")
    _hp_pill.name = "HpPill"
    row.add_child(_hp_pill)
    _era_pill = UI.pill_label("", "info", "font-title")
    _era_pill.name = "EraPill"
    row.add_child(_era_pill)
    _gold_pill = UI.pill_label("", "accent", "font-title")
    _gold_pill.name = "GoldPill"
    row.add_child(_gold_pill)
    _timer_label = UI.pill_label("—", "accent", "font-title")
    _timer_label.name = "TimerLabel"
    row.add_child(_timer_label)
    return panel


func _build_players_rail() -> Control:
    var panel := UI.card_panel("surface")
    panel.name = "PlayersPanel"
    panel.custom_minimum_size = Vector2(UITheme.metric("rail-width"), 0)
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    panel.add_child(box)
    box.add_child(UI.section_title("积分榜"))

    var scroll := ScrollContainer.new()
    scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    box.add_child(scroll)

    _players_rail = VBoxContainer.new()
    _players_rail.name = "PlayersRail"
    _players_rail.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _players_rail.add_theme_constant_override("separation", UITheme.space("space-xs"))
    scroll.add_child(_players_rail)
    return panel


func _build_center() -> Control:
    var center := VBoxContainer.new()
    center.name = "CenterPanel"
    center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    center.size_flags_vertical = Control.SIZE_EXPAND_FILL
    center.add_theme_constant_override("separation", UITheme.space("space-md"))
    center.add_child(_build_recruit_panel())
    center.add_child(_build_combat_panel())
    center.add_child(_build_hand_panel())
    return center


func _build_recruit_panel() -> Control:
    _recruit_panel = VBoxContainer.new()
    _recruit_panel.name = "RecruitPanel"
    _recruit_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _recruit_panel.add_theme_constant_override("separation", UITheme.space("space-sm"))

    # 商店：6 个槽位与刷新/升时代/冻结；整块面板同时是棋盘随从的出售落点。
    var shop_panel := SellDrop.new()
    shop_panel.name = "ShopPanel"
    _sell_drop = shop_panel
    shop_panel.sell_dropped.connect(_on_sell_dropped)
    shop_panel.hover_changed.connect(_on_sell_hover)
    var shop_box := VBoxContainer.new()
    shop_box.add_theme_constant_override("separation", UITheme.space("space-xs"))
    shop_panel.add_child(shop_box)

    var shop_head := HBoxContainer.new()
    shop_head.add_theme_constant_override("separation", UITheme.space("space-sm"))
    shop_head.add_child(UI.section_title("商店"))
    _sell_hint = UI.label("", UITheme.font_size("font-caption"), false)
    _sell_hint.name = "SellHint"
    _sell_hint.add_theme_color_override("font_color", UITheme.color("color-accent"))
    _sell_hint.visible = false
    shop_head.add_child(_sell_hint)
    var shop_spacer := Control.new()
    shop_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    shop_head.add_child(shop_spacer)

    _reroll_button = UI.styled_button("刷新", "default", 28)
    _reroll_button.name = "RerollButton"
    _reroll_button.pressed.connect(_on_reroll)
    shop_head.add_child(_reroll_button)

    _upgrade_button = UI.styled_button("升时代", "default", 28)
    _upgrade_button.name = "UpgradeButton"
    _upgrade_button.pressed.connect(_on_upgrade)
    shop_head.add_child(_upgrade_button)

    _freeze_button = UI.styled_button("冻结商店", "default", 28)
    _freeze_button.name = "FreezeButton"
    _freeze_button.pressed.connect(_on_freeze)
    shop_head.add_child(_freeze_button)
    shop_box.add_child(shop_head)

    _shop_row = _flow_row("ShopRow")
    shop_box.add_child(_shop_row)
    _recruit_panel.add_child(shop_panel)

    # 我方棋盘。
    var board_panel := UI.card_panel("surface")
    board_panel.name = "BoardPanel"
    board_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    var board_box := VBoxContainer.new()
    board_box.add_theme_constant_override("separation", UITheme.space("space-xs"))
    board_panel.add_child(board_box)

    var board_head := HBoxContainer.new()
    board_head.add_child(UI.section_title("我方棋盘"))
    _board_hint = UI.label("", UITheme.font_size("font-caption"), false)
    _board_hint.name = "BoardHint"
    _board_hint.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    _board_hint.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _board_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
    board_head.add_child(_board_hint)
    board_box.add_child(board_head)

    var top_spacer := Control.new()
    top_spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
    board_box.add_child(top_spacer)

    var board_scroll := ScrollContainer.new()
    board_scroll.custom_minimum_size = Vector2(0, UITheme.metric("minion-card-height"))
    board_scroll.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    board_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    board_box.add_child(board_scroll)
    _board_row = BoardDrop.new()
    _board_row.name = "BoardRow"
    _board_row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _board_row.add_theme_constant_override("h_separation", UITheme.space("space-sm"))
    _board_row.add_theme_constant_override("v_separation", UITheme.space("space-sm"))
    _board_row.dropped.connect(_on_board_dropped)
    board_scroll.add_child(_board_row)

    var bottom_spacer := Control.new()
    bottom_spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
    board_box.add_child(bottom_spacer)
    _recruit_panel.add_child(board_panel)
    return _recruit_panel


## 手牌面板：招募与演示共用，演示期间只读。
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


func _build_combat_panel() -> Control:
    _combat_root = UI.card_panel("surface")
    _combat_root.name = "CombatPanel"
    _combat_root.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _combat_root.visible = false

    _combat_body = VBoxContainer.new()
    _combat_body.name = "CombatBody"
    _combat_body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _combat_body.add_theme_constant_override("separation", UITheme.space("space-sm"))
    _combat_root.add_child(_combat_body)

    _combat_opponent_strip = HBoxContainer.new()
    _combat_opponent_strip.name = "CombatOpponentStrip"
    _combat_opponent_strip.add_theme_constant_override("separation", UITheme.space("space-md"))
    _combat_body.add_child(_combat_opponent_strip)

    _combat_body.add_child(UI.section_title("对手随从"))
    _combat_opponent_row = _flow_row("CombatOpponentRow")
    _combat_body.add_child(_combat_opponent_row)

    _combat_banner = UI.label("", UITheme.font_size("font-display"), false)
    _combat_banner.name = "CombatBanner"
    _combat_banner.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _combat_banner.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _combat_body.add_child(_combat_banner)

    _combat_body.add_child(UI.section_title("我方随从"))
    _combat_own_row = _flow_row("CombatOwnRow")
    _combat_body.add_child(_combat_own_row)

    var strip_spacer := Control.new()
    strip_spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _combat_body.add_child(strip_spacer)

    _combat_self_strip = HBoxContainer.new()
    _combat_self_strip.name = "CombatSelfStrip"
    _combat_self_strip.add_theme_constant_override("separation", UITheme.space("space-md"))
    _combat_body.add_child(_combat_self_strip)
    return _combat_root


func _build_side() -> Control:
    var side := VBoxContainer.new()
    side.name = "SidePanel"
    side.custom_minimum_size = Vector2(UITheme.metric("side-width"), 0)
    side.add_theme_constant_override("separation", UITheme.space("space-sm"))

    var hero_panel := UI.card_panel("surface")
    hero_panel.name = "SelfHeroPanel"
    _self_hero_box = VBoxContainer.new()
    _self_hero_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    hero_panel.add_child(_self_hero_box)
    side.add_child(hero_panel)

    _hero_power_button = UI.styled_button("英雄技能", "default", 40)
    _hero_power_button.name = "HeroPowerButton"
    _hero_power_button.pressed.connect(_on_hero_power)
    side.add_child(_hero_power_button)

    _end_button = UI.styled_button("结束招募", "primary", 46)
    _end_button.name = "EndButton"
    _end_button.pressed.connect(_on_end_recruit)
    side.add_child(_end_button)

    var log_panel := UI.card_panel("surface")
    log_panel.name = "LogPanel"
    log_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    var log_box := VBoxContainer.new()
    log_box.add_theme_constant_override("separation", UITheme.space("space-xs"))
    log_panel.add_child(log_box)
    log_box.add_child(UI.section_title("战报与事件"))
    _log_text = RichTextLabel.new()
    _log_text.name = "LogText"
    _log_text.fit_content = false
    _log_text.scroll_active = true
    _log_text.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _log_text.add_theme_font_size_override("normal_font_size", UITheme.font_size("font-body-sm"))
    _log_text.add_theme_color_override("default_color", UITheme.color("color-text-dim"))
    log_box.add_child(_log_text)
    side.add_child(log_panel)
    return side


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
    if _combat_playing():
        _combat.advance(delta * 1000.0)
        _render_combat_rows()
        _refresh_combat_banner()


func _refresh() -> void:
    _hide_detail()
    _detail_cards.clear()
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

    _refresh_header(view, self_state, phase)
    _refresh_players(view)

    var in_combat := phase == Protocol.PHASE_COMBAT or phase == Protocol.PHASE_SETTLE
    _set_combat_mode(in_combat)
    if in_combat:
        _refresh_combat(view)
    else:
        _combat_turn = -1
        _refresh_recruit(self_state, recruited, phase)
    _refresh_hand(self_state, phase)

    _refresh_side(self_state)
    _refresh_log()
    _refresh_overlay(self_state, recruited)
    _refresh_connection()
    _prune_target_requests()


func _refresh_header(view: Dictionary, self_state: Dictionary, phase: String) -> void:
    _turn_label.text = "第 %d 回合 · %s" % [
        int(view.get("turn", 1)),
        PHASE_TEXT.get(phase, phase),
    ]
    _hp_pill.text = "♥ %d" % int(self_state.get("hp", 0))
    _era_pill.text = _era_text(int(self_state.get("era", 1)))
    _gold_pill.text = "● %d 金" % int(self_state.get("gold", 0))


func _refresh_players(view: Dictionary) -> void:
    UI.clear(_players_rail)
    var players: Array = _sorted_players(view)
    if players.is_empty():
        _players_rail.add_child(UI.empty_hint("暂无玩家"))
        return
    for player in players:
        _players_rail.add_child(_seat_row(player))


func _sorted_players(view: Dictionary) -> Array:
    var players: Array = view.get("players", []).duplicate()
    players.sort_custom(func(a, b): return int(a.get("seat", 0)) < int(b.get("seat", 0)))
    return players


func _seat_row(player: Dictionary) -> PanelContainer:
    var player_id := str(player.get("playerId", ""))
    var is_self := player_id == GameStore.player_id
    var alive := bool(player.get("alive", true))
    var panel := UI.card_panel("accent-row" if is_self else "row")
    panel.name = "SeatRow"
    panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    if not alive:
        UI.apply_dim(panel)

    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    panel.add_child(box)

    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-sm"))
    box.add_child(row)
    row.add_child(UI.avatar(_optional_text(player.get("heroCardId")), UITheme.metric("avatar-md")))

    var info := VBoxContainer.new()
    info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    info.add_theme_constant_override("separation", 0)
    row.add_child(info)

    var name_row := HBoxContainer.new()
    name_row.add_theme_constant_override("separation", UITheme.space("space-xs"))
    info.add_child(name_row)

    var name_label := UI.label(_optional_text(player.get("displayName")), UITheme.font_size("font-body-sm"), false)
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    if is_self:
        name_label.add_theme_color_override("font_color", UITheme.color("color-accent"))
    name_row.add_child(name_label)

    var marks := PackedStringArray()
    if is_self:
        marks.append("我")
    if player.get("placement") != null:
        marks.append("第 %d 名" % int(player.get("placement")))
    if not alive:
        marks.append("淘汰")
    elif bool(player.get("recruited", false)):
        marks.append("已招募")
    if not marks.is_empty():
        var mark := UI.label("、".join(marks), UITheme.font_size("font-caption"), false)
        mark.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
        name_row.add_child(mark)

    var meta := UI.label("HP %d · %s · 随从%d" % [
        int(player.get("hp", 0)),
        _era_text(int(player.get("era", 1))),
        int(player.get("boardCount", 0)),
    ], UITheme.font_size("font-caption"), false)
    meta.clip_text = true
    meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    info.add_child(meta)
    return panel


func _refresh_recruit(self_state: Dictionary, recruited: bool, phase: String) -> void:
    _refresh_board(self_state, recruited, phase)
    _refresh_shop(self_state)
    _refresh_sell_gate()
    _refresh_actions(self_state, recruited, phase)


## 商店出售落点与棋盘拖拽共用一套实时门控；不满足时商店不接收棋盘卡。
func _refresh_sell_gate() -> void:
    if _sell_drop == null:
        return
    _sell_drop.enabled = _can_play_now()
    if not _sell_drop.enabled:
        _sell_hint.visible = false


func _refresh_board(self_state: Dictionary, recruited: bool, phase: String) -> void:
    UI.clear(_board_row)
    var aff: Dictionary = self_state.get("affordances", {})
    var board: Array = self_state.get("board", [])
    var slots := _board_slots()
    var can_rearrange := not _view_stale and not _deadline_expired and not _target_pending and phase == Protocol.PHASE_RECRUIT and not recruited
    _board_row.board_full = board.size() >= slots
    _board_hint.text = "随从 %d/%d" % [board.size(), slots]
    for position in board.size():
        var minion: Dictionary = board[position]
        var minion_aff := _find_by_index(aff.get("boardMinions", []), int(minion.get("index", position)))
        _board_row.add_child(_minion_card(minion, minion_aff, can_rearrange, position))
    for _position in range(board.size(), slots):
        _board_row.add_child(_empty_slot())


## 棋盘容量：以服务端下发的视图为准，字段缺失（旧服务端）时回退 7。
func _board_slots() -> int:
    return maxi(int(GameStore.view.get("boardSlots", 7)), 1)


func _empty_slot() -> PanelContainer:
    var panel := UI.card_panel("empty")
    panel.name = "EmptySlot"
    panel.custom_minimum_size = Vector2(UITheme.metric("minion-card-width"), UITheme.metric("minion-card-height"))
    var hint := UI.empty_hint("空位")
    hint.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    panel.add_child(hint)
    return panel


func _minion_card(
    minion: Dictionary,
    minion_aff: Dictionary,
    can_rearrange: bool,
    position: int,
    read_only := false,
) -> PanelContainer:
    var minion_index := int(minion.get("index", position))
    var panel := BoardCard.new()
    panel.add_theme_stylebox_override("panel", UITheme.panel_style("raised-tight"))
    panel.name = "BoardMinion%d" % minion_index
    panel.set_meta("minion_id", str(minion.get("minionId", "")))
    panel.set_meta("minion_index", minion_index)
    panel.custom_minimum_size = Vector2(UITheme.metric("minion-card-width"), 0 if read_only else UITheme.metric("minion-card-height"))

    var box := VBoxContainer.new()
    box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    panel.add_child(box)

    var head := HBoxContainer.new()
    head.mouse_filter = Control.MOUSE_FILTER_IGNORE
    head.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    box.add_child(head)
    var name_label := UI.label(CardDB.display_name(str(minion.get("cardId", ""))), UITheme.font_size("font-body-sm"), false)
    name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    head.add_child(name_label)
    if bool(minion.get("golden", false)):
        var star := UI.label("★", UITheme.font_size("font-caption"), false)
        star.mouse_filter = Control.MOUSE_FILTER_IGNORE
        star.add_theme_color_override("font_color", UITheme.color("color-accent"))
        head.add_child(star)

    var stats := UI.label("%d / %d" % [
        int(minion.get("attack", 0)),
        int(minion.get("health", 0)),
    ], UITheme.font_size("font-title"), false)
    stats.name = "Stats"
    stats.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_child(stats)

    var keywords := _join_cn(minion.get("keywords", []))
    var keyword_label := UI.label(keywords if keywords != "" else "—", UITheme.font_size("font-caption"), false)
    keyword_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    keyword_label.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    keyword_label.max_lines_visible = 1
    keyword_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    box.add_child(keyword_label)

    var spacer := Control.new()
    spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
    box.add_child(spacer)

    _attach_detail(panel, minion)
    if read_only:
        return panel

    # 棋盘卡只提供拖拽：拖到棋盘内换位，拖到商店出售；卡内不再放操作按钮。
    panel.payload = {
        "epoch": _view_epoch,
        "minionId": str(minion.get("minionId", "")),
        "fromIndex": minion_index,
        "sellable": bool(minion_aff.get("sellable", false)),
        "golden": bool(minion.get("golden", false)),
    }
    panel.drag_allowed = can_rearrange and Net.is_open()
    if not panel.drag_allowed:
        UI.apply_dim(panel)
    return panel


func _refresh_shop(self_state: Dictionary) -> void:
    UI.clear(_shop_row)
    var aff: Dictionary = self_state.get("affordances", {})
    for slot in self_state.get("shop", []):
        var index := int(slot.get("index", 0))
        var card_id := _optional_text(slot.get("cardId"))
        var button := UI.styled_button("", "slot", UITheme.metric("shop-card-height"))
        button.name = "ShopSlot%d" % index
        button.custom_minimum_size = Vector2(UITheme.metric("shop-card-width"), UITheme.metric("shop-card-height"))
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
                button.tooltip_text = UI.error_text(str(slot_aff.get("disabledReason")))
            button.pressed.connect(_on_buy.bind(index))
        _shop_row.add_child(button)


## 手牌拖拽门槛：阶段、视图新鲜度、倒计时、连接、目标待选与可打出性全部满足才允许。
##
## 抽成静态函数便于冒烟直接验证两侧判定，不必真的连上服务端。
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


func _refresh_actions(self_state: Dictionary, recruited: bool, phase: String) -> void:
    var aff: Dictionary = self_state.get("affordances", {})
    _reroll_button.text = "刷新 %d 金" % int(self_state.get("rerollCost", 0))
    _reroll_button.disabled = _view_stale or _deadline_expired or _target_pending or not bool(aff.get("canReroll", false))

    var upgrade_cost = self_state.get("upgradeCost")
    if upgrade_cost != null:
        _upgrade_button.text = "升时代 %d 金" % int(upgrade_cost)
    else:
        _upgrade_button.text = "时代已满"
    _upgrade_button.disabled = _view_stale or _deadline_expired or _target_pending or not bool(aff.get("canUpgradeEra", false))
    _upgrade_button.tooltip_text = "" if aff.get("upgradeDisabledReason") == null else UI.error_text(str(aff.get("upgradeDisabledReason")))

    var power: Dictionary = self_state.get("heroPower", {})
    _hero_power_button.text = "英雄技能（剩 %d）" % int(power.get("usesLeft", 0))
    _hero_power_button.disabled = _view_stale or _deadline_expired or _target_pending or not bool(aff.get("canUseHeroPower", false))
    _hero_power_button.tooltip_text = "" if aff.get("heroPowerDisabledReason") == null else UI.error_text(str(aff.get("heroPowerDisabledReason")))

    _freeze_button.disabled = _view_stale or _deadline_expired or _target_pending or phase != Protocol.PHASE_RECRUIT or recruited
    _end_button.text = "已结束招募" if recruited else "结束招募"
    _end_button.disabled = _view_stale or _deadline_expired or _target_pending or not bool(aff.get("canEndRecruit", false))


func _refresh_side(self_state: Dictionary) -> void:
    UI.clear(_self_hero_box)
    var hero_id := _optional_text(self_state.get("heroCardId"))
    _self_hero_box.add_child(UI.hero_tile(hero_id, UITheme.metric("side-width") - UITheme.space("space-2xl")))

    var stat_line := UI.label("生命 %d · %s · 金币 %d" % [
        int(self_state.get("hp", 0)),
        _era_text(int(self_state.get("era", 1))),
        int(self_state.get("gold", 0)),
    ], UITheme.font_size("font-body-sm"), false)
    stat_line.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    _self_hero_box.add_child(stat_line)

    var power: Dictionary = self_state.get("heroPower", {})
    var skill_id := _optional_text(power.get("skillId"))
    var skill_text := CardDB.display_text(skill_id) if skill_id != "" else ""
    if skill_text != "":
        var skill := UI.label(skill_text, UITheme.font_size("font-caption"))
        skill.max_lines_visible = 2
        skill.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
        skill.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
        _self_hero_box.add_child(skill)


func _set_combat_mode(in_combat: bool) -> void:
    _recruit_panel.visible = not in_combat
    _combat_root.visible = in_combat
    _hero_power_button.visible = not in_combat
    _end_button.visible = not in_combat


func _refresh_combat(view: Dictionary) -> void:
    _fill_combat_strip(_combat_opponent_strip, _opponent_of(view), "对手")
    _fill_combat_strip(_combat_self_strip, _self_player(view), "我")
    if not GameStore.combat_is_current():
        _combat_banner.visible = false
        UI.clear(_combat_opponent_row)
        UI.clear(_combat_own_row)
        _combat_opponent_row.add_child(UI.empty_hint("等待战斗动作…"))
        return
    var combat: Dictionary = GameStore.last_combat
    if _combat_turn != GameStore.turn:
        _start_combat(combat)
    _render_combat_rows()
    _refresh_combat_banner()


## 装载本场演示：战前阵容与动作序列，并按 `phaseDeadline` 反推续播进度。
func _start_combat(combat: Dictionary) -> void:
    _combat.start(
        combat.get("actions", []),
        combat.get("ownBoard", []),
        combat.get("opponentBoard", [])
    )
    var deadline := _unix_from_rfc3339(GameStore.deadline)
    if GameStore.phase == Protocol.PHASE_COMBAT and deadline > 0.0:
        var remaining_ms := maxf(deadline - Time.get_unix_time_from_system(), 0.0) * 1000.0
        _combat.seek(maxf(float(_combat.total_ms()) - remaining_ms, 0.0))
    _combat_turn = GameStore.turn


## 演示进行中：阶段为战斗、播放器属于本回合、有本回合战报且尚未播完。
func _combat_playing() -> bool:
    return (
        GameStore.phase == Protocol.PHASE_COMBAT
        and _combat_turn == GameStore.turn
        and GameStore.combat_is_current()
        and not _combat.is_finished()
    )


## 按播放器当前状态重画两行：已存在的节点只改数值与明暗，召唤的随从插入新卡。
func _render_combat_rows() -> void:
    _reconcile_row(_combat_opponent_row, _combat.opponent)
    _reconcile_row(_combat_own_row, _combat.own)


## 把一行对齐到 `minions` 里的存活条目：按下标复用同实例的卡片，多余的同步释放。
##
## 播放器保留阵亡条目作为顺序锚点（与引擎一致），但行内只渲染存活随从：
## 阵亡卡立刻消失，复生后重新按存活序列的位置出现。
## 复用靠卡片上的 `minion_id` 元数据：不依赖外部映射表，重画多少次都幂等。
func _reconcile_row(row: HFlowContainer, minions: Array) -> void:
    var alive: Array = []
    for minion in minions:
        if bool(minion.get("alive", true)):
            alive.append(minion)

    var wanted: Array = []
    for minion in alive:
        wanted.append(str(minion.get("minionId", "")))

    # 先摘掉不再需要的卡片（阵亡或被服务端撤回的随从），以及非卡片的占位提示：
    # 战报未到时 `_refresh_combat` 会在行内放一条等待提示，战报到达后必须随之清掉。
    var existing: Dictionary = {}
    for child in row.get_children():
        if not (child is Control) or not child.has_meta("minion_id"):
            row.remove_child(child)
            child.queue_free()
            continue
        var minion_id := str(child.get_meta("minion_id"))
        if wanted.has(minion_id) and not existing.has(minion_id):
            existing[minion_id] = child
        else:
            row.remove_child(child)
            child.queue_free()

    for position in alive.size():
        var minion: Dictionary = alive[position]
        var minion_id := str(minion.get("minionId", ""))
        var panel: Control = existing.get(minion_id)
        if panel == null:
            panel = _minion_card(minion, {}, false, 0, true)
            panel.set_meta("minion_id", minion_id)
            row.add_child(panel)
        var stats = panel.find_child("Stats", true, false)
        if stats != null:
            stats.text = "%d / %d" % [int(minion.get("attack", 0)), int(minion.get("health", 0))]
        row.move_child(panel, position)


## 横幅：动作播完并经过 TAIL_MS 后才出现。
func _refresh_combat_banner() -> void:
    _combat_banner.visible = _combat.banner_visible()
    if not _combat_banner.visible:
        return
    _set_combat_banner(GameStore.last_combat)


func _set_combat_banner(combat: Dictionary) -> void:
    var own_board: Array = combat.get("ownBoard", [])
    var opponent_board: Array = combat.get("opponentBoard", [])
    var winner = combat.get("winnerId")
    var result := "平局"
    var variant := "accent"
    if winner != null:
        if str(winner) == GameStore.player_id:
            result = "胜利"
            variant = "success"
        else:
            result = "失败"
            variant = "danger"
    var mirror := "（镜像）" if bool(combat.get("mirror", false)) else ""
    _combat_banner.text = "%s%s · 伤害 %d · 我方 %d vs 对手 %d" % [
        result,
        mirror,
        int(combat.get("damage", 0)),
        _combat.own.size(),
        _combat.opponent.size(),
    ]
    _combat_banner.add_theme_color_override("font_color", UITheme.color("color-" + variant) if variant != "accent" else UITheme.color("color-accent"))


func _fill_combat_strip(container: HBoxContainer, player: Dictionary, fallback: String) -> void:
    UI.clear(container)
    var hero_id := _optional_text(player.get("heroCardId"))
    container.add_child(UI.avatar(hero_id, UITheme.metric("avatar-lg")))

    var lines := VBoxContainer.new()
    lines.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    lines.add_theme_constant_override("separation", 0)
    container.add_child(lines)

    var display := _optional_text(player.get("displayName"))
    var name_label := UI.label(display if display != "" else fallback, UITheme.font_size("font-title"), false)
    lines.add_child(name_label)

    var meta := UI.label("HP %d · %s · 随从 %d" % [
        int(player.get("hp", 0)),
        _era_text(int(player.get("era", 1))),
        int(player.get("boardCount", 0)),
    ], UITheme.font_size("font-caption"), false)
    meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    lines.add_child(meta)


func _opponent_of(view: Dictionary) -> Dictionary:
    var opponent_id := _optional_text(view.get("opponentPlayerId"))
    if opponent_id == "":
        return {}
    for player in view.get("players", []):
        if str(player.get("playerId", "")) == opponent_id:
            return player
    return {}


func _self_player(view: Dictionary) -> Dictionary:
    for player in view.get("players", []):
        if str(player.get("playerId", "")) == GameStore.player_id:
            return player
    return view.get("self", {})


func _refresh_log() -> void:
    var lines: Array = []
    if GameStore.combat_is_current():
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


func _refresh_connection() -> void:
    var disconnected := not Net.is_open() and not GameStore.view.is_empty()
    _conn_label.text = "未连接 · 正在重连" if disconnected else ""


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


# ── 随从详情 ──────────────────────────────────────────────────────────────

const DETAIL_ROWS := ["initial", "permanent", "turn", "aura", "damage"]
const DETAIL_ROW_TEXT := {
    "initial": "初始",
    "permanent": "永久",
    "turn": "回合",
    "aura": "光环",
    "damage": "受击",
}


## 棋盘随从卡统一挂详情：悬停进入/离开、鼠标左键按下/松开、触摸按下/松开。
## 卡内按钮与 spacer 必须是 STOP/IGNORE 混合，祖先悬停通知会被子控件截断，
## 因此真正的矩形命中判断放在 _input，这里只保留就近的进入/离开兜底。
func _attach_detail(card: Control, minion: Dictionary) -> void:
    card.set_meta("detail_minion", minion)
    _detail_cards.append(card)
    card.mouse_entered.connect(_show_detail.bind(minion, card))
    card.gui_input.connect(_on_card_gui_input.bind(minion, card))


func _on_card_gui_input(event: InputEvent, minion: Dictionary, card: Control) -> void:
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        if event.pressed:
            _show_detail(minion, card)
        else:
            _update_detail_hover(card.get_global_rect().position + event.position)
    elif event is InputEventScreenTouch:
        if event.pressed:
            _show_detail(minion, card)
        else:
            _update_detail_hover(card.get_global_rect().position + event.position)


## 跨子控件的真实命中判断：指针落在卡片矩形内即显示，移出才隐藏。
func _input(event: InputEvent) -> void:
    if _detail_layer == null:
        return
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
    if _overlay != null and _overlay.visible:
        _hide_detail()
        return
    var found: Control = null
    for card in _detail_cards:
        if is_instance_valid(card) and card.is_visible_in_tree() and card.get_global_rect().has_point(point):
            found = card
            break
    if found == null:
        if _hover_card != null:
            _hide_detail()
        return
    if found == _hover_card and _detail_panel != null and _detail_panel.visible:
        return
    _show_detail(found.get_meta("detail_minion", {}), found)


func _show_detail(minion: Dictionary, anchor: Control) -> void:
    if _detail_panel == null or anchor == null or not is_instance_valid(anchor):
        return
    # 同一张卡已在展示：只校正位置，避免每次按下/移入都重建整块面板。
    # `_refresh` 会先 `_hide_detail()`，因此数据更新仍会触发重建。
    if anchor == _hover_card and _detail_panel.visible:
        _place_detail(anchor.get_global_rect())
        return
    if not is_instance_valid(_hover_card) or _hover_card != anchor:
        _hover_card = anchor
    UI.clear(_detail_panel)
    var box := VBoxContainer.new()
    box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _detail_panel.add_child(box)

    var card_id := str(minion.get("cardId", ""))
    var name_label := UI.label(CardDB.display_name(card_id), UITheme.font_size("font-title"), false)
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    box.add_child(name_label)

    var stats: Dictionary = minion.get("stats", {})
    if typeof(stats) != TYPE_DICTIONARY or stats.is_empty():
        box.add_child(_detail_row("当前", "%d / %d" % [
            int(minion.get("attack", 0)),
            int(minion.get("health", 0)),
        ]))
    else:
        var attack: Dictionary = _stat_layers(stats.get("attack"))
        var health: Dictionary = _stat_layers(stats.get("health"))
        for layer in DETAIL_ROWS:
            var a := int(attack.get(layer, 0))
            var h := int(health.get(layer, 0))
            if layer == "damage" and a == 0 and h == 0:
                continue
            var label := str(DETAIL_ROW_TEXT[layer])
            if layer == "initial":
                box.add_child(_detail_row(label, "%d / %d" % [a, h]))
            elif layer == "damage":
                box.add_child(_detail_row(label, "%s / %s" % [_damage_text(a), _damage_text(h)]))
            else:
                box.add_child(_detail_row(label, "%s / %s" % [_signed(a), _signed(h)]))
        box.add_child(_detail_row("当前", "%d / %d" % [
            int(attack.get("current", 0)),
            int(health.get("current", 0)),
        ]))

    _detail_layer.visible = true
    _detail_panel.visible = true
    _place_detail(anchor.get_global_rect())


## 以卡片全局矩形右上角内侧为锚点，按视口尺寸夹紧面板位置。
func _place_detail(card_rect: Rect2) -> void:
    var viewport := get_viewport_rect().size
    var panel_size := _detail_panel.get_combined_minimum_size()
    var margin := UITheme.space("space-xs")
    var x := card_rect.end.x - panel_size.x
    var y := card_rect.position.y
    x = clampf(x, margin, maxf(margin, viewport.x - panel_size.x - margin))
    y = clampf(y, margin, maxf(margin, viewport.y - panel_size.y - margin))
    _detail_panel.position = Vector2(x, y)


func _hide_detail() -> void:
    if _detail_panel != null:
        _detail_panel.visible = false
    if _detail_layer != null:
        _detail_layer.visible = false
    _hover_card = null


func _stat_layers(value: Variant) -> Dictionary:
    if typeof(value) == TYPE_DICTIONARY:
        return value
    return {}


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


func _signed(value: int) -> String:
    return "+%d" % value if value >= 0 else str(value)


## 受击行以「损失量」呈现：层和高于当前量为负数，复生超过层和为正数。
func _damage_text(value: int) -> String:
    if value > 0:
        return "-%d" % value
    if value < 0:
        return "+%d" % -value
    return "0"


# ── 弹层 ──────────────────────────────────────────────────────────────────

func _refresh_overlay(self_state: Dictionary, recruited: bool) -> void:
    UI.clear(_overlay)
    var choices: Array = self_state.get("pendingChoices", [])
    var phase := str(GameStore.view.get("phase", GameStore.phase))
    # 待选只在招募阶段可操作：跨回合保留的发现/进化待选不能盖在战斗演示上。
    if _view_stale or _deadline_expired or recruited or phase != Protocol.PHASE_RECRUIT:
        _target_intent = {}
        _overlay.visible = false
        return
    var head: Dictionary = choices[0] if not choices.is_empty() else {}
    if str(head.get("kind", "")) == "target":
        # 服务端目标待选优先：丢弃英雄技能的客户端目标态，改为选随从结算战吼。
        _target_intent = {}
        _build_target_pending_overlay(head, self_state)
        return
    if not _target_intent.is_empty():
        _build_target_overlay(self_state)
    elif not choices.is_empty():
        _build_choice_overlay(choices[0])
    else:
        _overlay.visible = false


func _overlay_frame(title: String, parent: VBoxContainer) -> void:
    _overlay.visible = true
    _hide_detail()
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
    _sell_hint.text = "松手出售 +%d 金" % refund
    _sell_hint.visible = hovering


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


# ── 工具 ──────────────────────────────────────────────────────────────────

func _find_by_index(items: Array, index: int) -> Dictionary:
    for item in items:
        if int(item.get("index", -1)) == index:
            return item
    return {}


func _optional_text(value: Variant) -> String:
    if value == null:
        return ""
    return str(value)


## 统一时代文案：顶栏、积分榜、属性行与战斗条都显示具体时代名。
func _era_text(era: int) -> String:
    return "时代 %s" % CardDB.era_name(era)


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
