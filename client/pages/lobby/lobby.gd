extends Control
## 大厅：创建或凭房间码加入；好友管理与邀请分别放在浮层里，主页只留开局入口。

var _top: LobbyTopBar
var _actions: LobbyActions
var _bottom: LobbyBottomBar
var _friends: FriendsPanel
var _queue: QueuePanel
var _invite_popup: InvitePopup
## 已经发出过一次离开大厅的导航；避免选英雄与开局两个信号重复切场景。
var _leaving := false


func _ready() -> void:
    UILayout.safe_area(self)
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.error_received.connect(_on_error)
    GameStore.auth_changed.connect(_on_auth_changed)
    GameStore.account_changed.connect(_render_rank)
    GameStore.social_changed.connect(_render_friends_entry)
    GameStore.match_found.connect(_on_match_found)
    GameStore.hero_pick_started.connect(_on_hero_pick_started)
    GameStore.game_started.connect(_on_game_started)
    Net.disconnected.connect(_on_disconnected)
    if not Net.is_open():
        _top.status.text = tr("ui.lobby.not_connected")
    _show_room_closed_reason()
    _render_friends_entry()
    _render_rank()
    _maybe_local_join()


## 被移出或房间解散后回到大厅：在到达的页面上提示原因，读后清空。
func _show_room_closed_reason() -> void:
    if GameStore.room_closed_reason == "":
        return
    _top.status.text = (
        tr("ui.room.kicked_notice")
        if GameStore.room_closed_reason == "kicked"
        else tr("ui.room.dissolved_notice")
    )
    GameStore.room_closed_reason = ""


func _build() -> void:
    add_child(UIControls.background())

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-2xl"))
    add_child(margin)

    # 正常尺寸内容不超过一屏；键盘弹出或极窄时由这层兜底滚动，不裁切入口。
    var scroll := ScrollContainer.new()
    scroll.name = "LobbyScroll"
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    margin.add_child(scroll)

    var column := VBoxContainer.new()
    column.name = "LobbyColumn"
    column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    column.size_flags_vertical = Control.SIZE_EXPAND_FILL
    column.add_theme_constant_override("separation", UITheme.space("space-lg"))
    scroll.add_child(column)

    _top = LobbyTopBar.new()
    _top.set_identity(GameStore.display_name, GameStore.player_code)
    _top.rank_pressed.connect(_on_rank_pressed)
    column.add_child(_top)

    var center := CenterContainer.new()
    center.name = "LobbyCenter"
    center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    center.size_flags_vertical = Control.SIZE_EXPAND_FILL
    column.add_child(center)

    _actions = LobbyActions.new()
    _actions.match_pressed.connect(_on_match_pressed)
    _actions.create_pressed.connect(_on_create_pressed)
    _actions.join_pressed.connect(_on_join_pressed)
    center.add_child(_actions)

    # 好友与图鉴固定在底部两角，中间留给开局操作。
    _bottom = LobbyBottomBar.new()
    _bottom.friends_pressed.connect(_on_friends_pressed)
    _bottom.gallery_pressed.connect(_on_gallery_pressed)
    _bottom.account_pressed.connect(_on_account_pressed)
    _bottom.leaderboard_pressed.connect(_on_leaderboard_pressed)
    column.add_child(_bottom)

    _friends = FriendsPanel.new()
    _friends.closed.connect(_close_friends)
    add_child(_friends)

    # 匹配排队浮层盖住主页；邀请弹窗后挂载，保持邀请始终在最上层。
    _queue = QueuePanel.new()
    _queue.cancel_pressed.connect(_on_queue_cancel)
    add_child(_queue)

    # 邀请弹窗是独立组件：自己订阅社交状态并处理响应，大厅只负责提示文案。
    # 后挂载保证邀请始终盖在好友浮层之上。
    _invite_popup = InvitePopup.new()
    _invite_popup.responded.connect(_on_invite_responded)
    add_child(_invite_popup)


## 好友入口只显示待处理申请数，社交列表不参与主页排版。
func _render_friends_entry() -> void:
    var pending := GameStore.incoming_requests.size()
    _bottom.friends_button.text = (
        tr("ui.lobby.friends_entry_pending") % pending
        if pending > 0
        else tr("ui.lobby.friends_entry")
    )


func _on_create_pressed() -> void:
    _top.status.text = tr("ui.lobby.creating")
    Net.send(Protocol.MSG_CREATE_ROOM, {})


func _on_match_pressed() -> void:
    if not Net.is_open():
        _top.status.text = tr("ui.lobby.not_connected")
        return
    _top.status.text = tr("ui.lobby.matching")
    GameStore.join_queue()


func _on_queue_cancel() -> void:
    GameStore.leave_queue()
    _top.status.text = tr("ui.lobby.match_cancelled")


func _on_match_found(_seat: int) -> void:
    _top.status.text = tr("ui.lobby.match_found")


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


func _on_disconnected() -> void:
    GameStore.abandon_queue()
    _top.status.text = tr("ui.login.disconnected")


func _on_gallery_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_GALLERY)


func _on_account_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_ACCOUNT)


func _on_leaderboard_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_LEADERBOARD)


func _on_rank_pressed() -> void:
    if GameStore.player_id == "":
        return
    GameStore.open_profile(GameStore.player_id, UINavigation.PAGE_LOBBY)
    UINavigation.navigate(self, UINavigation.PAGE_PROFILE)


## 段位与积分随账号视图刷新；未收到账号视图时隐藏入口。
func _render_rank() -> void:
    _top.set_ranked(GameStore.my_tier(), GameStore.my_rating())


func _on_friends_pressed() -> void:
    _friends.open()


func _close_friends() -> void:
    _friends.close()


func _on_join_pressed() -> void:
    var code := _actions.code_input.text.strip_edges().to_upper()
    if code == "":
        _top.status.text = tr("ui.lobby.enter_room_code")
        return
    _top.status.text = tr("ui.lobby.joining") % code
    Net.send(Protocol.MSG_JOIN_ROOM, {"code": code})


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
    _actions.code_input.text = code
    _on_join_pressed()


func _on_local_server_failed(reason: String) -> void:
    _top.status.text = reason


func _on_invite_responded(_invite_id: String, accept: bool) -> void:
    if accept:
        _top.status.text = tr("ui.lobby.accepting_invite")


func _on_room_changed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_ROOM)


func _on_auth_changed() -> void:
    _top.set_identity(GameStore.display_name, GameStore.player_code)
    _top.status.text = tr("ui.lobby.reconnected")


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    _top.status.text = UIWording.error_text(code)
    _top.status.tooltip_text = UIWording.error_tooltip(code)
    if _friends.visible:
        _friends.show_error(code)
    if UINavigation.error_route(code) == UINavigation.ROUTE_LOGIN:
        GameStore.reset_session()
        UINavigation.navigate(self, UINavigation.PAGE_LOGIN)
