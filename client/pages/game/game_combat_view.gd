class_name GameCombatView
extends PanelContainer
## 战斗组合：高 438 = 对手英雄行 84 + 6 + 对手行 146 + 6 + 河道 44 + 6 + 我方行 146。
##
## 演示进度由 `CombatPlayback` 纯数据播放器驱动；页面只负责在演示中调用
## `advance()`，本视图不发送任何意图。演示播完后发出一次英雄对冲请求，
## 河道只保留回合数，不再追加胜负文字。

## 演示播完、进入结果停留时发出：胜方 `own` / `opponent` / 空串（平局）与伤害。
signal hero_clash_requested(winner_side: String, damage: int)

## 卡片详情登记入口，由对局页注入；独立实例化时可为 null。
var detail: DetailLayer = null

var _state: GameUiState = null
var _opponent_view: GameOpponentView
var _river: GameRiver
var _opponent_row: HFlowContainer
var _own_row: HFlowContainer
## 演示播放器（纯数据）与当前演示所属回合。
var _combat := CombatPlayback.new()
var _combat_turn := -1
## 本场结束音效与对冲请求是否已发出；每场战斗重置，避免视图刷新时重复。
var _clash_announced := false
## 覆盖层演出：克隆卡碰撞往返、闪光、飘字与淡出。
var _fx: CombatFx
## 开战画面：双方对阵约 1.2 秒，显示期间暂停演示推进。
var _intro: CombatIntro


## 舞台内小间距：iPhone 基准下再收紧一档。
func _stage_gap() -> int:
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        return UITheme.space("space-xs")
    return UITheme.space("space-sm")


func _init() -> void:
    name = "CombatPanel"
    add_theme_stylebox_override("panel", UITheme.panel_style("bare"))
    custom_minimum_size = Vector2(
        0, UITheme.stage_metric("hero-card-size")
            + UITheme.stage_metric("minion-card-height") * 2
            + UITheme.stage_metric("river-height")
            + _stage_gap() * 3
    )
    size_flags_vertical = Control.SIZE_EXPAND_FILL
    visible = false

    var body := VBoxContainer.new()
    body.name = "CombatBody"
    body.add_theme_constant_override("separation", UITheme.space("space-none"))
    add_child(body)

    _opponent_view = GameOpponentView.new()
    body.add_child(_opponent_view)
    body.add_child(_gap("CombatOpponentGap", _stage_gap()))

    var opponent_center := CenterContainer.new()
    opponent_center.name = "CombatOpponentRowCenter"
    opponent_center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    body.add_child(opponent_center)
    _opponent_row = _new_flow_row("CombatOpponentRow")
    opponent_center.add_child(_opponent_row)

    body.add_child(_gap("CombatRiverGap", _stage_gap()))

    _river = GameRiver.new(GameRiver.Mode.COMBAT)
    body.add_child(_river)

    body.add_child(_gap("CombatOwnGap", _stage_gap()))

    var own_center := CenterContainer.new()
    own_center.name = "CombatOwnRowCenter"
    own_center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    body.add_child(own_center)
    _own_row = _new_flow_row("CombatOwnRow")
    own_center.add_child(_own_row)

    # 覆盖层最后加入，盖在阵容行与河道之上；不参与布局。
    _fx = CombatFx.new()
    _fx.name = "CombatFxLayer"
    _fx.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(_fx)

    # 开战画面最后加入，盖住阵容与碰撞演出；不参与布局、不拦截鼠标。
    _intro = CombatIntro.new()
    add_child(_intro)


func _gap(node_name: String, height: int) -> Control:
    var gap := Control.new()
    gap.name = node_name
    gap.custom_minimum_size = Vector2(0, height)
    gap.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return gap


func _new_flow_row(node_name: String) -> HFlowContainer:
    var row := HFlowContainer.new()
    row.name = node_name
    # 行高常驻：一方随从全灭时该行不塌陷，两行在演示全程保持位置。
    row.custom_minimum_size = Vector2(0, UITheme.stage_metric("minion-card-height"))
    row.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    row.add_theme_constant_override("h_separation", UITheme.space("space-md"))
    row.add_theme_constant_override("v_separation", UITheme.space("space-md"))
    return row


func set_detail(value: DetailLayer) -> void:
    detail = value
    _opponent_view.detail = value


func refresh(state: GameUiState) -> void:
    _state = state
    _opponent_view.refresh(state)
    _river.refresh(state)
    if not GameStore.combat_is_current():
        UIControls.clear(_opponent_row)
        UIControls.clear(_own_row)
        _opponent_row.custom_minimum_size.x = 0
        _own_row.custom_minimum_size.x = 0
        _opponent_row.add_child(UIControls.empty_hint(tr("ui.game.combat.waiting")))
        return
    var combat: Dictionary = GameStore.last_combat
    if _combat_turn != GameStore.turn:
        _start_combat(combat)
    _render_combat_rows()
    _refresh_combat_banner()


## 进入非战斗阶段时丢弃本场演示归属，下一场战斗从头播放。
func reset() -> void:
    _combat_turn = -1
    _clash_announced = false
    _intro.clear()
    _fx.clear()


## 演示进行中：阶段为战斗、播放器属于本回合、有本回合战报且尚未播完。
func playing() -> bool:
    return (
        GameStore.phase == Protocol.PHASE_COMBAT
        and _combat_turn == GameStore.turn
        and GameStore.combat_is_current()
        and not _combat.is_finished()
    )


## 推进演示；由页面在 `playing()` 为真时按帧调用。
##
## 先推进已有演出，再应用本帧到期动作并为其开启新演出：新克隆不会在同一帧
## 里被整段推进，飞行从原位开始。
func advance(delta: float) -> void:
    # 开战画面未结束：本帧只推进画面，动作与演出保持起点，避免首次攻击被遮挡。
    if _intro.advance(delta * 1000.0):
        return
    _fx.advance(delta)
    var events := _combat.advance(delta * 1000.0)
    _play_events(events)
    _render_combat_rows()
    _animate_new_cards(events)
    _refresh_combat_banner()


## 当前回合的演示已播完（含横幅停留）。
func finished() -> bool:
    return (
        _combat_turn == GameStore.turn
        and GameStore.combat_is_current()
        and _combat.is_finished()
    )


## 装载本场演示：战前阵容与动作序列，并按服务端 `startedAt` 对齐续播进度。
func _start_combat(combat: Dictionary) -> void:
    _fx.clear()
    _clash_announced = false
    var started_at := UIFormat.unix_from_rfc3339(str(combat.get("startedAt", "")))
    _combat.start(
        combat.get("actions", []),
        combat.get("ownBoard", []),
        combat.get("opponentBoard", []),
        started_at
    )
    var elapsed_ms := 0.0
    if started_at > 0.0:
        elapsed_ms = maxf(Time.get_unix_time_from_system() - started_at, 0.0) * 1000.0
    # 续播只对齐数据，不补演已经过去的动作。
    _combat.seek(elapsed_ms)
    _combat_turn = GameStore.turn
    # 首条动作尚未应用才算“开战”；续播已进入中段时不补开战音与开场画面。
    if elapsed_ms < CombatPlayback.PACE_MS:
        Sfx.play_cue(Sfx.CUE_COMBAT_START)
        _intro.present(_state)


## 按动作类型开启演出；数值已在播放器中按服务端快照落地。
##
## `attack` 只移动与反馈，不改存活；阵亡只由 `death` 触发，因此目标不会在
## 攻方到达前消失。
func _play_events(events: Array) -> void:
    for action in events:
        var data: Dictionary = action
        match str(data.get("kind", "")):
            "attack":
                var source_side := str(data.get("sourceSide", ""))
                var source_id := _minion_id(data.get("sourceMinionId"), data.get("source"))
                var target_side := str(data.get("targetSide", ""))
                var target_id := _minion_id(data.get("targetMinionId"), data.get("target"))
                Sfx.play_card(_card_id_for(source_side, source_id), Sfx.CUE_CARD_ATTACK)
                var source := _card_for(source_side, source_id)
                var target := _card_for(target_side, target_id)
                if source != null and target != null:
                    _fx.play_attack(source, target, int(data.get("damage", 0)))
            "death":
                # 阵亡动作的定位字段写在 `sourceSide`（受影响随从所在侧）。
                var side := str(data.get("sourceSide", ""))
                var minion_id := _minion_id(null, data.get("target"))
                Sfx.play_card(_card_id_for(side, minion_id), Sfx.CUE_CARD_DEATH)
                var card := _card_for(side, minion_id)
                if card != null:
                    _fx.fade_out(card)
            "contagion":
                var infected := _card_for(
                    str(data.get("targetSide", "")),
                    _minion_id(null, data.get("target"))
                )
                if infected != null:
                    _fx.flash(infected)


## 复生 / 召唤的卡片在本帧渲染后出现，再补缩放淡入。
func _animate_new_cards(events: Array) -> void:
    for action in events:
        var data: Dictionary = action
        var side := ""
        var minion_id := ""
        match str(data.get("kind", "")):
            "reborn":
                side = str(data.get("sourceSide", ""))
                minion_id = _minion_id(null, data.get("target"))
            "summon":
                var summon: Variant = data.get("summon")
                if summon is Dictionary:
                    side = str((summon as Dictionary).get("side", ""))
                    var minion: Variant = (summon as Dictionary).get("minion")
                    if minion is Dictionary:
                        minion_id = _minion_id(null, minion)
            _:
                continue
        var card := _card_for(side, minion_id)
        if card != null:
            _fx.appear(card)


## 动作里携带的随从实例 id：优先显式字段，其次看快照里的 `minionId`。
func _minion_id(explicit: Variant, snapshot: Variant) -> String:
    if explicit != null:
        return str(explicit)
    if snapshot is Dictionary:
        return str((snapshot as Dictionary).get("minionId", ""))
    return ""


## 在指定侧的行内按实例 id 找卡片；`side` 为空时两侧都找。
func _card_for(side: String, minion_id: String) -> Control:
    if minion_id == "":
        return null
    var rows: Array = []
    match side:
        "own":
            rows = [_own_row]
        "opponent":
            rows = [_opponent_row]
        _:
            rows = [_own_row, _opponent_row]
    for row in rows:
        for child in (row as HFlowContainer).get_children():
            if child is Control and str(child.get_meta("minion_id", "")) == minion_id:
                return child
    return null


## 动作涉及的卡牌 id：战斗动作只带随从实例 id，按侧别回到播放器列表反查。
##
## 列表保留已阵亡条目，召唤随从也带 `cardId`；查不到时返回空串，音效层回落默认。
func _card_id_for(side: String, minion_id: String) -> String:
    if minion_id == "":
        return ""
    var boards: Array = []
    match side:
        "own":
            boards = [_combat.own]
        "opponent":
            boards = [_combat.opponent]
        _:
            boards = [_combat.own, _combat.opponent]
    for board in boards:
        for minion in board:
            if str(minion.get("minionId", "")) == minion_id:
                return str(minion.get("cardId", ""))
    return ""


## 按播放器当前状态重画两行：已存在的节点只改数值与明暗，召唤的随从插入新卡。
func _render_combat_rows() -> void:
    _reconcile_row(_opponent_row, _combat.opponent)
    _reconcile_row(_own_row, _combat.own)


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
    # 战报未到时 `refresh` 会在行内放一条等待提示，战报到达后必须随之清掉。
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
            panel = BoardCardFace.build_card(minion, int(minion.get("index", 0)))
            panel.set_meta("minion_id", minion_id)
            if detail != null:
                detail.track(panel, {"detail_kind": "minion", "minion": minion})
            row.add_child(panel)
        BoardCardFace.set_stats(
            panel,
            str(int(minion.get("attack", 0))),
            str(int(minion.get("health", 0))),
        )
        row.move_child(panel, position)
    # HFlowContainer 的最小宽度只按单张卡计算；显式给出整行宽度才能保持单行居中。
    var count := alive.size()
    row.custom_minimum_size.x = (
        count * UITheme.stage_metric("minion-card-width") + maxi(count - 1, 0) * UITheme.space("space-md")
    )


## 对手英雄卡：对冲演出的对手方来源；重建中或等待配对时为 null。
func opponent_hero_card() -> Control:
    return _opponent_view.hero_card()


## 战斗河道的全局矩形：对冲的相遇点以它的中心为准。
func river_rect() -> Rect2:
    return _river.get_global_rect()


## 结束播报：演示播完并经过 TAIL_MS 后播一次结束音、发出一次对冲请求。
func _refresh_combat_banner() -> void:
    if not _combat.banner_visible() or _clash_announced:
        return
    _clash_announced = true
    Sfx.play_cue(Sfx.CUE_COMBAT_END)
    var combat: Dictionary = GameStore.last_combat
    var winner = combat.get("winnerId")
    var winner_side := ""
    if winner != null:
        winner_side = "own" if str(winner) == GameStore.player_id else "opponent"
    hero_clash_requested.emit(winner_side, int(combat.get("damage", 0)))
