extends Control
## 大厅：内容区 7:3 两栏。左栏大厅图与底栏入口，右栏玩家卡、段位/排队卡与快捷操作。
##
## 好友抽屉、加入弹窗、提示弹窗与断线弹窗都挂在本页，公共组件自己管状态；
## 页面只负责拼装、导航与把服务端状态转给组件。

var _left: VBoxContainer
var _right: VBoxContainer
var _middle: VBoxContainer
var _hall: LobbyHallImage
var _player: LobbyPlayerCard
var _rank: LobbyRankCard
var _queue_card: LobbyQueueCard
var _quick: LobbyQuickActions
var _left_actions: LobbyLeftActions
var _friends: FriendsPanel
var _invite_popup: InvitePopup
var _join_dialog: LobbyJoinDialog
var _notice: LobbyNoticeDialog
var _connection: ConnectionDialog

var _connected := false
var _creating := false
var _joining := false
## 已经发出过一次离开大厅的导航；避免选英雄与开局两个信号重复切场景。
var _leaving := false


func _ready() -> void:
    UILayout.safe_area(self)
    _build()
    _connect_signals()
    _connected = Net.is_open()
    _render_identity()
    _render_rank()
    _render_friends()
    _render_actions()
    if not _connected:
        _connection.show_disconnected()
    _show_room_closed_reason()
    _maybe_local_join()


func _build() -> void:
    add_child(UIControls.background())

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-2xl"))
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

    _quick = LobbyQuickActions.new()
    _right.add_child(_quick)

    _friends = FriendsPanel.new()
    _friends.closed.connect(_close_friends)
    add_child(_friends)

    _join_dialog = LobbyJoinDialog.new()
    _join_dialog.join_requested.connect(_on_join_requested)
    add_child(_join_dialog)

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
    _quick.join_pressed.connect(_on_join_pressed)
    _left_actions.friends_pressed.connect(_on_friends_pressed)
    _left_actions.gallery_pressed.connect(_on_gallery_pressed)
    _left_actions.leaderboard_pressed.connect(_on_leaderboard_pressed)

    GameStore.room_changed.connect(_on_room_changed)
    GameStore.error_received.connect(_on_error)
    GameStore.auth_changed.connect(_on_auth_changed)
    GameStore.account_changed.connect(_render_rank)
    GameStore.social_changed.connect(_render_friends)
    GameStore.queue_changed.connect(_render_actions)
    GameStore.match_found.connect(_on_match_found)
    GameStore.hero_pick_started.connect(_on_hero_pick_started)
    GameStore.game_started.connect(_on_game_started)
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


func _render_actions() -> void:
    var queueing: bool = GameStore.queueing
    _quick.set_state(
        {
            "queueing": queueing,
            "connected": _connected,
            "creating": _creating,
            "joining": _joining,
        }
    )
    _queue_card.visible = queueing
    _rank.visible = not queueing


func _on_match_toggled(was_queueing: bool) -> void:
    if was_queueing:
        GameStore.leave_queue()
    else:
        if not _connected:
            return
        GameStore.join_queue()
    _render_actions()


func _on_match_found(_seat: int) -> void:
    _queue_card.set_matched()


func _on_create_pressed() -> void:
    if not _connected:
        return
    _creating = true
    _render_actions()
    Net.send(Protocol.MSG_CREATE_ROOM, {})


func _on_join_pressed() -> void:
    if not _connected:
        return
    _join_dialog.open_dialog()


func _on_join_requested(code: String) -> void:
    _joining = true
    _join_dialog.set_joining(true)
    _render_actions()
    Net.send(Protocol.MSG_JOIN_ROOM, {"code": code})


func _on_friends_pressed() -> void:
    _friends.open()


func _close_friends() -> void:
    _friends.close()


func _on_gallery_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_GALLERY)


func _on_leaderboard_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_LEADERBOARD)


func _on_disconnected() -> void:
    _connected = false
    GameStore.abandon_queue()
    _connection.show_disconnected()
    _render_actions()


func _on_connected() -> void:
    _connected = true
    _connection.hide_dialog()
    _render_actions()


func _on_reconnect_requested() -> void:
    _connection.show_reconnecting()
    Net.relogin()


func _on_room_changed() -> void:
    if _host_room_mode():
        return
    _creating = false
    _joining = false
    UINavigation.navigate(self, UINavigation.PAGE_ROOM)


## 自动匹配局没有房间等待页：`HeroPickStarted` 先到，由大厅直接进入对局页的选英雄阶段；
## 匹配房随后仍会下发 `GameStarted`，此时大厅已开始离开，不重复导航。
func _on_hero_pick_started() -> void:
    _navigate_to_game()


func _on_game_started() -> void:
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
    if _join_dialog.visible:
        _joining = false
        _join_dialog.set_joining(false)
        _join_dialog.show_error(code)
        _render_actions()
        return
    if UINavigation.error_route(code) == UINavigation.ROUTE_LOGIN:
        GameStore.reset_session()
        UINavigation.navigate(self, UINavigation.PAGE_LOGIN)
        return
    _creating = false
    _render_actions()
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


## 调试包：本地启动器给出的房间码到达后自动入房；房间码只消费一次。
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
    _join_dialog.open_dialog(code)
    _on_join_requested(code)


func _on_local_server_failed(reason: String) -> void:
    _notice.show_notice(tr("ui.lobby.notice_title"), reason)
