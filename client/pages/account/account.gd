extends Control
## 账号页：展示名、玩家编号与手机号绑定；只读 `AccountView`，不接触网络层。
##
## 进入页面发一次 `RequestAccount` 刷新；绑定成功用返回的 `AccountView` 局部刷新。
## 页面不展示 Steam ID、票据或其他敏感标识。

const COUNTDOWN_SECONDS := 60

var _status: Label
var _form: VBoxContainer
var _name_value: Label
var _code_value: Label
var _phone_status: Label
var _phone_field: LineEdit
var _code_field: LineEdit
var _send_code_button: Button
var _bind_button: Button
var _retry_button: Button

var _awaiting_account := false
var _load_failed := false
var _busy := false
var _countdown_left := 0.0


func _ready() -> void:
    UILayout.safe_area(self)
    _build()
    GameStore.account_changed.connect(_on_account_changed)
    GameStore.phone_code_sent.connect(_on_code_sent)
    GameStore.error_received.connect(_on_error)
    _awaiting_account = true
    _render()
    GameStore.request_account()


func _process(delta: float) -> void:
    if _countdown_left <= 0.0:
        return
    _countdown_left = maxf(_countdown_left - delta, 0.0)
    _update_code_button()


func _build() -> void:
    add_child(UIControls.background())

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-2xl"))
    add_child(margin)

    var scroll := ScrollContainer.new()
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    margin.add_child(scroll)

    var column := VBoxContainer.new()
    column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    column.size_flags_vertical = Control.SIZE_EXPAND_FILL
    column.add_theme_constant_override("separation", UITheme.space("space-lg"))
    scroll.add_child(column)

    var top := HBoxContainer.new()
    top.add_theme_constant_override("separation", UITheme.space("space-lg"))
    top.add_child(UIControls.label(tr("ui.account.title"), UITheme.font_size("font-hero"), false))
    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    top.add_child(spacer)
    var back := UIControls.button(tr("ui.account.back"))
    back.name = "BackButton"
    back.pressed.connect(_on_back_pressed)
    top.add_child(back)
    column.add_child(top)
    column.add_child(HSeparator.new())

    _name_value = _add_row(column, tr("ui.account.display_name"), "AccountName")
    _code_value = _add_row(column, tr("ui.account.player_code"), "AccountCode")

    _phone_status = UIControls.label("", UITheme.font_size("font-body"))
    _phone_status.name = "PhoneStatus"
    column.add_child(_phone_status)

    _form = VBoxContainer.new()
    _form.name = "BindForm"
    _form.add_theme_constant_override("separation", UITheme.space("space-md"))
    column.add_child(_form)

    var phone_row := LoginField.new(tr("ui.account.phone"), "")
    _phone_field = phone_row.edit
    _phone_field.name = "PhoneField"
    _form.add_child(phone_row)

    var code_row := LoginField.new(tr("ui.account.phone_code"), "")
    _code_field = code_row.edit
    _code_field.name = "CodeField"
    _form.add_child(code_row)

    _send_code_button = UIControls.button(tr("ui.account.send_code"))
    _send_code_button.name = "SendCodeButton"
    _send_code_button.pressed.connect(_on_send_code_pressed)
    _form.add_child(_send_code_button)

    _bind_button = UIControls.styled_button(tr("ui.account.bind"), "primary", 36)
    _bind_button.name = "BindButton"
    _bind_button.pressed.connect(_on_bind_pressed)
    _form.add_child(_bind_button)

    var note := UIControls.label(tr("ui.account.bound_note"), UITheme.font_size("font-body-sm"))
    column.add_child(note)

    _status = UIControls.label("", UITheme.font_size("font-body"))
    _status.name = "AccountStatus"
    column.add_child(_status)

    _retry_button = UIControls.button(tr("ui.account.retry"))
    _retry_button.name = "RetryButton"
    _retry_button.visible = false
    _retry_button.pressed.connect(_on_retry_pressed)
    column.add_child(_retry_button)


func _add_row(parent: VBoxContainer, title: String, node_name: String) -> Label:
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    row.add_child(UIControls.label(title, UITheme.font_size("font-body-sm"), false))
    var value := UIControls.label("", UITheme.font_size("font-body"), false)
    value.name = node_name
    row.add_child(value)
    parent.add_child(row)
    return value


## 四种状态：加载中、加载失败、未绑定（显示表单）、已绑定（只显示掩码）。
func _render() -> void:
    var has_account := not GameStore.account.is_empty()
    var phone_bound := bool(GameStore.account.get("phoneBound", false))
    _name_value.text = str(GameStore.account.get("displayName", GameStore.display_name))
    _code_value.text = str(GameStore.account.get("playerCode", GameStore.player_code))
    _phone_status.text = (
        tr("ui.account.phone_bound") % str(GameStore.account.get("maskedPhone", ""))
        if phone_bound
        else tr("ui.account.phone_unbound")
    )
    _form.visible = has_account and not phone_bound and not _load_failed
    _retry_button.visible = _load_failed
    _bind_button.disabled = _busy
    _update_code_button()
    if _load_failed:
        if _status.text == "":
            _status.text = tr("ui.account.load_failed")
    elif not has_account:
        _status.text = tr("ui.account.loading")
    else:
        _status.text = ""


func _update_code_button() -> void:
    if _countdown_left > 0.0:
        _send_code_button.text = tr("ui.account.send_code_wait") % ceili(_countdown_left)
        _send_code_button.disabled = true
        return
    _send_code_button.text = tr("ui.account.send_code")
    _send_code_button.disabled = _busy


func _on_account_changed() -> void:
    _awaiting_account = false
    _load_failed = false
    _busy = false
    _status.text = ""
    _render()


func _on_code_sent() -> void:
    _busy = false
    _countdown_left = float(COUNTDOWN_SECONDS)
    _status.text = tr("ui.account.code_sent")
    _update_code_button()


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    if _awaiting_account:
        _awaiting_account = false
        _load_failed = true
        _busy = false
        _status.text = ""
        _render()
    else:
        _busy = false
        _render()
    _status.text = UIWording.error_text(code)


func _on_send_code_pressed() -> void:
    if _busy or _countdown_left > 0.0:
        return
    _busy = true
    _status.text = tr("ui.account.sending_code")
    _update_code_button()
    GameStore.request_phone_code(_phone_field.text)


func _on_bind_pressed() -> void:
    if _busy:
        return
    _busy = true
    _render()
    _status.text = tr("ui.account.binding")
    GameStore.bind_phone(_phone_field.text, _code_field.text)


func _on_retry_pressed() -> void:
    _load_failed = false
    _awaiting_account = true
    _status.text = ""
    _render()
    GameStore.request_account()


func _on_back_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_LOBBY)
