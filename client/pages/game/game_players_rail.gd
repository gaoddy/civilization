class_name GamePlayersRail
extends PanelContainer
## 左栏座位列：按服务端名次升序（第 1 名在最上），每座位一行头像，名次角标叠在左上、
## 血条叠在头像底部内侧；点击展开该座位的战报。
##
## 只读 `GameUiState.view`，不持有页面门控与意图；展开的座位按 `playerId`
## 记住，刷新整块重建后同一玩家仍然展开。

## 当前展开战报的座位 id；空串表示全部收起。
var _expanded_player_id := ""

## 上一次刷新传入的快照：点击展开/收起时按它原地重建。
var _last_state: GameUiState

## 同页面的详情层，由页面注入；悬停座位行时登记。
var detail: DetailLayer = null

## 战报浮层，由页面注入；展开战报时挂在全屏层上，允许覆盖其他 UI。
var report_layer: Control = null

var _rail: VBoxContainer
var _scroll: ScrollContainer
var _report_panel: PanelContainer
## 最近一次构行使用的头像边长；左栏尺寸变化后据此判断是否需要重排。
var _avatar_side := 0


func _init() -> void:
    name = "PlayersPanel"
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
    custom_minimum_size = Vector2(UITheme.stage_metric("rail-width"), 0)
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(box)

    _scroll = ScrollContainer.new()
    _scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    box.add_child(_scroll)
    _scroll.get_v_scroll_bar().value_changed.connect(_on_scroll_changed)

    _rail = VBoxContainer.new()
    _rail.name = "PlayersRail"
    _rail.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    # 撑满左栏可视高度，8 个座位行再按 EXPAND_FILL 均分这段高度。
    _rail.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _rail.add_theme_constant_override("separation", UITheme.stage_metric("seat-row-gap"))
    # 视口尺寸变化时按新的可用高度重算头像；监听滚动视口而不是内容高度，
    # 否则上一次的较大头像会把内容撑住、无法随视口收缩。
    _scroll.resized.connect(_on_rail_resized)
    _scroll.add_child(_rail)

    _report_panel = UIControls.card_panel("raised")
    _report_panel.name = "SeatReportPanel"
    _report_panel.custom_minimum_size = Vector2(UITheme.stage_metric("report-width"), 0)
    _report_panel.mouse_filter = Control.MOUSE_FILTER_STOP
    _report_panel.visible = false


func refresh(state: GameUiState) -> void:
    _last_state = state
    UIControls.clear(_rail)
    var players: Array = _sorted_players(state.view)
    if players.is_empty():
        _expanded_player_id = ""
        _avatar_side = 0
        _hide_report()
        _rail.add_child(UIControls.empty_hint(tr("ui.game.rail.empty")))
        return
    # 记住的 id 在新视图里找不到就清空，否则保持展开。
    if _expanded_player_id != "" and not _has_player(players, _expanded_player_id):
        _expanded_player_id = ""
    var avatar_side := _seat_avatar_side(players.size())
    _avatar_side = avatar_side
    for player in players:
        _rail.add_child(_seat_row(player, state, avatar_side))
    _sync_report()


## 按服务端下发的 `rank` 升序排列，第 1 名在最上；名次相同或缺失时按座位号，
## 无名次的座位排在所有有名次之后。客户端只使用服务端名次排显示顺序，不推导名次。
func _sorted_players(view: Dictionary) -> Array:
    var players: Array = view.get("players", []).duplicate()
    players.sort_custom(
        func(a, b):
            var rank_a := _rank_value(a)
            var rank_b := _rank_value(b)
            if rank_a != rank_b:
                return rank_a < rank_b
            return int(a.get("seat", 0)) < int(b.get("seat", 0))
    )
    return players


## 排序用的名次值；`rank` 缺失时用哨兵值垫底。
static func _rank_value(player: Dictionary) -> int:
    var rank = player.get("rank")
    return int(rank) if rank != null else 1 << 20


func _has_player(players: Array, player_id: String) -> bool:
    for player in players:
        if str(player.get("playerId", "")) == player_id:
            return true
    return false


## 手机基准（960×540）用紧凑座位行与名次角标，桌面与平板保持原尺寸。
static func _compact() -> bool:
    return ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE


static func _row_variant(is_self: bool, compact: bool) -> String:
    if is_self:
        return "accent-row-compact" if compact else "accent-row"
    return "row-compact" if compact else "row"


## 座位头像边长：座位行均分左栏高度后，每行内容高度与左栏内宽的较小值；
## 布局尚未完成（左栏尺寸为 0）时退回 `avatar-rail` 令牌。
func _seat_avatar_side(count: int) -> int:
    var fallback := UITheme.stage_metric("avatar-rail")
    if count <= 0 or _rail == null or _scroll == null:
        return fallback
    if _scroll.size.y <= 0.0 or _scroll.size.x <= 0.0:
        return fallback
    var gap := UITheme.stage_metric("seat-row-gap")
    var margin := UITheme.space("space-2xs") if _compact() else UITheme.space("space-md")
    var cell := (_scroll.size.y - float(gap * (count - 1))) / float(count)
    var by_height := int(floor(cell - float(margin * 2)))
    var by_width := int(floor(_scroll.size.x - float(margin * 2)))
    return maxi(mini(by_height, by_width), 1)


## 左栏尺寸变化后按新的可用空间重算头像边长；边长没变就不重排。
func _on_rail_resized() -> void:
    if _last_state == null:
        return
    var count := int(_last_state.view.get("players", []).size())
    if count == 0:
        return
    if _seat_avatar_side(count) == _avatar_side:
        return
    call_deferred("_rebuild")


## 一行座位：头像块内叠名次角标与贴底血条，不为血条单独留行高；本人金色描边，淘汰整行压暗。
## 座位行垂直均分左栏高度，头像边长按行内容高度与左栏内宽解算，手机基准随之放大。
func _seat_row(player: Dictionary, state: GameUiState, avatar_size: int) -> PanelContainer:
    var player_id := str(player.get("playerId", ""))
    var is_self := player_id == GameStore.player_id
    var alive := bool(player.get("alive", true))
    var compact := _compact()
    var panel := UIControls.card_panel(_row_variant(is_self, compact))
    panel.name = "SeatRow"
    panel.set_meta("player_id", player_id)
    panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    # 8 行平分左栏剩余高度，行高不再由头像固定尺寸决定。
    panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    panel.mouse_filter = Control.MOUSE_FILTER_STOP
    panel.gui_input.connect(_on_seat_input.bind(player_id))
    if not alive:
        UIControls.apply_dim(panel)

    var box := VBoxContainer.new()
    box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_theme_constant_override(
        "separation",
        UITheme.space("space-2xs") if compact else UITheme.space("space-xs")
    )
    panel.add_child(box)

    var avatar_box := Control.new()
    avatar_box.name = "SeatAvatar"
    avatar_box.custom_minimum_size = Vector2(avatar_size, avatar_size)
    avatar_box.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
    avatar_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_child(avatar_box)

    var avatar := Avatar.create(CardText.optional_text(player.get("heroCardId")), avatar_size)
    avatar.mouse_filter = Control.MOUSE_FILTER_IGNORE
    avatar_box.add_child(avatar)
    _ignore_mouse(avatar)

    # 血条叠在头像底部内侧：左右与底部各留 `space-2xs`，圆角取高度一半成胶囊；
    # 宽度随头像边长变化，不额外占行高。
    var hp_height := UITheme.stage_metric("seat-hp-bar-height")
    var hp_inset := UITheme.space("space-2xs")
    var hp := HpBar.create(
        int(player.get("hp", 0)),
        state.hp_cap,
        avatar_size - hp_inset * 2,
        int(hp_height * 0.5),
    )
    hp.name = "SeatHpBar"
    hp.position = Vector2(hp_inset, avatar_size - hp_height - hp_inset)
    avatar_box.add_child(hp)

    if player.get("rank") != null:
        var badge := RankBadge.create(int(player.get("rank")))
        badge.name = "SeatRankBadge"
        # 名次角标收进头像可选区域内，不再挂在头像外沿。
        var badge_inset := UITheme.space("space-2xs")
        badge.position = Vector2(badge_inset, badge_inset)
        badge.mouse_filter = Control.MOUSE_FILTER_IGNORE
        avatar_box.add_child(badge)
        _ignore_mouse(badge)

    _track_detail(panel, player, state.hp_cap)
    return panel


## 点击行切换展开；重建在下一帧做，避免在自身输入回调里释放行。
func _on_seat_input(event: InputEvent, player_id: String) -> void:
    # 模拟触控档位下鼠标事件会同时产生触摸事件，只认触摸，避免一次点击翻两次。
    var pressed := false
    if event is InputEventScreenTouch:
        pressed = event.pressed
    elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT \
            and not ScreenProfile.is_simulated_touch():
        pressed = event.pressed
    if not pressed:
        return
    _expanded_player_id = "" if _expanded_player_id == player_id else player_id
    call_deferred("_rebuild")


func _rebuild() -> void:
    if _last_state == null or not is_inside_tree():
        return
    refresh(_last_state)


## 战报不占左栏宽度：展开时放进全屏浮层，右侧可覆盖中区与右栏。
func _sync_report() -> void:
    if (
        report_layer == null
        or _last_state == null
        or _expanded_player_id == ""
    ):
        _hide_report()
        return
    var player := _find_player(_expanded_player_id)
    if player.is_empty():
        _hide_report()
        return
    if detail != null and is_instance_valid(detail):
        detail.report_open = true
        detail.hide_panel()
    if _report_panel.get_parent() == null:
        report_layer.add_child(_report_panel)
    UIControls.clear(_report_panel)
    _report_panel.add_child(_report_block(player, _last_state.view))
    _report_panel.visible = true
    call_deferred("_position_report")


func _hide_report() -> void:
    if _report_panel == null:
        return
    UIControls.clear(_report_panel)
    _report_panel.visible = false
    if detail != null and is_instance_valid(detail):
        detail.report_open = false


func _position_report() -> void:
    if _report_panel == null or not _report_panel.visible or report_layer == null:
        return
    var row := _find_row(_expanded_player_id)
    if row == null:
        _hide_report()
        return
    var panel_size := _report_panel.get_combined_minimum_size()
    _report_panel.size = panel_size
    var layer_rect := Rect2(Vector2.ZERO, report_layer.size)
    var margin := UITheme.space("space-xs")
    var gap := UITheme.space("space-sm")
    var row_global := row.get_global_rect()
    var layer_scale := report_layer.get_global_transform().get_scale().x
    var row_rect := Rect2(
        report_layer.get_global_transform().affine_inverse() * row_global.position,
        row_global.size / layer_scale
    )
    var position := Vector2(row_rect.end.x + gap, row_rect.position.y)
    position.x = clampf(
        position.x,
        margin,
        maxf(margin, layer_rect.size.x - panel_size.x - margin)
    )
    position.y = clampf(
        position.y,
        margin,
        maxf(margin, layer_rect.size.y - panel_size.y - margin)
    )
    _report_panel.position = position


func _on_scroll_changed(_value: float) -> void:
    if _report_panel != null and _report_panel.visible:
        call_deferred("_position_report")


func _find_row(player_id: String) -> Control:
    for child in _rail.get_children():
        if child is Control and str(child.get_meta("player_id", "")) == player_id:
            return child
    return null


func _find_player(player_id: String) -> Dictionary:
    if _last_state == null:
        return {}
    for player in _last_state.view.get("players", []):
        if str(player.get("playerId", "")) == player_id:
            return player
    return {}


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
        report.add_child(_report_line(tr("ui.game.rail.no_report")))
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
            names[str(player.get("playerId", ""))] = CardText.optional_text(
                player.get("displayName")
            )
    return names


## 「第 N 回合 · 对手名 · 胜/负 · 伤害 X」；幽灵战对手写「幽灵（名字）」，
## `ghostId` 为空时只写「幽灵」、不写空括号；伤害为 0 时省略伤害段。
func _entry_text(entry: Dictionary, player_id: String, names: Dictionary) -> String:
    var opponent_id := ""
    for pid in entry.get("participantIds", []):
        if str(pid) != player_id:
            opponent_id = str(pid)
            break
    var opponent := str(names.get(opponent_id, opponent_id))
    if bool(entry.get("mirror", false)):
        var ghost_id := "" if entry.get("ghostId") == null else str(entry.get("ghostId"))
        opponent = (
            tr("ui.game.rail.ghost")
            if ghost_id == ""
            else tr("ui.game.rail.ghost_named") % str(names.get(ghost_id, ghost_id))
        )

    var winner_id := "" if entry.get("winnerId") == null else str(entry.get("winnerId"))
    var loser_id := "" if entry.get("loserId") == null else str(entry.get("loserId"))
    var result := tr("ui.game.rail.draw")
    if winner_id == player_id:
        result = tr("ui.game.rail.win")
    elif loser_id == player_id or winner_id != "":
        result = tr("ui.game.rail.lose")

    var parts := PackedStringArray([
        tr("ui.game.rail.turn") % int(entry.get("turn", 0)),
        opponent,
        result,
    ])
    var damage := int(entry.get("damage", 0))
    if damage != 0:
        parts.append(tr("ui.game.rail.damage") % damage)
    return " · ".join(parts)


func _report_line(text: String) -> Label:
    var line := UIControls.label(text, UITheme.font_size("font-caption"), false)
    line.mouse_filter = Control.MOUSE_FILTER_IGNORE
    line.custom_minimum_size = Vector2(
        UITheme.stage_metric("report-width") - UITheme.space("space-lg") * 2,
        0,
    )
    line.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
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
