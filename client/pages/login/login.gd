extends Control
## 登录：开发渠道走手机号 + 开发验证码；Steam 渠道自动取票登录。
##
## 两个渠道共用连接与认证消息，页面只按 `Platform.channel()` 决定展示分支。

var _status: Label
var _login_button: Button
var _server: LineEdit
var _phone: LineEdit
var _code: LineEdit
var _name: LineEdit
var _retry_button: Button
var _exit_button: Button

var _steam_ticket := ""
var _steam_persona := ""

var _local_failed := false


func _ready() -> void:
    UILayout.safe_area(self)
    _build()
    Net.connected.connect(_on_connected)
    Net.disconnected.connect(_on_disconnected)
    GameStore.auth_changed.connect(_on_auth_changed)
    GameStore.error_received.connect(_on_error)
    if Platform.is_steam():
        _begin_steam_login.call_deferred()
    if LocalServer.enabled and LocalServer.auto_flow and _login_button != null:
        # 先订阅再延迟触发：LocalServer 的 failed 也通过 call_deferred 发出，
        # 在延迟回调里才订阅会漏掉启动阶段的失败。
        LocalServer.server_ready.connect(_on_local_server_ready, CONNECT_ONE_SHOT)
        LocalServer.failed.connect(_on_local_server_failed, CONNECT_ONE_SHOT)
        _auto_login.call_deferred()


func _build() -> void:
    add_child(UIControls.background())

    var center := CenterContainer.new()
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(center)

    var box := VBoxContainer.new()
    box.custom_minimum_size = Vector2(440, 0)
    box.add_theme_constant_override("separation", UITheme.space("space-md-plus"))
    center.add_child(box)

    box.add_child(UIControls.label(tr("ui.login.title"), UITheme.font_size("font-display-lg")))
    if Platform.is_steam():
        _build_steam(box)
    else:
        _build_dev(box)


func _build_dev(box: VBoxContainer) -> void:
    box.add_child(UIControls.label(tr("ui.login.subtitle"), UITheme.font_size("font-body")))
    box.add_child(HSeparator.new())

    _server = _add_field(box, tr("ui.login.server"), Net.default_url())
    _phone = _add_field(box, tr("ui.login.phone"), "13900000000")
    _code = _add_field(box, tr("ui.login.code"), "123456")
    _name = _add_field(box, tr("ui.login.display_name"), tr("ui.login.default_display_name"))

    _login_button = UIControls.button(tr("ui.login.submit"))
    _login_button.name = "LoginButton"
    _login_button.pressed.connect(_on_login_pressed)
    box.add_child(_login_button)

    _status = UIControls.label("", UITheme.font_size("font-body"))
    _status.name = "StatusLabel"
    box.add_child(_status)


func _build_steam(box: VBoxContainer) -> void:
    box.add_child(UIControls.label(tr("ui.login.steam.subtitle"), UITheme.font_size("font-body")))
    box.add_child(HSeparator.new())

    _status = UIControls.label(tr("ui.login.steam.starting"), UITheme.font_size("font-body"))
    _status.name = "StatusLabel"
    box.add_child(_status)

    _retry_button = UIControls.button(tr("ui.login.steam.retry"))
    _retry_button.name = "RetryButton"
    _retry_button.pressed.connect(_on_retry_pressed)
    _retry_button.visible = false
    box.add_child(_retry_button)

    _exit_button = UIControls.button(tr("ui.login.steam.exit"))
    _exit_button.name = "ExitButton"
    _exit_button.pressed.connect(_on_exit_pressed)
    box.add_child(_exit_button)


func _add_field(parent: VBoxContainer, title: String, value: String) -> LineEdit:
    var field := LoginField.new(title, value)
    parent.add_child(field)
    return field.edit


func _on_login_pressed() -> void:
    _login_button.disabled = true
    _status.text = tr("ui.login.connecting") % _server.text
    if Net.is_open():
        _on_connected()
    else:
        Net.connect_to(_server.text.strip_edges())


## 调试包：本地服务就绪后用预填的开发账号自动提交登录。
func _auto_login() -> void:
    # failed 与自动登录都在 deferred 批次里，失败可能先到；此时不再进入连接态。
    if _local_failed:
        return
    if LocalServer.is_server_ready():
        _on_login_pressed()
        return
    _login_button.disabled = true
    _status.text = tr("ui.login.connecting") % Net.default_url()


func _on_local_server_ready() -> void:
    _on_login_pressed()


func _on_local_server_failed(reason: String) -> void:
    _local_failed = true
    _status.text = reason
    _login_button.disabled = false


## Steam 自动登录：启动检查 → 取票 → 连接服务端 → 发认证。
func _begin_steam_login() -> void:
    _retry_button.visible = false
    _retry_button.disabled = true
    var startup := Platform.startup_error
    if startup != "":
        _show_steam_failure(startup)
        return
    _status.text = tr("ui.login.steam.signing_in")
    await _steam_sign_in()


func _steam_sign_in() -> void:
    var result: Dictionary = await Platform.sign_in()
    if not bool(result.get("ok", false)):
        _show_steam_failure(str(result.get("reason", "unavailable")))
        return
    _steam_ticket = str(result.get("ticket", ""))
    _steam_persona = str(result.get("personaName", ""))
    _status.text = tr("ui.login.steam.connecting")
    if Net.is_open():
        _send_steam_auth()
    else:
        Net.connect_to(Net.default_url())


func _send_steam_auth() -> void:
    if _steam_ticket == "":
        _show_steam_failure("auth_failed")
        return
    _status.text = tr("ui.login.steam.authenticating")
    GameStore.login_steam(_steam_ticket, _steam_persona)


func _on_retry_pressed() -> void:
    _retry_button.visible = false
    _retry_button.disabled = true
    var startup := Platform.retry_startup()
    if startup != "":
        _show_steam_failure(startup)
        return
    _status.text = tr("ui.login.steam.signing_in")
    await _steam_sign_in()


func _on_exit_pressed() -> void:
    get_tree().quit()


## 失败状态只保留一个文案，并给出唯一出口：「重试」。
func _show_steam_failure(reason: String) -> void:
    _status.text = _steam_failure_text(reason)
    _retry_button.visible = true
    _retry_button.disabled = false


func _steam_failure_text(reason: String) -> String:
    match reason:
        "not_running":
            return tr("ui.login.steam.not_running")
        "not_logged_in":
            return tr("ui.login.steam.not_logged_in")
        "network":
            return tr("ui.login.steam.network_error")
        "timeout", "auth_failed":
            return tr("ui.login.steam.failed")
        "unavailable":
            return tr("ui.login.steam.unavailable")
        "STEAM_DISABLED":
            return tr("ui.login.steam.unavailable")
        _:
            return UIWording.error_text(reason)


func _on_connected() -> void:
    if Platform.is_steam():
        _send_steam_auth()
        return
    _status.text = tr("ui.login.authenticating")
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
    UINavigation.navigate(self, UINavigation.PAGE_LOBBY)


func _on_disconnected() -> void:
    if Platform.is_steam():
        _show_steam_failure("network")
        return
    _status.text = tr("ui.login.disconnected")
    _login_button.disabled = false


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    if Platform.is_steam():
        _show_steam_failure(code)
        return
    _status.text = UIWording.error_text(code)
    _status.tooltip_text = UIWording.error_tooltip(code)
    _login_button.disabled = false
