extends Control
## 登录：连接服务端并完成手机号 + 开发验证码认证。

var _status: Label
var _login_button: Button
var _server: LineEdit
var _phone: LineEdit
var _code: LineEdit
var _name: LineEdit


func _ready() -> void:
    _build()
    Net.connected.connect(_on_connected)
    Net.disconnected.connect(_on_disconnected)
    GameStore.auth_changed.connect(_on_auth_changed)
    GameStore.error_received.connect(_on_error)


func _build() -> void:
    var background := ColorRect.new()
    background.color = Color("10141c")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(background)

    var center := CenterContainer.new()
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(center)

    var box := VBoxContainer.new()
    box.custom_minimum_size = Vector2(440, 0)
    box.add_theme_constant_override("separation", 10)
    center.add_child(box)

    box.add_child(UI.label("文明：七时代演进", 30))
    box.add_child(UI.label("开发环境登录，验证码固定 123456", 14))
    box.add_child(HSeparator.new())

    _server = _add_field(box, "服务器", Net.DEFAULT_URL)
    _phone = _add_field(box, "手机号", "13900000000")
    _code = _add_field(box, "验证码", "123456")
    _name = _add_field(box, "显示名", "玩家A")

    _login_button = UI.button("登录")
    _login_button.pressed.connect(_on_login_pressed)
    box.add_child(_login_button)

    _status = UI.label("", 14)
    box.add_child(_status)


func _add_field(parent: VBoxContainer, title: String, value: String) -> LineEdit:
    parent.add_child(UI.label(title, 14))
    var edit := LineEdit.new()
    edit.text = value
    parent.add_child(edit)
    return edit


func _on_login_pressed() -> void:
    _login_button.disabled = true
    _status.text = "正在连接 %s …" % _server.text
    if Net.is_open():
        _on_connected()
    else:
        Net.connect_to(_server.text.strip_edges())


func _on_connected() -> void:
    _status.text = "正在认证…"
    var payload := {
        "phone": _phone.text.strip_edges(),
        "code": _code.text.strip_edges(),
    }
    var display_name := _name.text.strip_edges()
    if display_name != "":
        payload["displayName"] = display_name
        GameStore.display_name = display_name
    Net.send(Protocol.MSG_AUTH_REQUEST, payload)


func _on_auth_changed() -> void:
    get_tree().change_scene_to_file("res://scenes/lobby.tscn")


func _on_disconnected() -> void:
    _status.text = "连接已断开，正在自动重连…"
    _login_button.disabled = false


func _on_error() -> void:
    _status.text = "%s：%s" % [
        str(GameStore.last_error.get("code", "")),
        str(GameStore.last_error.get("message", "")),
    ]
    _login_button.disabled = false
