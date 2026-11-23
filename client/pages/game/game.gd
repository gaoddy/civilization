extends Control
## 对局：按酒馆战棋结构呈现服务端 PlayerView 与 affordances，操作只发协议意图。
##
## 布局：右上角悬浮工具组 — 左栏积分榜 — 中央舞台（招募/战斗组合、英雄行、贴底手牌）。
## 状态提示（断线、未发出、错误、等待视图）走居中模态框。

## 每次 `_send_intent` 先发一条，供冒烟与调试断言意图内容。
signal intent_sent(intent: Dictionary)

## 服务端演示节奏：单条战斗动作的播放时长（与 room-service 的 PACE_MS 一致）。
const COMBAT_PACE_MS := 320

## 服务端演示节奏：结果横幅停留（与 room-service 的 TAIL_MS 一致）。
const COMBAT_TAIL_MS := 2000

## 错误文案在状态模态中的停留时长。
const ERROR_NOTICE_SECONDS := 4.0

# 页面级状态与门槛。
var _intent_counter := 0
## 技能瞄准期间缓存的 `UseHeroPower` 意图；命中候选后补 `target` 再发出。
var _aim_skill_intent: Dictionary = {}
var _elapsed := 0.0
var _deadline_expired := false
var _view_stale := false
## 每次 `_refresh` 递增；拖拽数据带上它，落点回调据此判断拖拽是否已过期。
var _view_epoch := 0
## 队首待选是目标待选：该座位除选目标外不能操作，手牌不可拖。
var _target_pending := false

## 已回报「演示播完」的回合；同一回合只回报一次。
var _combat_ack_turn := -1

# 音效：按商店槽位实例对比冻结标记，事件触发的重抽不推断冻结。
var _shop_frozen: Dictionary = {}
var _suppress_frozen_cue := false
var _initial_shop_enter_done := false

# 右上角悬浮工具组：图鉴、提示与退出对局。
var _tools: GameToolButtons

# 状态模态：等待、断线、未发出与错误的唯一提示载体。
var _status_modal: GameStatusModal
## 模态当前来源：waiting / connection / error / intent / 空。
var _notice_kind := ""
## 已应用的连接状态：open / reconnecting / disconnected；只在变化时弹模态。
var _connection_key := ""
## 错误文案停留计时；到时收起并恢复仍有效的连接提示。
var _error_notice_left := 0.0
var _error_notice_phase := ""
var _error_notice_turn := -1

# 左栏积分榜。
var _players_panel: GamePlayersRail

# 中央舞台：招募组合、战斗组合、英雄行与贴底手牌。
var _center: VBoxContainer
var _recruit_view: GameRecruitView
var _combat_view: GameCombatView
var _hero_row: GameHeroRow
var _hand_row: GameHandRow

var _overlay: GameChoiceOverlay

# 战斗结束演出：双方英雄卡缓慢靠岸后猛击对冲；页面级覆盖层，位于详情层之下。
var _hero_clash: CombatHeroClash
## 待启动的对冲请求：等下一次 `_process` 完成布局后再取矩形。
var _pending_clash: Dictionary = {}

# 随从详情：覆盖全屏但不拦截输入，层级位于内容与战报之上、选择浮层之下。
var _detail: DetailLayer

# 技能瞄准：弧线与候选标记，位于战报之上、选择浮层之下；本身不拦截输入。
var _aim: GameAimLayer

# 本局卡牌图鉴覆盖层：打开时盖住对局但不切换场景，关闭后对局状态原样保留。
var _gallery: CardGallery

# 首局引导：由顶部「提示」打开的非阻塞覆盖层，步骤随阶段变化。
var _guide: FirstGameGuide


func _ready() -> void:
	UILayout.safe_area(self)
	_build()
	GameStore.view_changed.connect(_refresh)
	GameStore.events_received.connect(_on_game_events)
	GameStore.error_received.connect(_on_error)
	GameStore.game_over.connect(_on_game_over)
	Net.connected.connect(_refresh_connection)
	Net.disconnected.connect(_refresh_connection)
	_refresh()
	_update_timer()


# ── 构建 ──────────────────────────────────────────────────────────────────

func _build() -> void:
	# 对局页背景按分区构图：桌面/iPad 与 iPhone 各一张，坐标对齐真实控件矩形。
	var bg_path := "res://assets/ui/bg/bg_game_phone.png" if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE else "res://assets/ui/bg/bg_game_desktop.png"
	add_child(UIControls.background(bg_path))

	# iPhone 基准 960×540：页边距、页面间距与舞台间距整体收紧，结构与桌面一致。
	var compact := ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
	var page_side := UITheme.space("space-sm") if compact else UITheme.space("space-xl")
	var page_top := UITheme.space("space-sm") if compact else UITheme.space("space-lg")
	var page_bottom := UITheme.space("space-xs") if compact else UITheme.space("space-md")

	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	margin.add_theme_constant_override("margin_left", page_side)
	margin.add_theme_constant_override("margin_right", page_side)
	margin.add_theme_constant_override("margin_top", page_top)
	margin.add_theme_constant_override("margin_bottom", page_bottom)
	add_child(margin)

	var body := HBoxContainer.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override(
		"separation", UITheme.space("space-sm") if compact else UITheme.space("space-lg")
	)
	margin.add_child(body)

	_players_panel = GamePlayersRail.new()
	body.add_child(_players_panel)
	_center = _build_center()
	body.add_child(_center)

	_build_tools(page_side, page_top)

	# 对冲演出盖住中央舞台，但位于详情 / 瞄准 / 状态 / 选择层之下。
	_hero_clash = CombatHeroClash.new()
	_hero_clash.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_hero_clash)
	_combat_view.hero_clash_requested.connect(_on_hero_clash_requested)

	_detail = DetailLayer.new()
	add_child(_detail)
	_players_panel.detail = _detail
	_recruit_view.set_detail(_detail)
	_combat_view.set_detail(_detail)
	_hero_row.detail = _detail

	_aim = GameAimLayer.new()
	_aim.set_anchors_preset(Control.PRESET_FULL_RECT)
	_aim.target_chosen.connect(_on_aim_target_chosen)
	_aim.target_abandoned.connect(_on_aim_target_abandoned)
	_aim.aim_cancelled.connect(_on_aim_cancelled)
	add_child(_aim)

	# 状态模态压在选择浮层之下：发现 / 进化待选时提示不遮挡候选。
	_status_modal = GameStatusModal.new()
	_status_modal.acknowledged.connect(_on_notice_acknowledged)
	add_child(_status_modal)

	_overlay = GameChoiceOverlay.new()
	_overlay.chosen.connect(_on_choose)
	add_child(_overlay)

	_guide = FirstGameGuide.new()
	_guide.target_root = self
	add_child(_guide)


## 右上角悬浮工具组：与页面外边距对齐，只占按钮自身尺寸，不挤压中央舞台。
## 外层容器全部放行输入，只有三枚胶囊按钮接收点击。
func _build_tools(page_side: int, page_top: int) -> void:
	var frame := MarginContainer.new()
	frame.name = "GameToolsOverlay"
	frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
	frame.add_theme_constant_override("margin_right", page_side)
	frame.add_theme_constant_override("margin_top", page_top)
	add_child(frame)

	var column := VBoxContainer.new()
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	frame.add_child(column)

	_tools = GameToolButtons.new()
	_tools.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_tools.gallery_pressed.connect(_open_gallery)
	_tools.hint_pressed.connect(_on_hint_pressed)
	_tools.leave_game_pressed.connect(_on_leave_game)
	_tools.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_tools.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
	column.add_child(_tools)


## 中央舞台：招募组合 → 战斗组合 → 英雄行 → 贴底手牌，自上而下固定顺序。
func _build_center() -> VBoxContainer:
	var center := VBoxContainer.new()
	center.name = "CenterPanel"
	center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	center.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var compact := ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
	center.add_theme_constant_override(
		"separation",
		UITheme.space("space-xs") if compact else UITheme.space("space-md")
	)
	# 手机基准：整组贴底，富余高度落在舞台顶部，河道不再贴着页面顶边。
	center.alignment = BoxContainer.ALIGNMENT_END if compact else BoxContainer.ALIGNMENT_BEGIN

	_recruit_view = GameRecruitView.new()
	_recruit_view.reroll_pressed.connect(_on_reroll)
	_recruit_view.upgrade_pressed.connect(_on_upgrade)
	_recruit_view.freeze_pressed.connect(_on_freeze)
	_recruit_view.board_dropped.connect(_on_board_dropped)
	_recruit_view.sell_dropped.connect(_on_sell_dropped)
	_recruit_view.sell_hover.connect(_on_sell_hover)
	_recruit_view.end_recruit_pressed.connect(_on_end_recruit)
	center.add_child(_recruit_view)

	_combat_view = GameCombatView.new()
	center.add_child(_combat_view)

	_hero_row = GameHeroRow.new()
	_hero_row.hero_power_pressed.connect(_on_hero_power)
	_hero_row.hero_power_down.connect(_on_hero_power_down)
	center.add_child(_hero_row)

	_hand_row = GameHandRow.new()
	_hand_row.target_dropped.connect(_on_hand_target_dropped)
	_hand_row.shop_dropped.connect(_on_shop_dropped)
	center.add_child(_hand_row)

	# 只有中央舞台自身撑满：组合与英雄行按固定高度排列。
	# 桌面由手牌吸收富余高度并保持卡片贴底；手机手牌固定卡高、整组贴底，
	# 富余高度留在舞台顶部。
	for view in [_recruit_view, _combat_view, _hero_row, _hand_row]:
		(view as Control).size_flags_vertical = Control.SIZE_SHRINK_BEGIN
	_hand_row.size_flags_vertical = (
		Control.SIZE_SHRINK_END if compact else Control.SIZE_EXPAND_FILL
	)
	return center


# ── 刷新 ──────────────────────────────────────────────────────────────────

func _process(delta: float) -> void:
	_elapsed += delta
	_tick_error_notice(delta)
	if _elapsed >= 0.25:
		_elapsed = 0.0
		_update_timer()
	_start_pending_clash()
	if _hero_clash.playing():
		_hero_clash.advance(delta)
	if _combat_view.playing():
		_combat_view.advance(delta)
	_report_combat_played()


## 错误与未发出的生命周期：停留时长用完即收起；阶段或回合变化由 `_refresh` 提前收起错误。
func _tick_error_notice(delta: float) -> void:
	if _notice_kind != "error" and _notice_kind != "intent":
		return
	_error_notice_left -= delta
	if _error_notice_left <= 0.0:
		_clear_timed_notice()


func _clear_error_notice() -> void:
	if _notice_kind != "error":
		return
	_clear_timed_notice()


## 到时收起；仍断线时恢复连接提示，视图已清空时回到等待提示。
func _clear_timed_notice() -> void:
	_error_notice_left = 0.0
	if GameStore.view.is_empty():
		_show_notice("waiting", tr("ui.game.status.waiting_view"))
		return
	if not Net.is_open():
		_show_connection_notice()
		return
	_hide_notice()


## 状态模态的唯一写入口；来源只用于判断收起与恢复，不参与文案。
func _show_notice(kind: String, text: String, tone := "dim", detail := "") -> void:
	_notice_kind = kind
	_error_notice_left = ERROR_NOTICE_SECONDS if kind in ["error", "intent"] else 0.0
	_status_modal.show_status(text, tone, detail)


func _hide_notice() -> void:
	_notice_kind = ""
	_error_notice_left = 0.0
	_status_modal.hide_status()


func _refresh() -> void:
	_detail.reset()
	if (
		_notice_kind == "error"
		and (
			GameStore.phase != _error_notice_phase
			or GameStore.turn != _error_notice_turn
		)
	):
		_clear_error_notice()
	var view: Dictionary = GameStore.view
	if view.is_empty():
		# 等待视图是空视图下的主状态；错误与未发出到期前保留自己的提示。
		if _notice_kind != "error" and _notice_kind != "intent":
			_show_notice("waiting", tr("ui.game.status.waiting_view"))
		_guide.set_phase("", {})
		_shop_frozen.clear()
		_suppress_frozen_cue = false
		_refresh_connection()
		return
	if _notice_kind == "waiting":
		_hide_notice()

	var self_state: Dictionary = view.get("self", {})
	var recruited := bool(self_state.get("recruited", false))
	var phase := str(view.get("phase", GameStore.phase))
	_view_stale = int(view.get("turn", 0)) != GameStore.turn
	_view_epoch += 1
	var pending: Array = self_state.get("pendingChoices", [])
	_target_pending = not pending.is_empty() and str(pending[0].get("kind", "")) == "target"
	var state := _build_state(view, self_state, phase, recruited)
	_track_shop_frozen(state)
	_play_initial_shop_enter()

	_players_panel.refresh(state)

	var in_combat := phase == Protocol.PHASE_COMBAT or phase == Protocol.PHASE_SETTLE
	_set_combat_mode(in_combat)
	if in_combat:
		# 招募组合隐藏后仍刷新河道：页面唯一的计时源要跟随当前阶段。
		_combat_view.refresh(state)
		_recruit_view.refresh_clock(state)
	else:
		_combat_view.reset()
		_combat_ack_turn = -1
		_recruit_view.refresh(state)
	_hero_row.refresh(state)
	_hand_row.refresh(state)

	_refresh_overlay(self_state, recruited)
	_refresh_aim(self_state, phase, recruited)
	_refresh_connection()
	_guide.set_phase(phase, self_state)


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


# ── 音效 ──────────────────────────────────────────────────────────────────

## 首回合的商店入场：`GameStarted` 不带事件，页面首次拿到招募视图时补播一次。
func _play_initial_shop_enter() -> void:
	if _initial_shop_enter_done:
		return
	if GameStore.turn != 1 or GameStore.phase != Protocol.PHASE_RECRUIT:
		return
	if GameStore.view.is_empty():
		return
	_initial_shop_enter_done = true
	Sfx.play_cue(Sfx.CUE_SHOP_ENTER)


## 事件到音效：一封 `StateUpdate` 只处理一次，不按视图差异重复推断。
##
## 回合开始的自动重抽与 `TurnStarted` 同一批到达，只播商店入场音效；
## 其余 `ShopRolled` 视为刷新（手动、首次免费、效果与升时代重抽）。
func _on_game_events(events: Array) -> void:
	var turn_started := false
	var rolled := false
	for entry in events:
		if not (entry is Dictionary):
			continue
		var kind := str(entry.get("kind", ""))
		if kind == Protocol.EVENT_TURN_STARTED:
			turn_started = true
		elif kind == Protocol.EVENT_SHOP_ROLLED:
			rolled = true
	# 重抽会整批换槽位，冻结差异不能当成玩家冻结/解冻。
	if turn_started or rolled:
		_suppress_frozen_cue = true
	for entry in events:
		if not (entry is Dictionary):
			continue
		if str(entry.get("kind", "")) == Protocol.EVENT_MINION_BOUGHT:
			Sfx.play_card(str(entry.get("cardId", "")), Sfx.CUE_CARD_BUY)
	if turn_started:
		Sfx.play_cue(Sfx.CUE_SHOP_ENTER)
	elif rolled:
		Sfx.play_cue(Sfx.CUE_REROLL)


## 冻结/解冻以服务端视图的 `frozen` 变化为准：同一槽位 false→true 或
## true→false 才播；新建/移除槽位、实例编号变化、重抽与进入新回合都不算。
func _track_shop_frozen(state: GameUiState) -> void:
	var frozen_now: Dictionary = {}
	for slot in state.self_state.get("shop", []):
		if not (slot is Dictionary) or slot.get("cardId") == null:
			continue
		var card_id := str(slot.get("cardId", ""))
		var instance_id := str(slot.get("instanceId", ""))
		var key := (
			instance_id
			if instance_id != ""
			else "%d:%s" % [int(slot.get("index", -1)), card_id]
		)
		frozen_now[key] = bool(slot.get("frozen", false))
	var froze := false
	var unfroze := false
	for key in frozen_now:
		if not _shop_frozen.has(key):
			continue
		var was := bool(_shop_frozen[key])
		var now := bool(frozen_now[key])
		if now and not was:
			froze = true
		elif was and not now:
			unfroze = true
	_shop_frozen = frozen_now
	if _suppress_frozen_cue:
		_suppress_frozen_cue = false
		return
	if state.phase != Protocol.PHASE_RECRUIT:
		return
	if froze:
		Sfx.play_cue(Sfx.CUE_FREEZE)
	if unfroze:
		Sfx.play_cue(Sfx.CUE_UNFREEZE)


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


func _set_combat_mode(in_combat: bool) -> void:
	_recruit_view.visible = not in_combat
	_combat_view.visible = in_combat
	_hero_row.set_combat_mode(in_combat)
	if not in_combat:
		_pending_clash = {}
		_hero_clash.clear()


## 战斗视图请求结束演出：先记下，等下一次 `_process` 布局完成后再启动。
func _on_hero_clash_requested(winner_side: String, damage: int) -> void:
	_pending_clash = {"winner": winner_side, "damage": damage}


## 启动待处理的对冲：首帧 / 重连时布局可能尚未完成，矩形为零时保留请求、下一帧再试。
func _start_pending_clash() -> void:
	if _pending_clash.is_empty():
		return
	var pending := _pending_clash
	_pending_clash = {}
	var own_card := _hero_row.hero_card()
	var opponent_card := _combat_view.opponent_hero_card()
	if own_card == null or opponent_card == null:
		return
	var river := _combat_view.river_rect()
	if river.size.x <= 0.0 or river.size.y <= 0.0:
		_pending_clash = pending
		return
	if _detail != null:
		_detail.hide_panel()
	_hero_clash.start(
		own_card,
		opponent_card,
		river,
		str(pending.get("winner", "")),
		int(pending.get("damage", 0)),
	)


## 演示播完后向服务端回报一次；断线时不发送也不重试（服务端把断线视为已回报）。
## 对冲演出期间不回报：服务端推进到招募会清空战斗视图，演出会被截断。
func _report_combat_played() -> void:
	if GameStore.phase != Protocol.PHASE_COMBAT:
		return
	if not GameStore.combat_is_current() or not _combat_view.finished():
		return
	if _hero_clash.playing() or not _pending_clash.is_empty():
		return
	if _combat_ack_turn == GameStore.turn:
		return
	_combat_ack_turn = GameStore.turn
	if not Net.is_open():
		return
	Net.send(Protocol.MSG_COMBAT_PLAYED, {})


func _refresh_connection() -> void:
	if _tools == null or _status_modal == null:
		return
	_tools.set_connected(Net.is_open())
	# 空视图时先由「等待对局状态」承载，收到视图后再显示连接提示。
	var key := "open"
	if not Net.is_open() and not GameStore.view.is_empty():
		# 有连接句柄但未就绪 = 正在重连；句柄为空 = 断线。
		key = "reconnecting" if Net.socket != null else "disconnected"
	if key == _connection_key:
		return
	_connection_key = key
	if key == "open":
		if _notice_kind == "connection":
			_hide_notice()
		return
	_show_connection_notice()


## 连接提示只在连接状态变化时弹出；手动关闭后同一状态不重复弹出。
func _show_connection_notice() -> void:
	var reconnecting := Net.socket != null
	_show_notice(
		"connection",
		tr("ui.game.status.reconnecting") if reconnecting else tr("ui.game.status.disconnected"),
		"dim" if reconnecting else "danger",
	)


## 倒计时由招募河道维护；页面按「是否存在截止时间」决定清零，避免把已过期
## （秒数为 0）误判成没有截止时间。
func _update_timer() -> void:
	if _recruit_view == null:
		return
	if not _recruit_view.has_deadline():
		_set_deadline_expired(false)
		return
	_set_deadline_expired(_recruit_view.seconds_left() <= 0)


func _set_deadline_expired(value: bool) -> void:
	if _deadline_expired == value:
		return
	_deadline_expired = value
	_refresh()


# ── 弹层 ──────────────────────────────────────────────────────────────────

## 打开本局卡牌图鉴：只列服务端 `enabledFactions` 对应的卡池，不含非池卡。
## 不切场景，关闭后保留拖拽、待选与战斗演示的全部状态。
func _open_gallery() -> void:
	if _overlay.visible:
		return
	if _gallery == null:
		_gallery = CardGallery.new()
		_gallery.name = "GalleryOverlay"
		_gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
		_gallery.closed.connect(_close_gallery)
		UILayout.safe_layer(self).add_child(_gallery)
	var enabled: Array = GameStore.view.get("enabledFactions", [])
	_gallery.set_catalog(
		CardPool.card_ids(enabled),
		tr("ui.gallery.back_match"),
		CardPool.hero_ids(enabled),
	)
	_gallery.visible = true


func _close_gallery() -> void:
	if _gallery != null:
		_gallery.visible = false


func _refresh_overlay(self_state: Dictionary, recruited: bool) -> void:
	var choices: Array = self_state.get("pendingChoices", [])
	var phase := str(GameStore.view.get("phase", GameStore.phase))
	# 待选只在招募阶段可操作：跨回合保留的发现/进化待选不能盖在战斗演示上。
	if _view_stale or _deadline_expired or recruited or phase != Protocol.PHASE_RECRUIT:
		_overlay.hide_choice()
	elif not choices.is_empty() and str(choices[0].get("kind", "")) != "target":
		_overlay.show_choice(choices[0])
	else:
		# 目标待选由瞄准层承接，不再构建候选弹层。
		_overlay.hide_choice()
	# 浮层可见性只在本函数收口；打开浮层时同步遮住详情层与图鉴。
	_detail.blocked = _overlay.visible
	if _overlay.visible:
		_detail.hide_panel()
		_close_gallery()


## 瞄准层的待选状态：视图过期、倒计时归零、已结束招募、非招募阶段或离线时关闭；
## 否则以服务端下发的目标待选为准，选目标与放弃都直接发意图，不等本地确认。
func _refresh_aim(self_state: Dictionary, phase: String, recruited: bool) -> void:
	var pending: Array = self_state.get("pendingChoices", [])
	var head: Dictionary = pending[0] if not pending.is_empty() else {}
	var blocked := (
		_view_stale
		or _deadline_expired
		or recruited
		or phase != Protocol.PHASE_RECRUIT
		or not Net.is_open()
	)
	if not blocked and str(head.get("kind", "")) == "target":
		_close_gallery()
		_aim_skill_intent = {}
		_aim.begin_pending(
			str(head.get("sourceMinionId", "")), Array(head.get("candidates", []))
		)
	elif _aim.skill_active():
		if blocked:
			_aim_skill_intent = {}
			_aim.clear()
	else:
		_aim.clear()
	_detail.blocked = _overlay.visible or _aim.aiming()


# ── 意图 ──────────────────────────────────────────────────────────────────

func _send_intent(intent: Dictionary) -> String:
	intent_sent.emit(intent)
	_intent_counter += 1
	var intent_id := Net.send(Protocol.MSG_INTENT, {
		"clientIntentId": "ui-%d-%d" % [Time.get_ticks_usec(), _intent_counter],
		"intent": intent,
	})
	if intent_id == "":
		_show_notice("intent", tr("ui.game.status.intent_not_sent"))
	return intent_id



## 商店卡落在手牌带上：买牌只入手牌，不涉及目标；落点复查与其它拖拽同源。
func _on_shop_dropped(data: Dictionary) -> void:
	if int(data.get("epoch", -1)) != _view_epoch:
		return
	if not _can_play_now():
		return
	_send_intent({"kind": Protocol.INTENT_BUY_MINION, "slot": int(data.get("shopSlot", 0))})


## 棋盘落点回调：随从牌入场与定点效果牌的唯一入口。拖拽途中视图已重建时按取消处理。
##
## 拖拽期间状态可能已经变化（视图重建、换回合、开战、断线、进入目标待选），
## 落点必须重新过一遍与拖拽起始同一组门控，任一不满足都不构造意图。
func _on_board_dropped(data: Dictionary, position: int, target: Dictionary) -> void:
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
	_send_play_card(data, position, target)


## 手牌落点回调：定点效果牌拖到另一张手牌候选上松手时提交。
func _on_hand_target_dropped(data: Dictionary, target: Dictionary) -> void:
	if int(data.get("epoch", -1)) != _view_epoch:
		return
	if not _can_play_now():
		return
	_send_play_card(data, 0, target)


## 拼出 `PlayCard`：随从牌带插入下标；需要目标的效果牌带 `target`，缺目标时不下发。
func _send_play_card(data: Dictionary, position: int, target: Dictionary) -> void:
	var card_kind := str(data.get("cardKind", ""))
	var requires_target := bool(data.get("requiresTarget", false))
	var intent := {"kind": Protocol.INTENT_PLAY_CARD, "handIndex": int(data.get("handIndex", 0))}
	if card_kind == "minion":
		intent["position"] = position
	elif requires_target:
		if target.is_empty():
			return
		intent["target"] = target
	_send_intent(intent)


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
	return not _self_eliminated() and drop_gate(
		str(GameStore.view.get("phase", GameStore.phase)),
		_view_stale,
		_deadline_expired,
		Net.is_open(),
		_target_pending,
		bool(GameStore.view.get("self", {}).get("recruited", false))
	)


## 点击类操作的服务端语义门控：非招募阶段、旧视图、倒计时归零、待选、已结束招募
## 与出局座位都会被服务端拒绝，这里先挡下，避免产生必然失败的意图。
## 不含连接状态：断线时仍交给 `_send_intent` 提示「未发出」。
func _click_allowed() -> bool:
	return (
		str(GameStore.view.get("phase", GameStore.phase)) == Protocol.PHASE_RECRUIT
		and not _view_stale
		and not _deadline_expired
		and not _target_pending
		and not bool(GameStore.view.get("self", {}).get("recruited", false))
		and not _self_eliminated()
	)


## 本人是否已出局：出局座位在招募阶段也只能观战，服务端会拒绝一切招募意图。
func _self_eliminated() -> bool:
	return GameUiState.view_eliminated(GameStore.view, GameStore.player_id)


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
	if not _click_allowed():
		return
	_send_intent({"kind": Protocol.INTENT_REROLL_SHOP})


func _on_upgrade() -> void:
	if not _click_allowed():
		return
	_send_intent({"kind": Protocol.INTENT_UPGRADE_ERA})


func _on_freeze() -> void:
	if not _click_allowed():
		return
	# 解冻只在冻结的当回合可用；跨回合保留的冻结仍走「冻结」按钮。
	var aff: Dictionary = GameStore.view.get("self", {}).get("affordances", {})
	_send_intent({
		"kind": (
			Protocol.INTENT_UNFREEZE_SHOP
			if bool(aff.get("canUnfreeze", false))
			else Protocol.INTENT_FREEZE_SHOP
		),
		"slot": null,
	})


## 按技能 id 取本座位视图里的主动技能条目；不存在时为空字典。
func _hero_power_of(skill_id: String) -> Dictionary:
	for power in GameStore.view.get("self", {}).get("heroPowers", []):
		if power is Dictionary and str(power.get("skillId", "")) == skill_id:
			return power
	return {}


func _on_hero_power(skill_id: String) -> void:
	if not _click_allowed():
		return
	var power := _hero_power_of(skill_id)
	if power.is_empty():
		return
	# 需要目标的技能由按下事件进入瞄准，抬起由瞄准层接管，不走 `pressed`。
	if bool(power.get("requiresTarget", false)):
		return
	_send_intent({"kind": Protocol.INTENT_USE_HERO_POWER, "skillId": skill_id})


## 技能按钮按下：需要目标时从按钮矩形起瞄准；候选为空说明技能不可用，不发意图。
func _on_hero_power_down(skill_id: String) -> void:
	var power := _hero_power_of(skill_id)
	if power.is_empty() or not bool(power.get("requiresTarget", false)):
		return
	if not _aim_submit_allowed():
		return
	var candidates := Array(power.get("targetCandidates", []))
	if candidates.is_empty():
		return
	_aim_skill_intent = {"kind": Protocol.INTENT_USE_HERO_POWER, "skillId": skill_id}
	_detail.blocked = true
	_detail.hide_panel()
	_aim.begin_skill(_hero_row.power_rect(skill_id), candidates)


func _on_end_recruit() -> void:
	if not _click_allowed():
		return
	_send_intent({"kind": Protocol.INTENT_END_RECRUIT})


func _on_choose(choice_index: int) -> void:
	if not _click_allowed():
		return
	_send_intent({"kind": Protocol.INTENT_CHOOSE_DISCOVER, "choice": choice_index})


## 瞄准提交：技能模式补 `target` 发 `UseHeroPower`；待选模式发 `ChooseTarget`。
func _on_aim_target_chosen(target: Dictionary) -> void:
	if not _aim_submit_allowed():
		return
	if not _aim_skill_intent.is_empty():
		var intent := _aim_skill_intent.duplicate()
		_aim_skill_intent = {}
		intent["target"] = target
		_send_intent(intent)
		_detail.blocked = _overlay.visible
		return
	_send_intent({"kind": Protocol.INTENT_CHOOSE_TARGET, "target": target})
	_detail.blocked = _overlay.visible


## 战吼待选放弃：服务端撤销本次入场，原牌退回原手牌位置。
func _on_aim_target_abandoned() -> void:
	if not _aim_submit_allowed():
		return
	_send_intent({"kind": Protocol.INTENT_ABANDON_TARGET})
	_detail.blocked = _overlay.visible


## 技能瞄准取消：只清本地待发意图，不发 `AbandonTarget`。
func _on_aim_cancelled() -> void:
	_aim_skill_intent = {}
	_detail.blocked = _overlay.visible


## 选目标与放弃的前提：招募阶段、视图新鲜、倒计时未归零、连接可用。
func _aim_submit_allowed() -> bool:
	return (
		str(GameStore.view.get("phase", GameStore.phase)) == Protocol.PHASE_RECRUIT
		and not _view_stale
		and not _deadline_expired
		and Net.is_open()
	)


# ── 错误与结算 ────────────────────────────────────────────────────────────

func _on_error() -> void:
	var code := str(GameStore.last_error.get("code", ""))
	var text := UIWording.error_text(code)
	var detail := UIWording.error_tooltip(code)
	_show_notice("error", text, "danger", detail if detail != text else "")
	# 一次拒绝只提示一段时间；阶段/回合变化后的下一次刷新会提前收起。
	_error_notice_phase = GameStore.phase
	_error_notice_turn = GameStore.turn
	match UINavigation.error_route(code):
		UINavigation.ROUTE_LOBBY:
			GameStore.reset_room()
			UINavigation.navigate(self, UINavigation.PAGE_LOBBY)
		UINavigation.ROUTE_LOGIN:
			GameStore.reset_session()
			UINavigation.navigate(self, UINavigation.PAGE_LOGIN)


func _on_hint_pressed() -> void:
	_guide.show_manual()


## 「知道了」：清空当前来源并取消计时；断线时手动关掉错误或未发出后恢复连接提示。
func _on_notice_acknowledged() -> void:
	var kind := _notice_kind
	_hide_notice()
	if kind != "connection" and not Net.is_open() and not GameStore.view.is_empty():
		_show_connection_notice()


## 对局内主动退出：先发 `LeaveRoom` 由服务端按弃权结算，再清房间并用令牌重连回大厅。
func _on_leave_game() -> void:
	Net.send(Protocol.MSG_LEAVE_ROOM, {})
	_guide.hide()
	GameStore.reset_room()
	Net.relogin()
	UINavigation.navigate(self, UINavigation.PAGE_LOBBY)


func _on_game_over() -> void:
	_guide.hide()
	UINavigation.navigate(self, UINavigation.PAGE_RESULT)


## 预览入口：返回对局页实例；状态由 `make ui-preview` 在调用前注入 GameStore。
static func preview(_opts: Dictionary = {}) -> Node:
	return (load("res://pages/game/game.tscn") as PackedScene).instantiate()
