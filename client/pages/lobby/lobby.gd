extends Control
## 大厅：内容区 7:3 两栏。左栏大厅图与底栏入口，右栏玩家卡、段位/排队卡与快捷操作。
##
## 好友抽屉、提示弹窗与断线弹窗都挂在本页，公共组件自己管状态；
## 页面只负责拼装、导航与把服务端状态转给组件。

var _left: VBoxContainer
var _right: VBoxContainer
var _middle: VBoxContainer
var _hall: LobbyHallImage
var _player: LobbyPlayerCard
var _rank: LobbyRankCard
var _queue_card: LobbyQueueCard
var _room_list: LobbyRoomList
var _room_actions: LobbyRoomActions
var _quick: LobbyQuickActions
var _left_actions: LobbyLeftActions
var _friends: FriendsPanel
var _invite_popup: InvitePopup
var _notice: LobbyNoticeDialog
var _connection: ConnectionDialog

var _connected := false
var _creating := false
var _joining := false
## 匹配成功到进入对局之间保留排队卡，显示成功文案。
var _matched := false
## 已经发出过一次离开大厅的导航；避免选英雄与开局两个信号重复切场景。
var _leaving := false

## Steam 邀请请求进行中与大厅创建失败后保持置灰：与房间等待区同一套流程。
var _steam_invite_busy := false
var _steam_invite_blocked := false


func _ready() -> void:
    UILayout.safe_area(self, PageBackgrounds.surface("lobby"))
    _build()
    FrameMaterial.dress_page(self)
    _connect_signals()
    _connected = Net.is_open()
    _render_identity()
    _render_rank()
    _render_friends()
    _apply_mode()
    if not _connected:
        _connection.show_disconnected()
    _show_room_closed_reason()
    _maybe_local_join()


func _build() -> void:
    add_child(UIControls.background(PageBackgrounds.ambient("lobby")))

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override(
            "margin_" + side, UITheme.stage_metric("lobby-page-margin")
        )
    add_child(margin)

    var columns := HBoxContainer.new()
    columns.name = "LobbyColumns"
    columns.add_theme_constant_override("separation", UITheme.space("space-2xl"))
    margin.add_child(columns)

    _left = _column("LobbyLeft", 7)
    columns.add_child(_left)
    _hall = LobbyHallImage.new()
    _hall.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _left.add_child(_hall)
    _left_actions = LobbyLeftActions.new()
    _left.add_child(_left_actions)

    _right = _column("LobbyRight", 3)
    columns.add_child(_right)
    _player = LobbyPlayerCard.new()
    _right.add_child(_player)

    _middle = VBoxContainer.new()
    _middle.name = "LobbyMiddle"
    _middle.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _right.add_child(_middle)
    _rank = LobbyRankCard.new()
    _rank.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _middle.add_child(_rank)
    _queue_card = LobbyQueueCard.new()
    _queue_card.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _middle.add_child(_queue_card)

    _room_list = LobbyRoomList.new()
    _room_list.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _room_list.visible = false
    _middle.add_child(_room_list)

    _quick = LobbyQuickActions.new()
    _right.add_child(_quick)

    _room_actions = LobbyRoomActions.new()
    _room_actions.visible = false
    _right.add_child(_room_actions)

    _friends = FriendsPanel.new()
    _friends.closed.connect(_close_friends)
    add_child(_friends)

    _notice = LobbyNoticeDialog.new()
    add_child(_notice)

    _connection = ConnectionDialog.new()
    _connection.reconnect_requested.connect(_on_reconnect_requested)
    add_child(_connection)

    # 邀请弹窗固定在页面最上层，盖住其余浮层。
    _invite_popup = InvitePopup.new()
    add_child(_invite_popup)



func _column(node_name: String, ratio: int) -> VBoxContainer:
    var column := VBoxContainer.new()
    column.name = node_name
    column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    column.size_flags_stretch_ratio = ratio
    column.add_theme_constant_override("separation", UITheme.space("space-md"))
    return column


func _connect_signals() -> void:
    _quick.match_toggled.connect(_on_match_toggled)
    _quick.create_pressed.connect(_on_create_pressed)
    _left_actions.friends_pressed.connect(_on_friends_pressed)
    _left_actions.gallery_pressed.connect(_on_gallery_pressed)
    _left_actions.account_pressed.connect(_on_account_pressed)
    _left_actions.leaderboard_pressed.connect(_on_leaderboard_pressed)
    _room_list.invite_pressed.connect(_on_room_invite_pressed)
    _room_list.kick_requested.connect(_on_kick_pressed)
    _room_list.steam_invite_pressed.connect(_on_steam_invite_pressed)
    _room_actions.start_pressed.connect(_on_start_pressed)
    _room_actions.dissolve_pressed.connect(_on_dissolve_pressed)

    GameStore.room_changed.connect(_on_room_changed)
    GameStore.room_closed.connect(_on_room_closed)
    GameStore.error_received.connect(_on_error)
    GameStore.auth_changed.connect(_on_auth_changed)
    GameStore.account_changed.connect(_render_rank)
    GameStore.social_changed.connect(_render_friends)
    GameStore.queue_changed.connect(_apply_mode)
    GameStore.match_found.connect(_on_match_found)
    GameStore.room_invite_received.connect(_on_room_invite_received)
    GameStore.steam_invite_rejected.connect(_on_steam_invite_rejected)
    GameStore.hero_pick_started.connect(_on_hero_pick_started)
    GameStore.game_started.connect(_on_game_started)
    GameStore.game_resumed.connect(_on_game_resumed)
    Net.disconnected.connect(_on_disconnected)
    Net.connected.connect(_on_connected)


## 建房后房主留在大厅右栏，其余入房路径进入房间等待页。
func _host_room_mode() -> bool:
    return (
        GameStore.phase == Protocol.PHASE_LOBBY
        and not GameStore.room_players.is_empty()
        and GameStore.am_host()
    )


func _render_identity() -> void:
    _player.set_identity(GameStore.display_name, GameStore.player_code)


## 段位与积分随账号视图刷新；未收到账号视图时显示空段位。
func _render_rank() -> void:
    _rank.set_ranked(GameStore.my_tier(), GameStore.my_rating())


## 好友入口只显示在线好友数。
func _render_friends() -> void:
    var online := 0
    for friend in GameStore.friends:
        if str(friend.get("status", "")) == "online":
            online += 1
    _left_actions.set_online_friends(online)


## 默认模式与创建模式的互斥显示：创建模式只留座位列表与房间操作。
func _apply_mode() -> void:
    var room_mode := _host_room_mode()
    _room_list.visible = room_mode
    _room_actions.visible = room_mode
    _quick.visible = not room_mode
    var queueing: bool = (GameStore.queueing or _matched) and not room_mode
    _queue_card.visible = queueing
    _rank.visible = not room_mode and not queueing
    if room_mode:
        _render_room()
        return
    _quick.set_state(
        {
            "queueing": GameStore.queueing,
            "connected": _connected,
            "creating": _creating,
            "joining": _joining,
            "matched": _matched,
        }
    )


func _render_room() -> void:
    var players: Array = GameStore.room_players
    var is_host: bool = GameStore.am_host()
    _room_list.render(
        players, GameStore.player_id, is_host, GameStore.room_bot_fill, players.size()
    )
    var ready_count := 0
    for player in players:
        if bool(player.get("ready", false)):
            ready_count += 1
    var start_reason := ""
    if not _connected:
        start_reason = tr("ui.room.not_connected")
    elif ready_count < players.size():
        start_reason = tr("ui.room.start_waiting_ready")
    elif not GameStore.room_bot_fill and players.size() < LobbyRoomList.SEAT_COUNT:
        start_reason = tr("ui.room.start_need_players") % (LobbyRoomList.SEAT_COUNT - players.size())
    _room_actions.set_state(
        {
            "connected": _connected,
            "can_start": start_reason == "",
            "start_reason": start_reason,
        }
    )
    _sync_steam_invite()


func _on_match_toggled(was_queueing: bool) -> void:
    if was_queueing:
        GameStore.leave_queue()
    else:
        if not _connected:
            return
        GameStore.join_queue()
    _apply_mode()


func _on_match_found(_seat: int) -> void:
    _matched = true
    _queue_card.set_matched()
    _apply_mode()


func _on_create_pressed() -> void:
    if not _connected:
        return
    _creating = true
    _apply_mode()
    Net.send(Protocol.MSG_CREATE_ROOM, {})


func _on_friends_pressed() -> void:
    _friends.open()


func _close_friends() -> void:
    _friends.close()


func _on_gallery_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_GALLERY)


func _on_account_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_ACCOUNT)


func _on_leaderboard_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_LEADERBOARD)


func _on_disconnected() -> void:
    _connected = false
    _matched = false
    _creating = false
    _joining = false
    GameStore.abandon_queue()
    _reset_steam_invite()
    _connection.show_disconnected()
    _apply_mode()


func _on_connected() -> void:
    _connected = true
    _connection.hide_dialog()
    _apply_mode()


func _on_reconnect_requested() -> void:
    _connection.show_reconnecting()
    Net.relogin()


func _on_room_changed() -> void:
    _creating = false
    _joining = false
    _matched = false
    if GameStore.room_players.is_empty():
        _apply_mode()
        return
    if _host_room_mode():
        _apply_mode()
        return
    UINavigation.navigate(self, UINavigation.PAGE_ROOM)


## 被移出或房间解散：房间状态已清空，回到默认模式并弹出原因。
func _on_room_closed() -> void:
    _creating = false
    _joining = false
    _matched = false
    _reset_steam_invite()
    _apply_mode()
    _show_room_closed_reason()


func _on_start_pressed() -> void:
    GameStore.request_start()


func _on_dissolve_pressed() -> void:
    Net.send(Protocol.MSG_DISSOLVE_ROOM, {})


func _on_kick_pressed(player_id: String) -> void:
    Net.send(Protocol.MSG_KICK_MEMBER, {"targetPlayerId": player_id})


func _on_room_invite_pressed() -> void:
    _friends.open_invite()


## Steam 邀请入口与房间等待区同一套流程：先向服务端请求 token，再创建大厅并打开好友邀请。
func _sync_steam_invite() -> void:
    var available := (
        not _steam_invite_busy
        and not _steam_invite_blocked
        and Platform.lobby_available()
        and Net.is_open()
        and GameStore.phase == Protocol.PHASE_LOBBY
    )
    _room_list.set_steam_state(
        available,
        tr("ui.room.invite_steam.unavailable") if not available else "",
    )


func _on_steam_invite_pressed() -> void:
    if _steam_invite_busy or not _host_room_mode():
        return
    _steam_invite_busy = true
    _room_list.set_status(tr("ui.room.invite_steam.creating"))
    _sync_steam_invite()
    GameStore.create_room_invite()


func _on_room_invite_received() -> void:
    if not _host_room_mode():
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
        _room_list.set_status(tr("ui.room.invite_steam.sent"))
    else:
        _steam_invite_blocked = true
        _room_list.set_status(tr("ui.room.invite_steam.unavailable"))
    _sync_steam_invite()


func _on_steam_invite_rejected(_reason: String) -> void:
    if not _host_room_mode():
        return
    _room_list.set_status(tr("ui.room.invite_steam.already_in_room"))


## 离开创建模式时清空邀请状态：token 已失效，状态行也不再延续。
func _reset_steam_invite() -> void:
    _steam_invite_busy = false
    _steam_invite_blocked = false
    _room_list.set_status("")


## 自动匹配局没有房间等待页：`HeroPickStarted` 先到，由大厅直接进入对局页的选英雄阶段；
## 匹配房随后仍会下发 `GameStarted`，此时大厅已开始离开，不重复导航。
func _on_hero_pick_started() -> void:
    _navigate_to_game()


func _on_game_started() -> void:
    _navigate_to_game()


## 新登录接管：快照在场景就绪前到达时，登录页可能已先切到大厅，这里补导航。
func _on_game_resumed() -> void:
    _navigate_to_game()


func _navigate_to_game() -> void:
    if _leaving or not is_inside_tree():
        return
    _leaving = true
    UINavigation.navigate(self, UINavigation.PAGE_GAME)


func _on_auth_changed() -> void:
    _render_identity()


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    # 认证类错误优先于其余提示：会话已失效时留在本页没有意义。
    if UINavigation.error_route(code) == UINavigation.ROUTE_LOGIN:
        _joining = false
        _creating = false
        GameStore.reset_session()
        UINavigation.navigate(self, UINavigation.PAGE_LOGIN)
        return
    # 好友抽屉打开时错误回填到抽屉状态行，不再叠一层提示弹窗。
    if _friends.visible:
        _friends.show_error(code)
        return
    _creating = false
    if UINavigation.error_route(code) == UINavigation.ROUTE_LOBBY:
        GameStore.reset_room()
    _apply_mode()
    _notice.show_notice(tr("ui.lobby.notice_title"), UIWording.error_text(code))


## 被移出或房间解散后回到大厅：在提示弹窗里展示原因，读后清空。
func _show_room_closed_reason() -> void:
    if GameStore.room_closed_reason == "":
        return
    _notice.show_notice(
        tr("ui.lobby.notice_title"),
        (
            tr("ui.room.kicked_notice")
            if GameStore.room_closed_reason == "kicked"
            else tr("ui.room.dissolved_notice")
        ),
    )
    GameStore.room_closed_reason = ""


## 调试包：本地启动器给出的房间码到达后直接入房；房间码只消费一次，不经过任何 UI。
func _maybe_local_join() -> void:
    if not LocalServer.enabled or not LocalServer.auto_flow:
        return
    var code := LocalServer.consume_room_code()
    if code != "":
        _join_local_room(code)
        return
    LocalServer.room_code_ready.connect(_on_local_room_code_ready, CONNECT_ONE_SHOT)
    LocalServer.failed.connect(_on_local_server_failed, CONNECT_ONE_SHOT)


func _on_local_room_code_ready(_code: String) -> void:
    var code := LocalServer.consume_room_code()
    if code != "":
        _join_local_room(code)


func _join_local_room(code: String) -> void:
    _joining = true
    _apply_mode()
    Net.send(Protocol.MSG_JOIN_ROOM, {"code": code})


func _on_local_server_failed(reason: String) -> void:
    _notice.show_notice(tr("ui.lobby.notice_title"), reason)
