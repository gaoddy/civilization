class_name GameCombatView
extends PanelContainer
## 战斗组合：高 418 = 对手 64 + 6 + 对手行 146 + 18 + 河道 32 + 6 + 我方行 146。
##
## 演示进度由 `CombatPlayback` 纯数据播放器驱动；页面只负责在演示中调用
## `advance()`，本视图不发送任何意图。结果追加在河道的回合数之后。

## 卡片详情登记入口，由对局页注入；独立实例化时可为 null。
var detail: GameDetailLayer = null

var _state: GameUiState = null
var _opponent_view: GameOpponentView
var _river: GameRiver
var _opponent_row: HFlowContainer
var _own_row: HFlowContainer
## 演示播放器（纯数据）与当前演示所属回合。
var _combat := CombatPlayback.new()
var _combat_turn := -1
## 覆盖层演出：克隆卡碰撞往返、闪光、飘字与淡出。
var _fx: CombatFx


## 舞台内小间距：iPhone 基准下再收紧一档。
func _stage_gap() -> int:
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        return UITheme.space("space-xs")
    return UITheme.space("space-sm")


func _init() -> void:
    name = "CombatPanel"
    add_theme_stylebox_override("panel", UITheme.panel_style("bare"))
    custom_minimum_size = Vector2(
        0, UITheme.stage_metric("portrait-size") + _stage_gap()
            + UITheme.stage_metric("card-height") * 2 + UITheme.stage_metric("recruit-shop-gap")
            + UITheme.stage_metric("river-height") + _stage_gap()
    )
    size_flags_vertical = Control.SIZE_EXPAND_FILL
    visible = false

    var body := VBoxContainer.new()
    body.name = "CombatBody"
    body.add_theme_constant_override("separation", 0)
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

    body.add_child(_gap("CombatRiverGap", UITheme.stage_metric("recruit-shop-gap")))

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
    row.custom_minimum_size = Vector2(0, UITheme.stage_metric("card-height"))
    row.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    row.add_theme_constant_override("h_separation", UITheme.space("space-md"))
    row.add_theme_constant_override("v_separation", UITheme.space("space-md"))
    return row


func set_detail(value: GameDetailLayer) -> void:
    detail = value
    _opponent_view.detail = value


func refresh(state: GameUiState) -> void:
    _state = state
    _opponent_view.refresh(state)
    _river.refresh(state)
    if not GameStore.combat_is_current():
        UI.clear(_opponent_row)
        UI.clear(_own_row)
        _opponent_row.custom_minimum_size.x = 0
        _own_row.custom_minimum_size.x = 0
        _opponent_row.add_child(UI.empty_hint(tr("ui.game.combat.waiting")))
        _river.set_combat_detail("", UITheme.color("color-accent"))
        return
    var combat: Dictionary = GameStore.last_combat
    if _combat_turn != GameStore.turn:
        _start_combat(combat)
    _render_combat_rows()
    _refresh_combat_banner()


## 进入非战斗阶段时丢弃本场演示归属，下一场战斗从头播放。
func reset() -> void:
    _combat_turn = -1
    _fx.clear()
    _river.set_combat_detail("", UITheme.color("color-accent"))


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
    var started_at := UI.unix_from_rfc3339(str(combat.get("startedAt", "")))
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


## 按动作类型开启演出；数值已在播放器中按服务端快照落地。
##
## `attack` 只移动与反馈，不改存活；阵亡只由 `death` 触发，因此目标不会在
## 攻方到达前消失。
func _play_events(events: Array) -> void:
    for action in events:
        var data: Dictionary = action
        match str(data.get("kind", "")):
            "attack":
                var source := _card_for(
                    str(data.get("sourceSide", "")),
                    _minion_id(data.get("sourceMinionId"), data.get("source"))
                )
                var target := _card_for(
                    str(data.get("targetSide", "")),
                    _minion_id(data.get("targetMinionId"), data.get("target"))
                )
                if source != null and target != null:
                    _fx.play_attack(source, target, int(data.get("damage", 0)))
            "death":
                # 阵亡动作的定位字段写在 `sourceSide`（受影响随从所在侧）。
                var card := _card_for(
                    str(data.get("sourceSide", "")),
                    _minion_id(null, data.get("target"))
                )
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
            panel = GameCardFace.read_only_minion_card(minion)
            panel.set_meta("minion_id", minion_id)
            if detail != null:
                detail.track(panel, {"detail_kind": "minion", "minion": minion})
            row.add_child(panel)
        var attack_label = panel.find_child("CardAttack", true, false)
        if attack_label != null:
            attack_label.text = str(int(minion.get("attack", 0)))
        var health_label = panel.find_child("CardHealth", true, false)
        if health_label != null:
            health_label.text = str(int(minion.get("health", 0)))
        row.move_child(panel, position)
    # HFlowContainer 的最小宽度只按单张卡计算；显式给出整行宽度才能保持单行居中。
    var count := alive.size()
    row.custom_minimum_size.x = (
        count * UITheme.stage_metric("card-width") + maxi(count - 1, 0) * UITheme.space("space-md")
    )


## 胜负与颜色：胜 success、负 danger、平 accent。
func _result(combat: Dictionary) -> Dictionary:
    var text := tr("ui.game.combat.draw")
    var variant := "accent"
    var winner = combat.get("winnerId")
    if winner != null:
        if str(winner) == GameStore.player_id:
            text = tr("ui.game.combat.win")
            variant = "success"
        else:
            text = tr("ui.game.combat.lose")
            variant = "danger"
    return {"text": text, "variant": variant}


## 对手展示名：优先 `displayName`，缺失回退 `playerId`；幽灵战追加幽灵后缀。
func _opponent_label() -> String:
    var opponent_id := ""
    if _state != null:
        opponent_id = GameCardFace.optional_text(_state.view.get("opponentPlayerId"))
    var name := ""
    if _state != null:
        for player in _state.view.get("players", []):
            if player is Dictionary and str(player.get("playerId", "")) == opponent_id:
                name = GameCardFace.optional_text(player.get("displayName"))
                if name == "":
                    name = GameCardFace.optional_text(player.get("playerId"))
                break
    if name == "":
        name = tr("ui.game.side_panel.unknown_hero")
    if bool(GameStore.last_combat.get("mirror", false)):
        name += tr("ui.game.combat.ghost_suffix")
    return name


## 横幅：动作播完并经过 TAIL_MS 后才把结果追加到河道的回合数之后。
func _refresh_combat_banner() -> void:
    if not _combat.banner_visible():
        _river.set_combat_detail("", UITheme.color("color-accent"))
        return
    var combat: Dictionary = GameStore.last_combat
    var result := _result(combat)
    var text := tr("ui.game.combat.banner") % [
        str(result.get("text", "")),
        "",
        _opponent_label(),
        int(combat.get("damage", 0)),
    ]
    _river.set_combat_detail(text, UITheme.color("color-" + str(result.get("variant", "accent"))))
