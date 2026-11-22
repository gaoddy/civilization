class_name GamePlayersRail
extends PanelContainer
## 左栏座位列：按服务端名次升序（第 1 名在最上），每个座位是一个方形可选框，
## 头像铺满框内，名次角标叠在左上、血条叠在头像底部内侧；悬停显示该座位的详情，
## 详情里同时给出身份与对战记录，不再有点击展开的战报浮层。
##
## 只读 `GameUiState.view`，不持有页面门控与意图。8 席固定一屏放下，不使用滚动容器；
## 座位行按栏内实际高度手动排布，不参与左栏最小尺寸，避免行高反过来撑大页面。

## 行描边宽度：由可选框底板绘制，头像内缩同宽，避免头像描边与框线叠成双线。
const ROW_BORDER_WIDTH := 2

## 上一次刷新传入的快照：左栏尺寸变化时按它原地重建。
var _last_state: GameUiState

## 同页面的详情层，由页面注入；悬停座位行时登记。
var detail: DetailLayer = null

var _rail: Control
## 最近一次构行使用的行边长；左栏尺寸变化后据此判断是否需要重排。
var _last_side := 0


func _init() -> void:
    name = "PlayersPanel"
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
    custom_minimum_size = Vector2(UITheme.stage_metric("rail-width"), 0)
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(box)

    # 普通 Control 承载座位行：行按边长手动排布并垂直居中，行高不进入最小尺寸，
    # 左栏高度因此只由布局分配决定（8 席不滚动，也不把行高拉伸到均分高度）。
    _rail = Control.new()
    _rail.name = "PlayersRail"
    _rail.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _rail.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _rail.mouse_filter = Control.MOUSE_FILTER_IGNORE
    # 尺寸变化后按新的可用空间重算边长；监听左栏自身，避免旧边长撑住布局。
    _rail.resized.connect(_on_rail_resized)
    box.add_child(_rail)


func refresh(state: GameUiState) -> void:
    _last_state = state
    UIControls.clear(_rail)
    var players: Array = _sorted_players(state.view)
    if players.is_empty():
        _last_side = 0
        _rail.add_child(UIControls.empty_hint(tr("ui.game.rail.empty")))
        return
    var side := _seat_side(players.size())
    _last_side = side
    var rows: Array = []
    for player in players:
        var row := _seat_row(player, state, side)
        _rail.add_child(row)
        rows.append(row)
    _layout_rows(rows, side)


## 座位行在栏内按边长排开：整组上下、左右居中；行尺寸由边长决定，不参与最小尺寸。
func _layout_rows(rows: Array, side: int) -> void:
    if rows.is_empty():
        return
    var gap := UITheme.stage_metric("seat-row-gap")
    var total := rows.size() * side + maxi(rows.size() - 1, 0) * gap
    var top := maxi(int((_rail.size.y - float(total)) * 0.5), 0)
    var left := maxi(int((_rail.size.x - float(side)) * 0.5), 0)
    for index in rows.size():
        var row := rows[index] as Control
        row.position = Vector2(left, top + index * (side + gap))
        row.size = Vector2(side, side)


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


## 座位行边长：栏内宽与「人均可用高度」的较小值；行就是头像的方形外框。
## 布局尚未完成（左栏尺寸为 0）时退回 `avatar-rail` 令牌。
func _seat_side(count: int) -> int:
    var fallback := UITheme.stage_metric("avatar-rail")
    if count <= 0 or _rail == null:
        return fallback
    if _rail.size.x <= 0.0 or _rail.size.y <= 0.0:
        return fallback
    # 栏内实际高度即可用高度：座位行手动排布，不再把自身高度撑进最小尺寸。
    var basis := int(_rail.size.y)
    var gap := UITheme.stage_metric("seat-row-gap")
    var by_height := int(floor((basis - float(gap * (count - 1))) / float(count)))
    var by_width := int(floor(_rail.size.x))
    return maxi(mini(by_height, by_width), ROW_BORDER_WIDTH * 2 + 1)


## 左栏尺寸变化后按新的可用空间重算边长；边长没变就不重排。
func _on_rail_resized() -> void:
    if _last_state == null:
        return
    var count := int(_last_state.view.get("players", []).size())
    if count == 0:
        return
    if _seat_side(count) == _last_side:
        return
    call_deferred("_rebuild")


## 行底板：方形外框，2px 描边与 `radius-md` 圆角，内容只让出描边宽度，头像铺满框内。
## 本人行用强调色背景与描边；悬停与普通行由 `SelectableBox` 叠加状态。
static func _row_style(is_self: bool) -> StyleBoxFlat:
    var box := StyleBoxFlat.new()
    box.bg_color = UITheme.alpha("color-accent", 0.12) if is_self else UITheme.color("color-surface")
    box.border_color = UITheme.color("color-accent") if is_self else UITheme.color("color-border")
    box.set_border_width_all(ROW_BORDER_WIDTH)
    box.set_corner_radius_all(UITheme.radius("radius-md"))
    box.set_content_margin_all(ROW_BORDER_WIDTH)
    return box


## 一个座位：方形框内头像铺满，名次角标与贴底血条叠在头像上。
## 头像自身描边透明，外框由行底板绘制，避免两层描边叠成粗线；淘汰整行压暗。
func _seat_row(player: Dictionary, state: GameUiState, side: int) -> SelectableBox:
    var player_id := str(player.get("playerId", ""))
    var is_self := player_id == GameStore.player_id
    var alive := bool(player.get("alive", true))
    var row := SelectableBox.new()
    row.name = "SeatRow_%s" % player_id
    row.set_meta("player_id", player_id)
    # 行尺寸由 `_layout_rows` 按边长写入；最小尺寸保持 0，避免行高反推左栏最小高度。
    row.custom_minimum_size = Vector2.ZERO
    row.mouse_filter = Control.MOUSE_FILTER_STOP
    row.configure(_row_style(is_self), SelectableBox.Outline.INLINE, not ScreenProfile.is_touch())
    if not alive:
        UIControls.apply_dim(row)

    # 头像边长 = 行边长减两侧描边，铺满行内容区，行与头像之间只剩描边本身。
    var inner := side - ROW_BORDER_WIDTH * 2
    var seat := Control.new()
    seat.name = "SeatAvatar"
    seat.mouse_filter = Control.MOUSE_FILTER_IGNORE
    seat.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    row.content().add_child(seat)

    var avatar := Avatar.create(
        CardText.optional_text(player.get("heroCardId")),
        inner,
        UITheme.color("color-transparent"),
    )
    avatar.mouse_filter = Control.MOUSE_FILTER_IGNORE
    avatar.custom_minimum_size = Vector2.ZERO
    avatar.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    seat.add_child(avatar)
    _ignore_mouse(avatar)

    # 血条叠在头像底部内侧：左右与底部各留 `space-2xs`，圆角取高度一半成胶囊。
    var hp_height := UITheme.stage_metric("seat-hp-bar-height")
    var hp_inset := UITheme.space("space-2xs")
    var hp := HpBar.create(
        int(player.get("hp", 0)),
        state.hp_cap,
        inner - hp_inset * 2,
        int(hp_height * 0.5),
    )
    hp.name = "SeatHpBar"
    hp.position = Vector2(hp_inset, inner - hp_height - hp_inset)
    seat.add_child(hp)

    if player.get("rank") != null:
        var badge := RankBadge.create(int(player.get("rank")))
        badge.name = "SeatRankBadge"
        var badge_inset := UITheme.space("space-2xs")
        badge.position = Vector2(badge_inset, badge_inset)
        badge.mouse_filter = Control.MOUSE_FILTER_IGNORE
        seat.add_child(badge)
        _ignore_mouse(badge)

    _track_detail(row, player, state)
    return row


func _rebuild() -> void:
    if _last_state == null or not is_inside_tree():
        return
    refresh(_last_state)


## 行内控件全部忽略鼠标，保证整行任意位置都能接收悬停。
func _ignore_mouse(node: Node) -> void:
    if node is Control:
        node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    for child in node.get_children():
        _ignore_mouse(child)


## 座位详情需要整份视图：对战记录按 `participantIds` 过滤时要查玩家名。
func _track_detail(row: PanelContainer, player: Dictionary, state: GameUiState) -> void:
    if detail == null or not is_instance_valid(detail):
        return
    detail.track(row, {
        "detail_kind": "seat",
        "player": player,
        "hpCap": state.hp_cap,
        "view": state.view,
    })


## 预览入口：用调用方给出的 GameUiState 构建左栏，避免为预览复制页面的状态装配。
static func preview(opts: Dictionary = {}) -> Node:
    var rail := GamePlayersRail.new()
    var state: GameUiState = opts.get("state")
    if state != null:
        rail.refresh(state)
    return rail
