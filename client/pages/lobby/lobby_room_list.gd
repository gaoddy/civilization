class_name LobbyRoomList
extends PanelContainer
## 创建房间模式的中部座位列表：8 行座位、准备圆点、房主标记与踢人入口。
##
## 列表不写标题、不写准备计数与准备文字；准备状态只用圆点表达，
## 圆点 tooltip 给读屏与悬停保留说明。列表底部的固定区是补位提示与邀请区：
## 邀请行放「邀请好友」与（Steam 构建）「邀请 Steam 好友」，Steam 状态行在邀请行下方；
## 滚动只发生在座位行上。

signal kick_requested(player_id: String)
signal invite_pressed
signal steam_invite_pressed

const SEAT_COUNT := 8

var _box: VBoxContainer
var _rows: VBoxContainer
var _fill_hint: Label
var _invite_box: VBoxContainer
var _invite: Button
var _steam: Button
var _status: Label


func _init() -> void:
    name = "LobbyRoomList"
    add_theme_stylebox_override("panel", UITheme.panel_style("raised-tight"))

    _box = VBoxContainer.new()
    _box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(_box)

    _rows = VBoxContainer.new()
    _rows.name = "RoomRows"
    _rows.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _rows.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _rows.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    # 座位行放进滚动区：安全区或更小屏幕上列表让出高度，底部邀请区与房间操作始终可见。
    var scroll := ScrollContainer.new()
    scroll.name = "RoomScroll"
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
    scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    scroll.add_child(_rows)
    _box.add_child(scroll)

    _fill_hint = UIControls.label("", UITheme.font_size("font-caption"), false)
    _fill_hint.name = "RoomFillHint"
    _fill_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _fill_hint.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    _box.add_child(_fill_hint)


## 邀请区只在有空位时存在：满员时整块移除，不留不可见的按钮与状态行。
func _render_invite(seat_count: int) -> void:
    if seat_count >= SEAT_COUNT:
        _clear_invite_box()
        return
    _build_invite_box()
    _box.move_child(_invite_box, _box.get_child_count() - 1)


## 邀请行与 Steam 状态行一起重建；状态文本随重建清空，由下一次状态同步写入。
func _build_invite_box() -> void:
    if _invite_box != null:
        return
    _invite_box = VBoxContainer.new()
    _invite_box.name = "RoomInviteBox"
    _invite_box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _box.add_child(_invite_box)

    var row := HBoxContainer.new()
    row.name = "RoomInviteRow"
    row.add_theme_constant_override("separation", UITheme.space("space-sm"))
    _invite_box.add_child(row)

    _invite = _invite_button(tr("ui.lobby.room_invite"), "RoomInviteButton", "default")
    _invite.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _invite.pressed.connect(func() -> void: invite_pressed.emit())
    row.add_child(_invite)

    # Steam 构建多一个邀请入口，与好友邀请同一行；其余构建整页不出现 Steam 文案。
    if Platform.is_steam():
        _steam = _invite_button(tr("ui.room.invite_steam.entry"), "SteamInviteButton", "ghost")
        _steam.tooltip_text = tr("ui.room.invite_steam.unavailable")
        _steam.pressed.connect(func() -> void: steam_invite_pressed.emit())
        row.add_child(_steam)

    _status = UIControls.label("", UITheme.font_size("font-caption"), false)
    _status.name = "RoomInviteStatus"
    _status.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    _status.visible = false
    _invite_box.add_child(_status)


func _clear_invite_box() -> void:
    if _invite_box == null:
        return
    if _invite_box.get_parent() != null:
        _invite_box.get_parent().remove_child(_invite_box)
    _invite_box.free()
    _invite_box = null
    _invite = null
    _steam = null
    _status = null


## 邀请行按钮同高：高度取自令牌，三端按视觉高度固定。
func _invite_button(text: String, node_name: String, style: String) -> Button:
    var button := UIControls.styled_button(
        text, style, UITheme.stage_metric("lobby-room-invite-height")
    )
    button.name = node_name
    # 创建模式的邀请行与座位操作保持静态边框，不参与引导流光。
    FrameMaterial.set_guide_preference(button, false)
    return button


func set_steam_state(available: bool, hint: String) -> void:
    if _steam == null:
        return
    _steam.disabled = not available
    _steam.tooltip_text = hint


func set_status(text: String) -> void:
    if _status == null:
        return
    _status.text = text
    _status.visible = text != ""


func render(players: Array, self_id: String, host: bool, bot_fill: bool, seat_count: int) -> void:
    UIControls.clear(_rows)
    var by_seat := {}
    for player in players:
        by_seat[int(player.get("seat", 0))] = player
    for seat in SEAT_COUNT:
        var row := _row(seat)
        _rows.add_child(row)
        if by_seat.has(seat):
            _fill_player_row(row, by_seat[seat], self_id, host)
        else:
            _fill_empty_row(row)

    _render_invite(seat_count)
    _render_fill_hint(host, bot_fill, seat_count)


## 补位提示按开关给不同文案；满员或宾客视角不占高度。
func _render_fill_hint(host: bool, bot_fill: bool, seat_count: int) -> void:
    var empty := maxi(0, SEAT_COUNT - int(seat_count))
    _fill_hint.visible = host and empty > 0
    if not _fill_hint.visible:
        _fill_hint.text = ""
        return
    var template := "ui.room.bot_fill_hint" if bot_fill else "ui.room.bot_fill_off_hint"
    _fill_hint.text = tr(template) % empty


func _row(seat: int) -> PanelContainer:
    var row := PanelContainer.new()
    row.name = "RoomRow%d" % seat
    row.custom_minimum_size.y = UITheme.stage_metric("lobby-room-row-height")
    row.clip_contents = true
    row.add_theme_stylebox_override("panel", UITheme.panel_style("row-compact"))
    return row


func _fill_player_row(row: PanelContainer, player: Dictionary, self_id: String, host: bool) -> void:
    var player_id := str(player.get("playerId", ""))
    var self_row := player_id == self_id
    if self_row:
        row.add_theme_stylebox_override("panel", UITheme.panel_style("accent-row-compact"))

    var line := HBoxContainer.new()
    line.add_theme_constant_override("separation", UITheme.space("space-sm"))
    row.add_child(line)

    var display_name := str(player.get("displayName", player_id))
    var avatar := PanelContainer.new()
    var avatar_side := UITheme.metric("avatar-sm")
    avatar.custom_minimum_size = Vector2(avatar_side, avatar_side)
    avatar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    var avatar_style := UITheme.panel_style("accent")
    avatar_style.set_content_margin_all(0)
    avatar.add_theme_stylebox_override("panel", avatar_style)
    var center := CenterContainer.new()
    avatar.add_child(center)
    var initial := UIControls.label(display_name.substr(0, 1), UITheme.font_size("font-caption"), false)
    center.add_child(initial)
    line.add_child(avatar)

    var name_label := UIControls.label(display_name, UITheme.font_size("font-caption"), false)
    name_label.name = "RoomName_%s" % player_id
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    line.add_child(name_label)

    if bool(player.get("isHost", false)):
        var pill := UIControls.pill_label(tr("ui.room.host"), "accent", "font-caption")
        pill.name = "RoomHost_%s" % player_id
        pill.custom_minimum_size.y = UITheme.stage_metric("lobby-room-pill-height")
        pill.size_flags_vertical = Control.SIZE_SHRINK_CENTER
        line.add_child(pill)

    var ready := bool(player.get("ready", false))
    var dot := _ready_dot(player_id, ready)
    line.add_child(dot)

    if host and not self_row:
        var kick := UIControls.styled_button(
            tr("ui.room.kick"), "micro", UITheme.stage_metric("lobby-room-kick-height")
        )
        kick.name = "RoomKick_%s" % player_id
        # 座位行高度固定，移除入口按令牌取视觉高度，不撑高整行。
        kick.size_flags_vertical = Control.SIZE_SHRINK_CENTER
        kick.pressed.connect(func() -> void: kick_requested.emit(player_id))
        FrameMaterial.set_guide_preference(kick, false)
        line.add_child(kick)


func _fill_empty_row(row: PanelContainer) -> void:
    var hint := UIControls.empty_hint(tr("ui.room.seat_empty"))
    hint.add_theme_font_size_override("font_size", UITheme.font_size("font-caption"))
    row.add_child(hint)


## 准备圆点：已准备实心，未准备空心；状态只在圆点与 tooltip 里表达。
func _ready_dot(player_id: String, ready: bool) -> Panel:
    var side := UITheme.stage_metric("lobby-room-ready-dot")
    var dot := Panel.new()
    dot.name = "RoomReady_%s" % player_id
    dot.custom_minimum_size = Vector2(side, side)
    dot.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    dot.mouse_filter = Control.MOUSE_FILTER_IGNORE
    dot.tooltip_text = tr("ui.room.seat_ready") if ready else tr("ui.room.seat_unready")
    dot.add_theme_stylebox_override("panel", UITheme.ready_dot_style(side, ready))
    return dot
