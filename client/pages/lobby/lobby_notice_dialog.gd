class_name LobbyNoticeDialog
extends PopupLayer
## 大厅提示弹窗：房间关闭原因、服务端错误等只有一段说明的提示。

var _title: Label
var _body: Label


func _init() -> void:
    super(PopupLayer.Placement.CENTER, UITheme.opacity("scrim-alpha"))
    name = "LobbyNoticeDialog"
    mouse_filter = Control.MOUSE_FILTER_STOP

    var card := UIControls.card_panel("raised")
    card.name = "NoticeCard"
    card.custom_minimum_size = Vector2(UITheme.metric("status-dialog-width"), 0)
    set_content(card)

    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    card.add_child(box)

    _title = UIControls.label("", UITheme.font_size("font-title-lg"), false)
    _title.name = "NoticeTitle"
    box.add_child(_title)

    _body = UIControls.label("", UITheme.font_size("font-body"))
    _body.name = "NoticeBody"
    _body.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    box.add_child(_body)

    var row := HBoxContainer.new()
    row.alignment = BoxContainer.ALIGNMENT_END
    box.add_child(row)
    var confirm := UIControls.styled_button(
        tr("ui.lobby.close"), "primary", UITheme.stage_metric("action-button-height")
    )
    confirm.name = "NoticeConfirmButton"
    confirm.pressed.connect(close_layer)
    row.add_child(confirm)

    dismiss_on_scrim = true


func show_notice(title: String, text: String) -> void:
    _title.text = title
    _body.text = text
    open_layer()
