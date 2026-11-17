class_name GameCombatView
extends PanelContainer
## 战斗视图：双方随从行与结果横幅。
##
## 演示进度由 `CombatPlayback` 纯数据播放器驱动；页面只负责在演示中调用
## `advance()`，本视图不发送任何意图。

## 卡片详情登记入口，由对局页注入；独立实例化时可为 null。
var detail: GameDetailLayer = null

var _body: VBoxContainer
var _opponent_row: HFlowContainer
var _banner_slot: Control
var _banner: Label
var _own_row: HFlowContainer
## 演示播放器（纯数据）与当前演示所属回合。
var _combat := CombatPlayback.new()
var _combat_turn := -1
## 覆盖层演出：克隆卡碰撞往返、闪光、飘字与淡出。
var _fx: CombatFx


func _init() -> void:
    name = "CombatPanel"
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
    size_flags_vertical = Control.SIZE_EXPAND_FILL
    visible = false

    _body = VBoxContainer.new()
    _body.name = "CombatBody"
    _body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _body.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(_body)

    _body.add_child(UI.section_title("对手随从"))
    _opponent_row = _new_flow_row("CombatOpponentRow")
    _body.add_child(_opponent_row)

    # 横幅槽：高度常驻，横幅出现时只填充文案，不推动上下两行。
    _banner_slot = Control.new()
    _banner_slot.name = "CombatBannerSlot"
    _banner_slot.custom_minimum_size = Vector2(0, UITheme.metric("combat-banner-height"))
    _banner_slot.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _body.add_child(_banner_slot)

    _banner = UI.label("", UITheme.font_size("font-display"), false)
    _banner.name = "CombatBanner"
    _banner.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _banner.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    _banner.set_anchors_preset(Control.PRESET_FULL_RECT)
    _banner_slot.add_child(_banner)

    _body.add_child(UI.section_title("我方随从"))
    _own_row = _new_flow_row("CombatOwnRow")
    _body.add_child(_own_row)

    # 覆盖层最后加入，盖在阵容行与横幅之上；不参与布局。
    _fx = CombatFx.new()
    add_child(_fx)


func _new_flow_row(node_name: String) -> HFlowContainer:
    var row := HFlowContainer.new()
    row.name = node_name
    # 行高常驻：一方随从全灭时该行不塌陷，两行在演示全程保持位置。
    row.custom_minimum_size = Vector2(0, UITheme.metric("card-height"))
    row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_theme_constant_override("h_separation", UITheme.space("space-sm"))
    row.add_theme_constant_override("v_separation", UITheme.space("space-sm"))
    return row


func refresh(_state: GameUiState) -> void:
    if not GameStore.combat_is_current():
        _banner.visible = false
        UI.clear(_opponent_row)
        UI.clear(_own_row)
        _opponent_row.add_child(UI.empty_hint("等待战斗动作…"))
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


## 横幅：动作播完并经过 TAIL_MS 后才出现。
func _refresh_combat_banner() -> void:
    _banner.visible = _combat.banner_visible()
    if not _banner.visible:
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
    # `mirror` 为真表示本场是对战已出局者的幽灵阵容，不是镜像战。
    var ghost := "（幽灵）" if bool(combat.get("mirror", false)) else ""
    _banner.text = "%s%s · 伤害 %d · 我方 %d vs 对手 %d" % [
        result,
        ghost,
        int(combat.get("damage", 0)),
        _combat.own.size(),
        _combat.opponent.size(),
    ]
    _banner.add_theme_color_override("font_color", UITheme.color("color-" + variant) if variant != "accent" else UITheme.color("color-accent"))

