class_name FriendsPanel
extends PopupLayer
## 好友浮层：添加好友、处理申请、管理好友与从最近队友回加。
##
## 组件自己订阅 `GameStore.social_changed` 并渲染；接受、拒绝、删除与回加直接
## 发对应消息。宿主只负责打开/关闭，并把服务端错误转进浮层。Esc 与关闭按钮
## 发出 `closed`，由页面决定收起；遮罩与居中由 `PopupLayer` 承担。

signal closed

var _panel: PanelContainer
var _scroll: ScrollContainer
var _status: Label
var _add_input: LineEdit
var _incoming_box: VBoxContainer
var _friends_box: VBoxContainer
var _recent_box: VBoxContainer
var _outgoing: Label


func _init() -> void:
    super(PopupLayer.Placement.CENTER, UITheme.opacity("scrim-alpha-friends"))
    name = "FriendsPanel"
    mouse_filter = Control.MOUSE_FILTER_STOP
    scrim.mouse_filter = Control.MOUSE_FILTER_IGNORE

    _panel = UIControls.card_panel("raised")
    _panel.name = "FriendsPanelCard"
    _panel.custom_minimum_size = Vector2(600, 0)
    set_content(_panel)

    var root := VBoxContainer.new()
    root.add_theme_constant_override("separation", UITheme.space("space-md"))
    _panel.add_child(root)

    var header := HBoxContainer.new()
    header.name = "FriendsHeader"
    header.add_theme_constant_override("separation", UITheme.space("space-sm"))
    root.add_child(header)
    header.add_child(UIControls.label(tr("ui.lobby.friends_section"), UITheme.font_size("font-title-lg"), false))
    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    header.add_child(spacer)
    var close_button := UIControls.button(tr("ui.lobby.close"))
    close_button.name = "FriendsClose"
    close_button.pressed.connect(_on_close)
    header.add_child(close_button)

    _scroll = ScrollContainer.new()
    _scroll.name = "FriendsScroll"
    _scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    _scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    root.add_child(_scroll)

    var body := VBoxContainer.new()
    body.name = "FriendsBody"
    body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override("separation", UITheme.space("space-sm"))
    _scroll.add_child(body)

    var add_row := HBoxContainer.new()
    add_row.add_theme_constant_override("separation", UITheme.space("space-md"))
    body.add_child(add_row)
    _add_input = UIControls.line_edit(tr("ui.lobby.add_friend_placeholder"), 220)
    _add_input.name = "FriendCodeInput"
    _add_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    add_row.add_child(_add_input)
    var add := UIControls.button(tr("ui.lobby.add_friend"))
    add.name = "AddFriendButton"
    add.pressed.connect(_on_add_friend_pressed)
    add_row.add_child(add)

    _status = UIControls.label("", UITheme.font_size("font-body-sm"))
    body.add_child(_status)

    _outgoing = UIControls.label("", UITheme.font_size("font-body-sm"))
    _outgoing.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    body.add_child(_outgoing)

    body.add_child(UIControls.label(tr("ui.lobby.incoming_section"), UITheme.font_size("font-title"), false))
    _incoming_box = VBoxContainer.new()
    _incoming_box.name = "IncomingRequests"
    _incoming_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    body.add_child(_incoming_box)

    _friends_box = VBoxContainer.new()
    _friends_box.name = "FriendList"
    _friends_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    body.add_child(_friends_box)

    body.add_child(UIControls.label(tr("ui.lobby.recent_section"), UITheme.font_size("font-title"), false))
    _recent_box = VBoxContainer.new()
    _recent_box.name = "RecentPlayers"
    _recent_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    body.add_child(_recent_box)


func _ready() -> void:
    # 安全区包裹会重新挂载子节点并再触发一次 `_ready`，连接前先判重。
    if not resized.is_connected(_sync_layout):
        resized.connect(_sync_layout)
    if not GameStore.social_changed.is_connected(_render):
        GameStore.social_changed.connect(_render)
    _sync_layout()
    _render()


func _exit_tree() -> void:
    # 浮层随宿主页面一起释放；不主动断开会让全局信号持有已释放的节点。
    if GameStore.social_changed.is_connected(_render):
        GameStore.social_changed.disconnect(_render)


## 打开浮层；列表按当前社交状态重算。
func open() -> void:
    _render()
    open_layer()


func close() -> void:
    close_layer()


## 服务端错误就地显示，不跳回大厅主页面。
func show_error(code: String) -> void:
    _status.text = UIWording.error_text(code)
    _status.tooltip_text = UIWording.error_tooltip(code)
    _status.add_theme_color_override("font_color", UITheme.color("color-danger"))


func _unhandled_input(event: InputEvent) -> void:
    if not visible or not is_visible_in_tree():
        return
    if event.is_action_pressed("ui_cancel"):
        closed.emit()
        get_viewport().set_input_as_handled()


## 浮层高度按视口收缩：头部与输入行固定，列表在剩余高度内滚动。
func _sync_layout() -> void:
    var width := mini(640, maxi(360, int(size.x) - UITheme.space("space-2xl") * 2))
    _panel.custom_minimum_size.x = width
    _scroll.custom_minimum_size.y = maxi(180, int(size.y) - 230)


func _render() -> void:
    if _incoming_box == null:
        return
    _outgoing.visible = not GameStore.outgoing_requests.is_empty()
    _outgoing.text = tr("ui.lobby.outgoing_hint") % GameStore.outgoing_requests.size()

    UIControls.clear(_incoming_box)
    if GameStore.incoming_requests.is_empty():
        _incoming_box.add_child(UIControls.empty_hint(tr("ui.lobby.no_requests")))
    for request in GameStore.incoming_requests:
        var row := FriendRequestRow.new(request)
        row.responded.connect(_on_respond_pressed)
        _incoming_box.add_child(row)

    UIControls.clear(_friends_box)
    if GameStore.friends.is_empty():
        _friends_box.add_child(UIControls.empty_hint(tr("ui.lobby.no_friends")))
    for friend in GameStore.friends:
        var row := FriendRow.new(friend)
        row.removed.connect(_on_remove_pressed)
        _friends_box.add_child(row)

    UIControls.clear(_recent_box)
    if GameStore.recent_players.is_empty():
        _recent_box.add_child(UIControls.empty_hint(tr("ui.lobby.no_recent")))
    for player in GameStore.recent_players:
        var row := RecentPlayerRow.new(player)
        row.add_requested.connect(_add_friend_by_code)
        _recent_box.add_child(row)


func _on_close() -> void:
    closed.emit()


func _on_add_friend_pressed() -> void:
    _add_friend_by_code(_add_input.text)


func _add_friend_by_code(raw_code: String) -> void:
    var code := raw_code.strip_edges()
    if code.length() != 6 or not code.is_valid_int():
        _set_status(tr("ui.lobby.invalid_player_code"), true)
        return
    Net.send(Protocol.MSG_FRIEND_REQUEST, {"toPlayerCode": code})
    _set_status(tr("ui.lobby.request_sent") % code)


func _on_respond_pressed(player_id: String, accept: bool) -> void:
    Net.send(Protocol.MSG_FRIEND_RESPOND, {"fromPlayerId": player_id, "accept": accept})


func _on_remove_pressed(player_id: String) -> void:
    Net.send(Protocol.MSG_FRIEND_REMOVE, {"friendId": player_id})


func _set_status(text: String, is_error := false) -> void:
    _status.text = text
    _status.tooltip_text = ""
    _status.add_theme_color_override(
        "font_color",
        UITheme.color("color-danger") if is_error else UITheme.color("color-text-muted"),
    )
