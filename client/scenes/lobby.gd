extends Control
## 大厅：创建或凭房间码加入，并管理玩家编号、好友与好友邀请。

var _status: Label
var _code_input: LineEdit
var _greeting: Label
var _add_input: LineEdit
var _incoming_box: VBoxContainer
var _friends_box: VBoxContainer
var _recent_box: VBoxContainer
var _outgoing: Label
var _invite_popup: InvitePopup


func _ready() -> void:
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.error_received.connect(_on_error)
    GameStore.auth_changed.connect(_on_auth_changed)
    GameStore.social_changed.connect(_render_social)
    if not Net.is_open():
        _status.text = tr("ui.lobby.not_connected")
    _render_social()


func _build() -> void:
    var background := ColorRect.new()
    background.color = Color("10141c")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(background)

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, 32)
    add_child(margin)

    var scroll := ScrollContainer.new()
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    margin.add_child(scroll)

    var box := VBoxContainer.new()
    box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    box.add_theme_constant_override("separation", 12)
    scroll.add_child(box)

    box.add_child(UI.label(tr("ui.lobby.title"), 28))
    _greeting = UI.label(tr("ui.lobby.logged_in") % [GameStore.display_name, GameStore.player_code], 14)
    box.add_child(_greeting)
    box.add_child(HSeparator.new())

    var create := UI.button(tr("ui.lobby.create_room"))
    create.pressed.connect(_on_create_pressed)
    box.add_child(create)

    var gallery := UI.button(tr("ui.gallery.entry"))
    gallery.name = "GalleryButton"
    gallery.pressed.connect(_on_gallery_pressed)
    box.add_child(gallery)

    box.add_child(UI.label(tr("ui.lobby.join_section"), 16))
    var join_row := HBoxContainer.new()
    join_row.add_theme_constant_override("separation", 8)
    _code_input = LineEdit.new()
    _code_input.placeholder_text = tr("ui.lobby.room_code_placeholder")
    _code_input.custom_minimum_size = Vector2(220, 0)
    join_row.add_child(_code_input)
    var join := UI.button(tr("ui.lobby.join"))
    join.pressed.connect(_on_join_pressed)
    join_row.add_child(join)
    box.add_child(join_row)

    _status = UI.label("", 14)
    box.add_child(_status)

    box.add_child(HSeparator.new())
    _build_friends(box)

    # 邀请弹窗是独立组件：自己订阅社交状态并处理响应，大厅只负责提示文案。
    _invite_popup = InvitePopup.new()
    _invite_popup.responded.connect(_on_invite_responded)
    add_child(_invite_popup)


## 好友区：按编号添加、处理收到的申请、查看与删除好友、从最近队友回加。
func _build_friends(parent: VBoxContainer) -> void:
    parent.add_child(UI.section_title(tr("ui.lobby.friends_section")))

    var add_row := HBoxContainer.new()
    add_row.add_theme_constant_override("separation", 8)
    _add_input = LineEdit.new()
    _add_input.name = "FriendCodeInput"
    _add_input.placeholder_text = tr("ui.lobby.add_friend_placeholder")
    _add_input.custom_minimum_size = Vector2(220, 0)
    add_row.add_child(_add_input)
    var add := UI.button(tr("ui.lobby.add_friend"))
    add.name = "AddFriendButton"
    add.pressed.connect(_on_add_friend_pressed)
    add_row.add_child(add)
    parent.add_child(add_row)

    _outgoing = UI.label("", 12)
    parent.add_child(_outgoing)

    parent.add_child(UI.label(tr("ui.lobby.incoming_section"), 14))
    _incoming_box = VBoxContainer.new()
    _incoming_box.name = "IncomingRequests"
    _incoming_box.add_theme_constant_override("separation", 6)
    parent.add_child(_incoming_box)

    _friends_box = VBoxContainer.new()
    _friends_box.name = "FriendList"
    _friends_box.add_theme_constant_override("separation", 6)
    parent.add_child(_friends_box)

    parent.add_child(UI.label(tr("ui.lobby.recent_section"), 14))
    _recent_box = VBoxContainer.new()
    _recent_box.name = "RecentPlayers"
    _recent_box.add_theme_constant_override("separation", 6)
    parent.add_child(_recent_box)


func _render_social() -> void:
    _greeting.text = tr("ui.lobby.logged_in") % [GameStore.display_name, GameStore.player_code]

    UI.clear(_incoming_box)
    if GameStore.incoming_requests.is_empty():
        _incoming_box.add_child(UI.empty_hint(tr("ui.lobby.no_requests")))
    for request in GameStore.incoming_requests:
        _incoming_box.add_child(_request_row(request))

    UI.clear(_friends_box)
    if GameStore.friends.is_empty():
        _friends_box.add_child(UI.empty_hint(tr("ui.lobby.no_friends")))
    for friend in GameStore.friends:
        _friends_box.add_child(_friend_row(friend))

    UI.clear(_recent_box)
    if GameStore.recent_players.is_empty():
        _recent_box.add_child(UI.empty_hint(tr("ui.lobby.no_recent")))
    for player in GameStore.recent_players:
        _recent_box.add_child(_recent_row(player))

    _outgoing.visible = not GameStore.outgoing_requests.is_empty()
    _outgoing.text = tr("ui.lobby.outgoing_hint") % GameStore.outgoing_requests.size()


func _request_row(request: Dictionary) -> HBoxContainer:
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", 8)
    var player_id := str(request.get("playerId", ""))
    row.add_child(
        UI.label(
            tr("ui.lobby.request_incoming") % [
                str(request.get("displayName", player_id)),
                str(request.get("playerCode", "")),
            ],
            14,
            false,
        )
    )
    var accept := UI.button(tr("ui.lobby.accept"))
    accept.name = "Accept_%s" % player_id
    accept.pressed.connect(_on_respond_pressed.bind(player_id, true))
    row.add_child(accept)
    var decline := UI.button(tr("ui.lobby.decline"))
    decline.name = "Decline_%s" % player_id
    decline.pressed.connect(_on_respond_pressed.bind(player_id, false))
    row.add_child(decline)
    return row


func _friend_row(friend: Dictionary) -> HBoxContainer:
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", 8)
    var player_id := str(friend.get("playerId", ""))
    var status := UI.enum_text("friend_status", str(friend.get("status", "offline")))
    row.add_child(
        UI.label(
            tr("ui.lobby.friend_row") % [
                str(friend.get("displayName", player_id)),
                str(friend.get("playerCode", "")),
                status,
            ],
            14,
            false,
        )
    )
    var remove := UI.button(tr("ui.lobby.remove_friend"))
    remove.name = "Remove_%s" % player_id
    remove.pressed.connect(_on_remove_pressed.bind(player_id))
    row.add_child(remove)
    return row


func _recent_row(player: Dictionary) -> HBoxContainer:
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", 8)
    var player_id := str(player.get("playerId", ""))
    row.add_child(
        UI.label(
            tr("ui.lobby.recent_row") % [
                str(player.get("displayName", player_id)),
                str(player.get("playerCode", "")),
            ],
            14,
            false,
        )
    )
    var add := UI.button(tr("ui.lobby.add_friend"))
    add.name = "AddRecent_%s" % player_id
    add.pressed.connect(_on_add_recent_pressed.bind(str(player.get("playerCode", ""))))
    row.add_child(add)
    return row


func _on_create_pressed() -> void:
    _status.text = tr("ui.lobby.creating")
    Net.send(Protocol.MSG_CREATE_ROOM, {})


func _on_gallery_pressed() -> void:
    UI.navigate(self, "res://scenes/gallery.tscn")


func _on_join_pressed() -> void:
    var code := _code_input.text.strip_edges().to_upper()
    if code == "":
        _status.text = tr("ui.lobby.enter_room_code")
        return
    _status.text = tr("ui.lobby.joining") % code
    Net.send(Protocol.MSG_JOIN_ROOM, {"code": code})


func _on_add_friend_pressed() -> void:
    _add_friend_by_code(_add_input.text)


func _on_add_recent_pressed(code: String) -> void:
    _add_friend_by_code(code)


func _add_friend_by_code(raw_code: String) -> void:
    var code := raw_code.strip_edges()
    if code.length() != 6 or not code.is_valid_int():
        _status.text = tr("ui.lobby.invalid_player_code")
        return
    Net.send(Protocol.MSG_FRIEND_REQUEST, {"toPlayerCode": code})
    _status.text = tr("ui.lobby.request_sent") % code


func _on_respond_pressed(player_id: String, accept: bool) -> void:
    Net.send(Protocol.MSG_FRIEND_RESPOND, {"fromPlayerId": player_id, "accept": accept})


func _on_remove_pressed(player_id: String) -> void:
    Net.send(Protocol.MSG_FRIEND_REMOVE, {"friendId": player_id})


func _on_invite_responded(_invite_id: String, accept: bool) -> void:
    if accept:
        _status.text = tr("ui.lobby.accepting_invite")


func _on_room_changed() -> void:
    UI.navigate(self, "res://scenes/room.tscn")


func _on_auth_changed() -> void:
    _greeting.text = tr("ui.lobby.logged_in") % [GameStore.display_name, GameStore.player_code]
    _status.text = tr("ui.lobby.reconnected")


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    _status.text = UI.error_text(code)
    _status.tooltip_text = UI.error_tooltip(code)
    if UI.error_route(code) == "login":
        GameStore.reset_session()
        UI.navigate(self, "res://scenes/login.tscn")
