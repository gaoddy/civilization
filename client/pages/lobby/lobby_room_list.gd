class_name LobbyRoomList
extends PanelContainer
## 创建房间模式的中部座位列表：8 行座位、准备圆点、房主标记与踢人入口。
##
## 列表不写标题、不写准备计数与准备文字；准备状态只用圆点表达，
## 圆点 tooltip 给读屏与悬停保留说明。邀请入口只在有空位时存在。

signal kick_requested(player_id: String)
signal invite_pressed

const SEAT_COUNT := 8

var _box: VBoxContainer
var _rows: VBoxContainer
var _invite: Button
var _fill_hint: Label


func _init() -> void:
    name = "LobbyRoomList"
    add_theme_stylebox_override("panel", UITheme.panel_style("raised-tight"))

    _box = VBoxContainer.new()
    _box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(_box)

    _rows = VBoxContainer.new()
    _rows.name = "RoomRows"
    _rows.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _rows.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _box.add_child(_rows)

    _fill_hint = UIControls.label("", UITheme.font_size("font-caption"), false)
    _fill_hint.name = "RoomFillHint"
    _fill_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _fill_hint.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    _box.add_child(_fill_hint)


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
    _fill_hint.visible = host
    _fill_hint.text = tr("ui.room.bot_fill_hint") % maxi(0, SEAT_COUNT - int(seat_count))
    if not host:
        _fill_hint.text = ""


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
        var kick := UIControls.label(tr("ui.room.kick"), UITheme.font_size("font-caption"), false)
        kick.name = "RoomKick_%s" % player_id
        kick.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
        kick.mouse_filter = Control.MOUSE_FILTER_STOP
        kick.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
        kick.gui_input.connect(
            func(event: InputEvent) -> void:
                if event is InputEventMouseButton and event.pressed:
                    kick_requested.emit(player_id)
        )
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
    var style := StyleBoxFlat.new()
    style.set_corner_radius_all(int(side / 2.0))
    style.set_border_width_all(1)
    if ready:
        style.bg_color = UITheme.color("color-success")
        style.border_color = UITheme.color("color-success")
    else:
        style.bg_color = UITheme.alpha("color-surface", 0.0)
        style.border_color = UITheme.color("color-border-strong")
    dot.add_theme_stylebox_override("panel", style)
    return dot


## 邀请入口只在有空位时存在：满员时直接移除节点，不留不可见的按钮。
func _render_invite(seat_count: int) -> void:
    if seat_count >= SEAT_COUNT:
        if _invite != null and is_instance_valid(_invite):
            if _invite.get_parent() != null:
                _invite.get_parent().remove_child(_invite)
            _invite.free()
        _invite = null
        return
    if _invite == null:
        _invite = UIControls.button(tr("ui.lobby.room_invite"))
        _invite.name = "RoomInviteButton"
        _invite.custom_minimum_size.y = UITheme.stage_metric("lobby-room-invite-height")
        _invite.pressed.connect(func() -> void: invite_pressed.emit())
        _box.add_child(_invite)
    _box.move_child(_invite, maxi(0, _box.get_child_count() - 2))
