extends Control
## 房间等待区：8 座位矩阵、准备、房主开始与补位、离开确认与 Steam 邀请。
##
## 本页不显示房间码，也不展示任何英雄候选；选英雄由对局页承担。
## 开始后服务端下发 `HeroPickStarted`，页面据此离开大厅并导航到对局页。

const SEAT_COUNT := 8

var _title: Label
var _meta: Label
var _grid: RoomSeatGrid
var _bot_fill_hint: Label
var _invite: RoomInviteRow
var _actions: RoomActionBar
var _leave_dialog: ConfirmationDialog

## 当前由状态计算写入状态行的文案；用于只清理自己写入的提示，不覆盖邀请/错误提示。
var _state_status := ""
## 状态行正在显示服务端错误：此期间 `_sync_state()` 不覆盖，下一次房间状态变化解除。
var _status_is_error := false

## Steam 邀请请求进行中：按钮置灰，避免重复创建 token 与大厅。
var _steam_invite_busy := false
## 大厅创建失败后保持置灰，提示改用房间码。
var _steam_invite_blocked := false


func _ready() -> void:
    UILayout.safe_area(self)
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.hero_pick_started.connect(_on_hero_pick_started)
    GameStore.room_closed.connect(_on_room_closed)
    GameStore.error_received.connect(_on_error)
    GameStore.social_changed.connect(_render_invites)
    GameStore.invite_result_received.connect(_on_invite_result)
    GameStore.room_invite_received.connect(_on_room_invite_received)
    GameStore.steam_invite_rejected.connect(_on_steam_invite_rejected)
    # 断线不产生房间或视图消息：直接订阅连接状态，否则「未连接」永远不刷新。
    Net.connected.connect(_on_connection_changed)
    Net.disconnected.connect(_on_connection_changed)
    _refresh()


func _build() -> void:
    add_child(UIControls.background())

    var panel := UIControls.card_panel("surface")
    panel.name = "RoomPanel"
    panel.set_anchors_preset(Control.PRESET_FULL_RECT)
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(panel)

    var column := VBoxContainer.new()
    column.name = "RoomColumn"
    column.mouse_filter = Control.MOUSE_FILTER_IGNORE
    column.add_theme_constant_override("separation", UITheme.space("space-lg"))
    panel.add_child(UIControls.margin(column, UITheme.space("space-lg")))

    column.add_child(_top_bar())

    _grid = RoomSeatGrid.new()
    column.add_child(_grid)

    # 补位提示只在房主可见：触屏端没有 hover，不能只依赖按钮 tooltip。
    _bot_fill_hint = UIControls.label("", UITheme.font_size("font-caption"), false)
    _bot_fill_hint.name = "BotFillHint"
    _bot_fill_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
    _bot_fill_hint.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    column.add_child(_bot_fill_hint)

    _invite = RoomInviteRow.new()
    _invite.invite_pressed.connect(_on_invite_pressed)
    column.add_child(_invite)

    _actions = RoomActionBar.new()
    _actions.ready_pressed.connect(_on_ready_pressed)
    _actions.start_pressed.connect(_on_start_pressed)
    _actions.bot_fill_pressed.connect(_on_bot_fill_pressed)
    _actions.leave_pressed.connect(_on_leave_pressed)
    _actions.dissolve_pressed.connect(_on_dissolve_pressed)
    _actions.steam_invite_pressed.connect(_on_steam_invite_pressed)
    column.add_child(_actions)

    # 离开确认在构建时创建一次，避免首次弹窗时才入树触发布局重挂。
    _leave_dialog = ConfirmationDialog.new()
    _leave_dialog.name = "LeaveConfirm"
    _leave_dialog.title = tr("ui.room.leave")
    _leave_dialog.dialog_text = tr("ui.room.leave_confirm")
    _leave_dialog.ok_button_text = tr("ui.room.leave_confirm_action")
    _leave_dialog.cancel_button_text = tr("ui.lobby.close")
    _leave_dialog.confirmed.connect(_confirm_leave)
    # 挂到 PageBody 而不是页面根：避免被安全区收纳，保持安全区内只有 RoomPanel。
    var safe := UILayout.safe_area(self)
    safe.get_parent().add_child(_leave_dialog)


## 顶部：标题与「已准备 N/8 · 补位开/关」。
func _top_bar() -> HBoxContainer:
    var bar := HBoxContainer.new()
    bar.name = "RoomTopBar"
    bar.add_theme_constant_override("separation", UITheme.space("space-md"))

    _title = UIControls.label(tr("ui.room.title"), UITheme.font_size("font-title"), false)
    _title.name = "RoomTitle"
    _title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    bar.add_child(_title)

    _meta = UIControls.label("", UITheme.font_size("font-body-sm"), false)
    _meta.name = "RoomMeta"
    _meta.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    bar.add_child(_meta)
    return bar


func _refresh() -> void:
    _render_seats()
    _render_invites()
    _sync_state()


func _render_seats() -> void:
    _grid.render(GameStore.room_players, GameStore.player_id)
    var can_manage := GameStore.am_host() and GameStore.phase == Protocol.PHASE_LOBBY
    for child in _grid.get_children():
        var cell := child as RoomSeatCell
        if cell != null:
            cell.set_kick_handler(_on_kick_pressed, can_manage and not cell.is_self)


## 好友邀请入口的渲染由 RoomInviteRow 负责。
func _render_invites() -> void:
    if _invite == null:
        return
    _invite.render(GameStore.friends)


func _sync_state() -> void:
    if not Net.is_open():
        _steam_invite_busy = false

    var ready_count := 0
    for player in GameStore.room_players:
        if bool(player.get("ready", false)):
            ready_count += 1
    var human_count := GameStore.room_players.size()
    var connected := Net.is_open()
    var is_host := GameStore.am_host()
    var bot_fill := GameStore.room_bot_fill
    var bot_count := maxi(0, SEAT_COUNT - human_count)

    var start_reason := ""
    if not connected:
        start_reason = tr("ui.room.not_connected")
    elif ready_count < human_count:
        start_reason = tr("ui.room.start_waiting_ready")
    elif not bot_fill and human_count < SEAT_COUNT:
        start_reason = tr("ui.room.start_need_players") % (SEAT_COUNT - human_count)
    var can_start := start_reason == ""

    _meta.text = "%s · %s %s" % [
        tr("ui.room.ready_progress") % ready_count,
        tr("ui.room.bot_fill"),
        tr("ui.room.bot_fill_on") if bot_fill else tr("ui.room.bot_fill_off"),
    ]
    _bot_fill_hint.visible = is_host
    _bot_fill_hint.text = tr("ui.room.bot_fill_hint") % bot_count
    _actions.set_state({
        "ready": GameStore.my_ready(),
        "is_host": is_host,
        "connected": connected,
        "ready_count": ready_count,
        "human_count": human_count,
        "bot_fill": bot_fill,
        "can_start": can_start,
        "start_reason": start_reason,
        "bot_count": bot_count,
    })

    # 触屏端看不到按钮 tooltip：未连接或无法开始时把原因写到状态行。
    if not _status_is_error:
        var state_status := ""
        if not connected:
            state_status = tr("ui.room.not_connected")
        elif not can_start and start_reason != "":
            state_status = start_reason
        if state_status != "":
            _actions.status.text = state_status
            _state_status = state_status
        elif _actions.status.text == _state_status:
            _actions.status.text = ""
            _state_status = ""
    _sync_steam_invite()


## Steam 邀请入口：已入房、已连接、等待区且大厅可用时才可点击。
func _sync_steam_invite() -> void:
    if _actions.steam_invite == null:
        return
    var available := (
        not _steam_invite_busy
        and not _steam_invite_blocked
        and Platform.lobby_available()
        and Net.is_open()
        and GameStore.phase == Protocol.PHASE_LOBBY
    )
    _actions.set_steam_available(
        available,
        tr("ui.room.invite_steam.unavailable") if not available else "",
    )


func _on_ready_pressed() -> void:
    Net.send(Protocol.MSG_READY, {"ready": not GameStore.my_ready()})


func _on_start_pressed() -> void:
    GameStore.request_start()


func _on_bot_fill_pressed() -> void:
    GameStore.set_bot_fill(not GameStore.room_bot_fill)


## 离开前二次确认；对话框在构建时已创建，这里只弹出。
func _on_leave_pressed() -> void:
    _leave_dialog.popup_centered()


func _confirm_leave() -> void:
    Net.send(Protocol.MSG_LEAVE_ROOM, {})
    GameStore.reset_room()
    UINavigation.navigate(self, UINavigation.PAGE_LOBBY)


func _on_kick_pressed(player_id: String) -> void:
    Net.send(Protocol.MSG_KICK_MEMBER, {"targetPlayerId": player_id})


func _on_dissolve_pressed() -> void:
    Net.send(Protocol.MSG_DISSOLVE_ROOM, {})


func _on_invite_pressed(player_id: String, friend_name: String) -> void:
    Net.send(Protocol.MSG_INVITE_TO_ROOM, {"friendId": player_id})
    _actions.status.text = tr("ui.room.invite_sent") % friend_name


func _on_invite_result() -> void:
    if bool(GameStore.last_invite_result.get("accepted", false)):
        _actions.status.text = tr("ui.room.invite_accepted")
    else:
        _actions.status.text = tr("ui.room.invite_declined")


func _on_steam_invite_pressed() -> void:
    if _steam_invite_busy:
        return
    _steam_invite_busy = true
    _actions.status.text = tr("ui.room.invite_steam.creating")
    _sync_steam_invite()
    GameStore.create_room_invite()


## 服务端下发 token 后才创建 Steam 大厅：大厅只承载邀请，房间仍以服务端为准。
func _on_room_invite_received() -> void:
    if _actions.steam_invite == null:
        return
    var token := str(GameStore.room_invite.get("token", ""))
    var room_code := str(GameStore.room_invite.get("roomCode", GameStore.room_code))
    var expires_at := str(GameStore.room_invite.get("expiresAt", ""))
    if token == "":
        _steam_invite_busy = false
        _sync_steam_invite()
        return
    var created: bool = await Platform.create_lobby(room_code, token, expires_at)
    if not is_inside_tree():
        return
    _steam_invite_busy = false
    if created:
        Platform.invite_friends()
        _actions.status.text = tr("ui.room.invite_steam.sent")
    else:
        _steam_invite_blocked = true
        _actions.status.text = tr("ui.room.invite_steam.unavailable")
    _sync_steam_invite()


func _on_steam_invite_rejected(_reason: String) -> void:
    _actions.status.text = tr("ui.room.invite_steam.already_in_room")


## 开始后进入对局页的选英雄阶段；邀请 token 已无意义，先离开大厅。
func _on_hero_pick_started() -> void:
    if not is_inside_tree():
        return
    Platform.leave_lobby()
    UINavigation.navigate(self, UINavigation.PAGE_GAME)


func _on_connection_changed() -> void:
    _status_is_error = false
    _sync_state()


func _on_room_changed() -> void:
    _status_is_error = false
    _refresh()


## 被移出或房间解散：先写原因文案，再回大厅；房间状态已由 GameStore 清空。
func _on_room_closed() -> void:
    if GameStore.room_closed_reason == "kicked":
        _actions.status.text = tr("ui.room.kicked_notice")
    else:
        _actions.status.text = tr("ui.room.dissolved_notice")
    UINavigation.navigate(self, UINavigation.PAGE_LOBBY)


func _error_route(code: String) -> String:
    return UINavigation.error_route(code)


func _on_error() -> void:
    _steam_invite_busy = false
    var code := str(GameStore.last_error.get("code", ""))
    match _error_route(code):
        UINavigation.ROUTE_LOBBY:
            GameStore.reset_room()
            UINavigation.navigate(self, UINavigation.PAGE_LOBBY)
            return
        UINavigation.ROUTE_LOGIN:
            GameStore.reset_session()
            UINavigation.navigate(self, UINavigation.PAGE_LOGIN)
            return
    # 请求被拒时没有新的 RoomUpdate：先按当前状态重算按钮，否则会一直停在禁用。
    _sync_state()
    # 错误文案最后写入，并在下一次房间状态变化前保持；桌面端 tooltip 同步更新。
    _actions.status.text = UIWording.error_text(code)
    _actions.status.tooltip_text = UIWording.error_tooltip(code)
    _state_status = _actions.status.text
    _status_is_error = true
