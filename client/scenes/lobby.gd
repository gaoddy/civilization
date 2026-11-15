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
        _status.text = "未连接服务器"


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

    box.add_child(UI.label("大厅", 28))
    _greeting = UI.label("已登录：%s  %s" % [GameStore.display_name, GameStore.player_id], 14)
    box.add_child(_greeting)
    box.add_child(HSeparator.new())

    var create := UI.button("创建房间")
    create.pressed.connect(_on_create_pressed)
    box.add_child(create)

    box.add_child(UI.label("加入已有房间", 16))
    var join_row := HBoxContainer.new()
    join_row.add_theme_constant_override("separation", 8)
    _code_input = LineEdit.new()
    _code_input.placeholder_text = "6 位房间码"
    _code_input.custom_minimum_size = Vector2(220, 0)
    join_row.add_child(_code_input)
    var join := UI.button("加入")
    join.pressed.connect(_on_join_pressed)
    join_row.add_child(join)
    box.add_child(join_row)

    _status = UI.label("", 14)
    box.add_child(_status)


func _on_create_pressed() -> void:
    _status.text = "正在创建房间…"
    Net.send(Protocol.MSG_CREATE_ROOM, {})


func _on_join_pressed() -> void:
    var code := _code_input.text.strip_edges().to_upper()
    if code == "":
        _status.text = "请输入房间码"
        return
    _status.text = "正在加入 %s …" % code
    Net.send(Protocol.MSG_JOIN_ROOM, {"code": code})


func _on_room_changed() -> void:
    get_tree().change_scene_to_file("res://scenes/room.tscn")


func _on_auth_changed() -> void:
    _greeting.text = "已登录：%s  %s" % [GameStore.display_name, GameStore.player_id]
    _status.text = "连接已恢复"


func _on_error() -> void:
    _status.text = "%s：%s" % [
        str(GameStore.last_error.get("code", "")),
        str(GameStore.last_error.get("message", "")),
    ]
