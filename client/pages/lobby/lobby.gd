extends Control
## 大厅：创建或凭房间码加入；好友管理与邀请分别放在浮层里，主页只留开局入口。

var _status: Label
var _code_input: LineEdit
var _greeting: Label
var _friends_button: Button
var _friends: FriendsPanel
var _invite_popup: InvitePopup


func _ready() -> void:
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.error_received.connect(_on_error)
    GameStore.auth_changed.connect(_on_auth_changed)
    GameStore.social_changed.connect(_render_friends_entry)
    if not Net.is_open():
        _status.text = tr("ui.lobby.not_connected")
    _show_room_closed_reason()
    _render_friends_entry()
    UILayout.safe_area(self)


## 被移出或房间解散后回到大厅：在到达的页面上提示原因，读后清空。
func _show_room_closed_reason() -> void:
    if GameStore.room_closed_reason == "":
        return
    _status.text = (
        tr("ui.room.kicked_notice")
        if GameStore.room_closed_reason == "kicked"
        else tr("ui.room.dissolved_notice")
    )
    GameStore.room_closed_reason = ""


func _build() -> void:
    add_child(UI.background())

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

    var top := HBoxContainer.new()
    top.name = "LobbyTop"
    top.add_theme_constant_override("separation", UITheme.space("space-lg"))
    column.add_child(top)
    top.add_child(UI.label(tr("ui.lobby.title"), UITheme.font_size("font-hero"), false))
    _greeting = UI.label(
        tr("ui.lobby.logged_in") % [GameStore.display_name, GameStore.player_code],
        UITheme.font_size("font-body"),
        false,
    )
    top.add_child(_greeting)
    var top_spacer := Control.new()
    top_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    top.add_child(top_spacer)
    _status = UI.label("", UITheme.font_size("font-body-sm"))
    _status.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
    top.add_child(_status)

    var center := CenterContainer.new()
    center.name = "LobbyCenter"
    center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    center.size_flags_vertical = Control.SIZE_EXPAND_FILL
    column.add_child(center)

    var panel := UI.card_panel("surface")
    panel.name = "LobbyActions"
    panel.custom_minimum_size = Vector2(620, 0)
    center.add_child(panel)

    var actions := VBoxContainer.new()
    actions.add_theme_constant_override("separation", UITheme.space("space-lg"))
    panel.add_child(actions)

    var create := UI.styled_button(tr("ui.lobby.create_room"), "primary", 50)
    create.name = "CreateRoomButton"
    create.pressed.connect(_on_create_pressed)
    actions.add_child(create)

    actions.add_child(UI.label(tr("ui.lobby.join_section"), UITheme.font_size("font-title"), false))
    var join_row := HBoxContainer.new()
    join_row.add_theme_constant_override("separation", UITheme.space("space-md"))
    actions.add_child(join_row)
    _code_input = LineEdit.new()
    _code_input.placeholder_text = tr("ui.lobby.room_code_placeholder")
    _code_input.custom_minimum_size = Vector2(220, 0)
    _code_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    join_row.add_child(_code_input)
    var join := UI.button(tr("ui.lobby.join"))
    join.name = "JoinButton"
    join.pressed.connect(_on_join_pressed)
    join_row.add_child(join)

    # 好友与图鉴固定在底部两角，中间留给开局操作。
    var bottom := HBoxContainer.new()
    bottom.name = "LobbyBottom"
    bottom.add_theme_constant_override("separation", UITheme.space("space-md"))
    column.add_child(bottom)
    _friends_button = UI.button("")
    _friends_button.name = "FriendsButton"
    _friends_button.custom_minimum_size = Vector2(200, _friends_button.custom_minimum_size.y)
    _friends_button.pressed.connect(_on_friends_pressed)
    bottom.add_child(_friends_button)
    var bottom_spacer := Control.new()
    bottom_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    bottom.add_child(bottom_spacer)
    var gallery := UI.button(tr("ui.gallery.entry"))
    gallery.name = "GalleryButton"
    gallery.custom_minimum_size = Vector2(200, gallery.custom_minimum_size.y)
    gallery.pressed.connect(_on_gallery_pressed)
    bottom.add_child(gallery)

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
    _friends_button.text = (
        tr("ui.lobby.friends_entry_pending") % pending
        if pending > 0
        else tr("ui.lobby.friends_entry")
    )


func _on_create_pressed() -> void:
    _status.text = tr("ui.lobby.creating")
    Net.send(Protocol.MSG_CREATE_ROOM, {})


func _on_gallery_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_GALLERY)


func _on_friends_pressed() -> void:
    _friends.open()


func _close_friends() -> void:
    _friends.close()


func _on_join_pressed() -> void:
    var code := _code_input.text.strip_edges().to_upper()
    if code == "":
        _status.text = tr("ui.lobby.enter_room_code")
        return
    _status.text = tr("ui.lobby.joining") % code
    Net.send(Protocol.MSG_JOIN_ROOM, {"code": code})


func _on_invite_responded(_invite_id: String, accept: bool) -> void:
    if accept:
        _status.text = tr("ui.lobby.accepting_invite")


func _on_room_changed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_ROOM)


func _on_auth_changed() -> void:
    _greeting.text = tr("ui.lobby.logged_in") % [GameStore.display_name, GameStore.player_code]
    _status.text = tr("ui.lobby.reconnected")


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    _status.text = UIWording.error_text(code)
    _status.tooltip_text = UIWording.error_tooltip(code)
    if _friends.visible:
        _friends.show_error(code)
    if UINavigation.error_route(code) == UINavigation.ROUTE_LOGIN:
        GameStore.reset_session()
        UINavigation.navigate(self, UINavigation.PAGE_LOGIN)
