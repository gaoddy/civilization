extends Control
## 对局：按酒馆战棋结构呈现服务端 PlayerView 与 affordances，操作只发协议意图。
##
## 布局：顶部工具条 — 左栏积分榜 — 中央舞台（招募/战斗组合、英雄行、贴底手牌）。
## 状态文本（断线、未发出、错误）写在顶部工具条左侧。

## 每次 `_send_intent` 先发一条，供冒烟与调试断言意图内容。
signal intent_sent(intent: Dictionary)

## 服务端演示节奏：单条战斗动作的播放时长（与 room-service 的 PACE_MS 一致）。
const COMBAT_PACE_MS := 320

## 服务端演示节奏：结果横幅停留（与 room-service 的 TAIL_MS 一致）。
const COMBAT_TAIL_MS := 2000

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

# 顶部工具条：状态文本与图鉴 / 提示 / 退出对局入口。
var _top_bar: GameTopBar
## 状态文本当前承载的是连接提示：恢复连接时据此清空，避免误清错误文案。
var _connection_notice := false

# 左栏积分榜。
var _players_panel: GamePlayersRail

# 中央舞台：招募组合、战斗组合、英雄行与贴底手牌。
var _center: VBoxContainer
var _recruit_view: GameRecruitView
var _combat_view: GameCombatView
var _hero_row: GameHeroRow
var _hand_row: GameHandRow

var _overlay: Control
# 左栏战报浮层：位于详情层之上、选择浮层之下，允许覆盖中央舞台。
var _report_layer: Control

# 随从详情：覆盖全屏但不拦截输入，层级位于内容与战报之上、选择浮层之下。
var _detail: GameDetailLayer

# 技能瞄准：弧线与候选标记，位于战报之上、选择浮层之下；本身不拦截输入。
var _aim: GameAimLayer

# 本局卡牌图鉴覆盖层：打开时盖住对局但不切换场景，关闭后对局状态原样保留。
var _gallery: CardGallery

# 首局引导：非阻塞覆盖层，步骤随阶段与本人视图变化。
var _guide: FirstGameGuide


func _ready() -> void:
    _build()
    GameStore.view_changed.connect(_refresh)
    GameStore.error_received.connect(_on_error)
    GameStore.game_over.connect(_on_game_over)
    Net.connected.connect(_refresh_connection)
    Net.disconnected.connect(_refresh_connection)
    _refresh()
    _guide.start()
    _update_timer()
    UI.safe_area(self)


# ── 构建 ──────────────────────────────────────────────────────────────────

func _build() -> void:
    var background := ColorRect.new()
    background.color = UITheme.color("color-bg")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(background)

    # iPhone 基准 960×540：页边距、页面间距与舞台间距整体收紧，结构与桌面一致。
    var compact := ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    margin.add_theme_constant_override(
        "margin_left", UITheme.space("space-sm") if compact else UITheme.space("space-xl")
    )
    margin.add_theme_constant_override(
        "margin_right", UITheme.space("space-sm") if compact else UITheme.space("space-xl")
    )
    margin.add_theme_constant_override(
        "margin_top", UITheme.space("space-sm") if compact else UITheme.space("space-lg")
    )
    margin.add_theme_constant_override(
        "margin_bottom", UITheme.space("space-xs") if compact else UITheme.space("space-md")
    )
    add_child(margin)

    var root := VBoxContainer.new()
    root.add_theme_constant_override(
        "separation", UITheme.space("space-xs") if compact else UITheme.space("space-md")
    )
    margin.add_child(root)

    _top_bar = GameTopBar.new()
    _top_bar.gallery_pressed.connect(_open_gallery)
    _top_bar.hint_pressed.connect(_on_hint_pressed)
    _top_bar.leave_game_pressed.connect(_on_leave_game)
    root.add_child(_top_bar)

    var body := HBoxContainer.new()
    body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override(
        "separation", UITheme.space("space-sm") if compact else UITheme.space("space-lg")
    )
    root.add_child(body)

    _players_panel = GamePlayersRail.new()
    body.add_child(_players_panel)
    _center = _build_center()
    body.add_child(_center)

    _detail = GameDetailLayer.new()
    add_child(_detail)
    _players_panel.detail = _detail
    _recruit_view.set_detail(_detail)
    _combat_view.set_detail(_detail)
    _hero_row.detail = _detail

    _report_layer = Control.new()
    _report_layer.name = "ReportLayer"
    _report_layer.set_anchors_preset(Control.PRESET_FULL_RECT)
    _report_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(_report_layer)
    _players_panel.report_layer = _report_layer

    _aim = GameAimLayer.new()
    _aim.set_anchors_preset(Control.PRESET_FULL_RECT)
    _aim.target_chosen.connect(_on_aim_target_chosen)
    _aim.target_abandoned.connect(_on_aim_target_abandoned)
    _aim.aim_cancelled.connect(_on_aim_cancelled)
    add_child(_aim)

    _overlay = Control.new()
    _overlay.name = "ChooserOverlay"
    _overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
    _overlay.visible = false
    add_child(_overlay)

    _guide = FirstGameGuide.new()
    _guide.target_root = self
    add_child(_guide)


## 中央舞台：招募组合 → 战斗组合 → 英雄行 → 贴底手牌，自上而下固定顺序。
func _build_center() -> VBoxContainer:
    var center := VBoxContainer.new()
    center.name = "CenterPanel"
    center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    center.size_flags_vertical = Control.SIZE_EXPAND_FILL
    center.add_theme_constant_override(
        "separation",
        UITheme.space("space-xs") if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
        else UITheme.space("space-md")
    )

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

    # 只有中央舞台自身撑满：组合与英雄行按固定高度自上而下排列，
    # 手牌吸收富余高度并保持卡片贴底。
    for view in [_recruit_view, _combat_view, _hero_row, _hand_row]:
        (view as Control).size_flags_vertical = Control.SIZE_SHRINK_BEGIN
    _hand_row.size_flags_vertical = Control.SIZE_EXPAND_FILL
    return center


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
        _top_bar.set_status(tr("ui.game.status.waiting_view"), UITheme.color("color-text-dim"))
        _guide.set_phase("", {})
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
    if _top_bar == null:
        return
    _top_bar.set_connected(Net.is_open())
    var disconnected := not Net.is_open() and not GameStore.view.is_empty()
    if disconnected:
        # 有连接句柄但未就绪 = 正在重连（dim）；句柄为空 = 断线（danger）。
        var reconnecting := Net.socket != null
        _top_bar.set_status(
            tr("ui.game.status.reconnecting") if reconnecting else tr("ui.game.status.disconnected"),
            UITheme.color("color-text-dim") if reconnecting else UITheme.color("color-danger"),
        )
        _connection_notice = true
        return
    if _connection_notice:
        _top_bar.set_status("", UITheme.color("color-text-dim"))
        _connection_notice = false


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

## 打开本局卡牌图鉴：只列服务端 `enabledFactions` 对应的卡池，不含 Token。
## 不切场景，关闭后保留拖拽、待选与战斗演示的全部状态。
func _open_gallery() -> void:
    if _overlay.visible:
        return
    if _gallery == null:
        _gallery = CardGallery.new()
        _gallery.name = "GalleryOverlay"
        _gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
        _gallery.closed.connect(_close_gallery)
        UI.safe_layer(self).add_child(_gallery)
    _gallery.set_catalog(
        CardPool.card_ids(GameStore.view.get("enabledFactions", [])),
        tr("ui.gallery.title_match"),
        tr("ui.gallery.back_match"),
        tr("ui.gallery.match_pool"),
    )
    _gallery.visible = true


func _close_gallery() -> void:
    if _gallery != null:
        _gallery.visible = false


func _refresh_overlay(self_state: Dictionary, recruited: bool) -> void:
    UI.clear(_overlay)
    var choices: Array = self_state.get("pendingChoices", [])
    var phase := str(GameStore.view.get("phase", GameStore.phase))
    # 待选只在招募阶段可操作：跨回合保留的发现/进化待选不能盖在战斗演示上。
    if _view_stale or _deadline_expired or recruited or phase != Protocol.PHASE_RECRUIT:
        _overlay.visible = false
    elif not choices.is_empty() and str(choices[0].get("kind", "")) != "target":
        _build_choice_overlay(choices[0])
    else:
        # 目标待选由瞄准层承接，不再构建候选弹层。
        _overlay.visible = false
    # 浮层可见性只在本函数收口；打开浮层时同步遮住详情层与图鉴。
    _detail.blocked = _overlay.visible
    if _overlay.visible:
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
        box.add_child(UI.label(tr("ui.game.overlay.discover_hint"), UITheme.font_size("font-body")))
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
        box.add_child(UI.label(tr("ui.game.overlay.evolve_hint"), UITheme.font_size("font-body")))
        var index := 0
        for option in choice.get("options", []):
            var button := UI.styled_button(UI.enum_text("evolve", str(option)), "slot", 44)
            button.pressed.connect(_on_choose.bind(index))
            box.add_child(button)
            index += 1
    _overlay_frame(tr("ui.game.overlay.title"), box)


# ── 意图 ──────────────────────────────────────────────────────────────────

func _send_intent(intent: Dictionary) -> String:
    intent_sent.emit(intent)
    _intent_counter += 1
    var intent_id := Net.send(Protocol.MSG_INTENT, {
        "clientIntentId": "ui-%d-%d" % [Time.get_ticks_usec(), _intent_counter],
        "intent": intent,
    })
    if intent_id == "":
        _top_bar.set_status(tr("ui.game.status.intent_not_sent"), UITheme.color("color-text-dim"))
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
    var frozen := false
    for slot in GameStore.view.get("self", {}).get("shop", []):
        if bool(slot.get("frozen", false)):
            frozen = true
            break
    _send_intent({
        "kind": Protocol.INTENT_UNFREEZE_SHOP if frozen else Protocol.INTENT_FREEZE_SHOP,
        "slot": null,
    })


func _on_hero_power() -> void:
    var power: Dictionary = GameStore.view.get("self", {}).get("heroPower", {})
    var skill_id := str(power.get("skillId", ""))
    if skill_id == "":
        return
    # 需要目标的技能由按下事件进入瞄准，抬起由瞄准层接管，不走 `pressed`。
    if bool(power.get("requiresTarget", false)):
        return
    _send_intent({"kind": Protocol.INTENT_USE_HERO_POWER, "skillId": skill_id})


## 技能按钮按下：需要目标时从按钮矩形起瞄准；候选为空说明技能不可用，不发意图。
func _on_hero_power_down() -> void:
    var power: Dictionary = GameStore.view.get("self", {}).get("heroPower", {})
    if not bool(power.get("requiresTarget", false)) or not _aim_submit_allowed():
        return
    var skill_id := str(power.get("skillId", ""))
    var candidates := Array(power.get("targetCandidates", []))
    if skill_id == "" or candidates.is_empty():
        return
    _aim_skill_intent = {"kind": Protocol.INTENT_USE_HERO_POWER, "skillId": skill_id}
    _detail.blocked = true
    _detail.hide_panel()
    _aim.begin_skill(_hero_row.power_rect(), candidates)


func _on_end_recruit() -> void:
    _send_intent({"kind": Protocol.INTENT_END_RECRUIT})


func _on_choose(choice_index: int) -> void:
    if _deadline_expired or str(GameStore.view.get("phase", GameStore.phase)) != Protocol.PHASE_RECRUIT:
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
    _top_bar.set_status(
        UI.error_text(code),
        UITheme.color("color-danger"),
        UI.error_tooltip(code),
    )
    match UI.error_route(code):
        "lobby":
            GameStore.reset_room()
            UI.navigate(self, "res://scenes/lobby.tscn")
        "login":
            GameStore.reset_session()
            UI.navigate(self, "res://scenes/login.tscn")


func _on_hint_pressed() -> void:
    _guide.show_manual()


## 对局内主动退出：先发 `LeaveRoom` 由服务端按弃权结算，再清房间并用令牌重连回大厅。
func _on_leave_game() -> void:
    Net.send(Protocol.MSG_LEAVE_ROOM, {})
    _guide.hide()
    GameStore.reset_room()
    Net.relogin()
    UI.navigate(self, "res://scenes/lobby.tscn")


func _on_game_over() -> void:
    _guide.hide()
    UI.navigate(self, "res://scenes/result.tscn")
