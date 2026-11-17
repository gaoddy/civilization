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
var _banner: Label
var _own_row: HFlowContainer
## 演示播放器（纯数据）与当前演示所属回合。
var _combat := CombatPlayback.new()
var _combat_turn := -1


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

    _banner = UI.label("", UITheme.font_size("font-display"), false)
    _banner.name = "CombatBanner"
    _banner.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _banner.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _body.add_child(_banner)

    _body.add_child(UI.section_title("我方随从"))
    _own_row = _new_flow_row("CombatOwnRow")
    _body.add_child(_own_row)


func _new_flow_row(node_name: String) -> HFlowContainer:
    var row := HFlowContainer.new()
    row.name = node_name
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


## 演示进行中：阶段为战斗、播放器属于本回合、有本回合战报且尚未播完。
func playing() -> bool:
    return (
        GameStore.phase == Protocol.PHASE_COMBAT
        and _combat_turn == GameStore.turn
        and GameStore.combat_is_current()
        and not _combat.is_finished()
    )


## 推进演示；由页面在 `playing()` 为真时按帧调用。
func advance(delta: float) -> void:
    _combat.advance(delta * 1000.0)
    _render_combat_rows()
    _refresh_combat_banner()


## 装载本场演示：战前阵容与动作序列，并按 `phaseDeadline` 反推续播进度。
func _start_combat(combat: Dictionary) -> void:
    _combat.start(
        combat.get("actions", []),
        combat.get("ownBoard", []),
        combat.get("opponentBoard", [])
    )
    var deadline := unix_from_rfc3339(GameStore.deadline)
    if GameStore.phase == Protocol.PHASE_COMBAT and deadline > 0.0:
        var remaining_ms := maxf(deadline - Time.get_unix_time_from_system(), 0.0) * 1000.0
        _combat.seek(maxf(float(_combat.total_ms()) - remaining_ms, 0.0))
    _combat_turn = GameStore.turn


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


## RFC3339 时间转 Unix 秒；供演示续播按 `phaseDeadline` 反推进度。
static func unix_from_rfc3339(value: String) -> float:
    var text := value
    if text.ends_with("Z"):
        text = text.substr(0, text.length() - 1)
    var dot := text.find(".")
    if dot >= 0:
        text = text.substr(0, dot)
    return Time.get_unix_time_from_datetime_string(text)
