extends Control
## 大厅：创建房间或凭房间码加入。

var _status: Label
var _code_input: LineEdit
var _greeting: Label


func _ready() -> void:
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.error_received.connect(_on_error)
    GameStore.auth_changed.connect(_on_auth_changed)
    if not Net.is_open():
        _status.text = tr("ui.lobby.not_connected")


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

    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", 12)
    margin.add_child(box)

    box.add_child(UI.label(tr("ui.lobby.title"), 28))
    _greeting = UI.label(tr("ui.lobby.logged_in") % [GameStore.display_name, GameStore.player_id], 14)
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


func _on_room_changed() -> void:
    UI.navigate(self, "res://scenes/room.tscn")


func _on_auth_changed() -> void:
    _greeting.text = tr("ui.lobby.logged_in") % [GameStore.display_name, GameStore.player_id]
    _status.text = tr("ui.lobby.reconnected")


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    _status.text = UI.error_text(code)
    _status.tooltip_text = UI.error_tooltip(code)
    if UI.error_route(code) == "login":
        GameStore.reset_session()
        UI.navigate(self, "res://scenes/login.tscn")
