class_name LobbyJoinDialog
extends PopupLayer
## 加入房间弹窗：只收房间码，校验通过后交给宿主发送 `JoinRoom`。
##
## 客户端只校验 `^[A-Z0-9]{6}$`，其余交服务端；错误就地显示在弹窗里。

signal join_requested(code: String)
signal closed

const CODE_PATTERN := "^[A-Z0-9]{6}$"

var _input: LineEdit
var _error: Label
var _confirm: Button


func _init() -> void:
    super(PopupLayer.Placement.CENTER, UITheme.opacity("scrim-alpha"))
    name = "LobbyJoinDialog"
    mouse_filter = Control.MOUSE_FILTER_STOP

    var card := UIControls.card_panel("raised")
    card.name = "JoinCard"
    card.custom_minimum_size = Vector2(UITheme.metric("status-dialog-width"), 0)
    set_content(card)

    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    card.add_child(box)
    box.add_child(UIControls.label(tr("ui.lobby.join_title"), UITheme.font_size("font-title-lg"), false))

    _input = UIControls.line_edit(tr("ui.lobby.room_code_placeholder"))
    _input.name = "JoinCodeInput"
    _input.max_length = 6
    box.add_child(_input)

    _error = UIControls.label("", UITheme.font_size("font-body-sm"))
    _error.name = "JoinErrorText"
    _error.add_theme_color_override("font_color", UITheme.color("color-danger"))
    box.add_child(_error)

    var row := HBoxContainer.new()
    row.alignment = BoxContainer.ALIGNMENT_END
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    box.add_child(row)

    var cancel := UIControls.button(tr("ui.lobby.close"))
    cancel.name = "JoinCancelButton"
    cancel.pressed.connect(_on_cancel)
    row.add_child(cancel)

    _confirm = UIControls.styled_button(tr("ui.lobby.join"), "primary")
    _confirm.name = "JoinConfirmButton"
    _confirm.pressed.connect(_on_confirm)
    row.add_child(_confirm)


## 每次打开都从空输入开始；本地联调可带入启动器给的房间码。
func open_dialog(initial_code := "") -> void:
    _input.text = initial_code
    _error.text = ""
    _confirm.disabled = false
    open_layer()
    _input.grab_focus()


func set_joining(joining: bool) -> void:
    _confirm.disabled = joining
    _input.editable = not joining


func show_error(code: String) -> void:
    _error.text = UIWording.error_text(code)


func _unhandled_input(event: InputEvent) -> void:
    if not visible or not is_visible_in_tree():
        return
    if event.is_action_pressed("ui_cancel"):
        _on_cancel()
        get_viewport().set_input_as_handled()


func _on_cancel() -> void:
    close_layer()
    closed.emit()


func _on_confirm() -> void:
    var code := _input.text.strip_edges().to_upper()
    var pattern := RegEx.new()
    pattern.compile(CODE_PATTERN)
    if pattern.search(code) == null:
        _error.text = tr("ui.lobby.room_code_invalid")
        return
    _error.text = ""
    join_requested.emit(code)
