class_name ConnectionDialog
extends PopupLayer
## 断线弹窗：连接丢失时覆盖当前页面，只提供重连入口。
##
## 组件只负责提示与按钮状态，不直接调用 `Net`；重连请求由宿主
## 监听 `reconnect_requested` 后执行。

signal reconnect_requested

var _title: Label
var _hint: Label
var _button: Button


func _init() -> void:
    super(PopupLayer.Placement.CENTER, UITheme.opacity("scrim-alpha-strong"))
    name = "ConnectionDialog"
    mouse_filter = Control.MOUSE_FILTER_STOP

    var card := UIControls.card_panel("raised")
    card.name = "ConnectionCard"
    card.custom_minimum_size = Vector2(UITheme.metric("status-dialog-width"), 0)
    set_content(card)

    var root := VBoxContainer.new()
    root.add_theme_constant_override("separation", UITheme.space("space-md"))
    card.add_child(root)

    _title = UIControls.label("", UITheme.font_size("font-title-lg"), false)
    _title.name = "ConnectionTitle"
    root.add_child(_title)

    _hint = UIControls.label("", UITheme.font_size("font-body-sm"))
    _hint.name = "ConnectionHint"
    _hint.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    root.add_child(_hint)

    _button = UIControls.styled_button(
        tr("ui.lobby.reconnect"), "primary", UITheme.stage_metric("action-button-height")
    )
    _button.name = "ReconnectButton"
    _button.pressed.connect(func() -> void: reconnect_requested.emit())
    root.add_child(_button)


## 连接丢失：提示原因，按钮可用。
func show_disconnected() -> void:
    _title.text = tr("ui.lobby.connection_lost")
    _hint.text = tr("ui.lobby.not_connected")
    _hint.visible = true
    _button.disabled = false
    open_layer()


## 重连中：切换文案并禁用按钮，避免重复请求。
func show_reconnecting() -> void:
    _title.text = tr("ui.lobby.reconnecting")
    _hint.visible = false
    _button.disabled = true
    open_layer()


func hide_dialog() -> void:
    close_layer()
