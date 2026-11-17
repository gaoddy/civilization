class_name GamePlayersRail
extends PanelContainer
## 左栏积分榜：每座位一行头像 + 名次角标 + 血条，点击展开该座位的战报。
##
## 只读 `GameUiState.view`，不持有页面门控与意图；展开的座位按 `playerId`
## 记住，刷新整块重建后同一玩家仍然展开。

## 当前展开战报的座位 id；空串表示全部收起。
var _expanded_player_id := ""

## 上一次刷新传入的快照：点击展开/收起时按它原地重建。
var _last_state: GameUiState

## 同页面的详情层，由页面注入；悬停座位行时登记。
var detail: GameDetailLayer = null

var _rail: VBoxContainer


func _init() -> void:
    name = "PlayersPanel"
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
    custom_minimum_size = Vector2(UITheme.metric("rail-width"), 0)
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(box)

    var scroll := ScrollContainer.new()
    scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    box.add_child(scroll)

    _rail = VBoxContainer.new()
    _rail.name = "PlayersRail"
    _rail.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _rail.add_theme_constant_override("separation", UITheme.space("space-xs"))
    scroll.add_child(_rail)


func refresh(state: GameUiState) -> void:
    _last_state = state
    UI.clear(_rail)
    var players: Array = _sorted_players(state.view)
    if players.is_empty():
        _expanded_player_id = ""
        _rail.add_child(UI.empty_hint("暂无玩家"))
        return
    # 记住的 id 在新视图里找不到就清空，否则保持展开。
    if _expanded_player_id != "" and not _has_player(players, _expanded_player_id):
        _expanded_player_id = ""
    for player in players:
        _rail.add_child(_seat_row(player, state))


func _sorted_players(view: Dictionary) -> Array:
    var players: Array = view.get("players", []).duplicate()
    players.sort_custom(func(a, b): return int(a.get("seat", 0)) < int(b.get("seat", 0)))
    return players


func _has_player(players: Array, player_id: String) -> bool:
    for player in players:
        if str(player.get("playerId", "")) == player_id:
            return true
    return false


## 一行座位：头像（名次角标叠在左上）与同宽血条；本人金色描边，淘汰整行压暗。
func _seat_row(player: Dictionary, state: GameUiState) -> PanelContainer:
    var player_id := str(player.get("playerId", ""))
    var is_self := player_id == GameStore.player_id
    var alive := bool(player.get("alive", true))
    var panel := UI.card_panel("accent-row" if is_self else "row")
    panel.name = "SeatRow"
    panel.set_meta("player_id", player_id)
    panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    panel.mouse_filter = Control.MOUSE_FILTER_STOP
    panel.gui_input.connect(_on_seat_input.bind(player_id))
    if not alive:
        UI.apply_dim(panel)

    var box := VBoxContainer.new()
    box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_theme_constant_override("separation", UITheme.space("space-xs"))
    panel.add_child(box)

    var avatar_size := UITheme.metric("avatar-rail")
    var avatar_box := Control.new()
    avatar_box.name = "SeatAvatar"
    avatar_box.custom_minimum_size = Vector2(avatar_size, avatar_size)
    avatar_box.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
    avatar_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_child(avatar_box)

    var avatar := UI.avatar(GameCardFace.optional_text(player.get("heroCardId")), avatar_size)
    avatar.mouse_filter = Control.MOUSE_FILTER_IGNORE
    avatar_box.add_child(avatar)
    _ignore_mouse(avatar)

    if player.get("rank") != null:
        var badge := UI.rank_badge(int(player.get("rank")))
        badge.name = "SeatRankBadge"
        badge.position = Vector2(-3, -3)
        badge.mouse_filter = Control.MOUSE_FILTER_IGNORE
        avatar_box.add_child(badge)
        _ignore_mouse(badge)

    var hp := UI.hp_bar(int(player.get("hp", 0)), state.hp_cap, avatar_size)
    hp.name = "SeatHpBar"
    box.add_child(hp)

    if player_id == _expanded_player_id:
        box.add_child(_report_block(player, state.view))

    _track_detail(panel, player, state.hp_cap)
    return panel


## 点击行切换展开；重建在下一帧做，避免在自身输入回调里释放行。
func _on_seat_input(event: InputEvent, player_id: String) -> void:
    var pressed := false
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        pressed = event.pressed
    elif event is InputEventScreenTouch:
        pressed = event.pressed
    if not pressed:
        return
    _expanded_player_id = "" if _expanded_player_id == player_id else player_id
    call_deferred("_rebuild")


func _rebuild() -> void:
    if _last_state == null or not is_inside_tree():
        return
    refresh(_last_state)


## 该座位参与过的战报；没有条目时显示「无战报」。
func _report_block(player: Dictionary, view: Dictionary) -> VBoxContainer:
    var player_id := str(player.get("playerId", ""))
    var report := VBoxContainer.new()
    report.name = "SeatReport"
    report.set_meta("player_id", player_id)
    report.mouse_filter = Control.MOUSE_FILTER_IGNORE
    report.add_theme_constant_override("separation", UITheme.space("space-2xs"))

    var entries := _combat_entries(view, player_id)
    if entries.is_empty():
        report.add_child(_report_line("无战报"))
        return report
    var names := _player_names(view)
    for entry in entries:
        report.add_child(_report_line(_entry_text(entry, player_id, names)))
    return report


func _combat_entries(view: Dictionary, player_id: String) -> Array:
    var entries: Array = []
    for entry in view.get("combatLog", []):
        if not (entry is Dictionary):
            continue
        var ids: Array = entry.get("participantIds", [])
        for pid in ids:
            if str(pid) == player_id:
                entries.append(entry)
                break
    return entries


func _player_names(view: Dictionary) -> Dictionary:
    var names := {}
    for player in view.get("players", []):
        if player is Dictionary:
            names[str(player.get("playerId", ""))] = GameCardFace.optional_text(
                player.get("displayName")
            )
    return names


## 「第 N 回合 · 对手名 · 胜/负 · 伤害 X」；幽灵战对手写「幽灵（名字）」，
## 伤害为 0 时省略伤害段。
func _entry_text(entry: Dictionary, player_id: String, names: Dictionary) -> String:
    var opponent_id := ""
    for pid in entry.get("participantIds", []):
        if str(pid) != player_id:
            opponent_id = str(pid)
            break
    var opponent := str(names.get(opponent_id, opponent_id))
    if bool(entry.get("mirror", false)):
        var ghost_id := "" if entry.get("ghostId") == null else str(entry.get("ghostId"))
        opponent = "幽灵" if ghost_id == "" else "幽灵（%s）" % str(names.get(ghost_id, ghost_id))

    var winner_id := "" if entry.get("winnerId") == null else str(entry.get("winnerId"))
    var loser_id := "" if entry.get("loserId") == null else str(entry.get("loserId"))
    var result := "平"
    if winner_id == player_id:
        result = "胜"
    elif loser_id == player_id or winner_id != "":
        result = "负"

    var parts := PackedStringArray([
        "第 %d 回合" % int(entry.get("turn", 0)),
        opponent,
        result,
    ])
    var damage := int(entry.get("damage", 0))
    if damage != 0:
        parts.append("伤害 %d" % damage)
    return " · ".join(parts)


func _report_line(text: String) -> Label:
    var line := UI.label(text, UITheme.font_size("font-caption"), false)
    line.mouse_filter = Control.MOUSE_FILTER_IGNORE
    line.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    return line


## 行内控件全部忽略鼠标，保证整行任意位置都能接收点击与悬停。
func _ignore_mouse(node: Node) -> void:
    if node is Control:
        node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    for child in node.get_children():
        _ignore_mouse(child)


func _track_detail(row: PanelContainer, player: Dictionary, hp_cap: int) -> void:
    if detail == null or not is_instance_valid(detail):
        return
    detail.track(row, {
        "detail_kind": "seat",
        "player": player,
        "hpCap": hp_cap,
    })
