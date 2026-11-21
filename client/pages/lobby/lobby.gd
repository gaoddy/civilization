extends Control
## 大厅：创建或凭房间码加入；好友管理与邀请分别放在浮层里，主页只留开局入口。

var _top: LobbyTopBar
var _actions: LobbyActions
var _bottom: LobbyBottomBar
var _friends: FriendsPanel
var _invite_popup: InvitePopup


func _ready() -> void:
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.error_received.connect(_on_error)
    GameStore.auth_changed.connect(_on_auth_changed)
    GameStore.social_changed.connect(_render_friends_entry)
    if not Net.is_open():
        _top.status.text = tr("ui.lobby.not_connected")
    _show_room_closed_reason()
    _render_friends_entry()
    UILayout.safe_area(self)


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
    column.add_child(_top)

    var center := CenterContainer.new()
    center.name = "LobbyCenter"
    center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    center.size_flags_vertical = Control.SIZE_EXPAND_FILL
    column.add_child(center)

    _actions = LobbyActions.new()
    _actions.create_pressed.connect(_on_create_pressed)
    _actions.join_pressed.connect(_on_join_pressed)
    center.add_child(_actions)

    # 好友与图鉴固定在底部两角，中间留给开局操作。
    _bottom = LobbyBottomBar.new()
    _bottom.friends_pressed.connect(_on_friends_pressed)
    _bottom.gallery_pressed.connect(_on_gallery_pressed)
    column.add_child(_bottom)

    _friends = FriendsPanel.new()
    _friends.closed.connect(_close_friends)
    add_child(_friends)

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


func _on_gallery_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_GALLERY)


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
