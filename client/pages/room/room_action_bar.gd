class_name RoomActionBar
extends VBoxContainer
## 选英雄页的底部操作区：确认、解散与状态提示。
##
## 只做呈现与按钮可用性；请求与提示文案由页面写入。

signal confirm_pressed
signal dissolve_pressed

var confirm: Button
var dissolve: Button
var status: Label


func _init() -> void:
    name = "RoomActions"
    add_theme_constant_override("separation", UITheme.space("space-md"))

    confirm = UIControls.styled_button(tr("ui.room.confirm"), "primary", 36)
    confirm.name = "ConfirmButton"
    # 纵向容器默认拉满整行宽；确认键按自身宽度居中，与其他内容同轴。
    confirm.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    confirm.pressed.connect(func(): confirm_pressed.emit())
    add_child(confirm)

    # 解散房间只对房主开放；非房主不显示任何管理控件。
    dissolve = UIControls.button(tr("ui.room.dissolve"))
    dissolve.name = "DissolveButton"
    dissolve.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    dissolve.visible = false
    dissolve.pressed.connect(func(): dissolve_pressed.emit())
    add_child(dissolve)

    status = UIControls.label("", UITheme.font_size("font-body-sm"))
    status.name = "StatusLabel"
    status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    add_child(status)


## 确认键的可用性与文案、解散入口的可见性。
func set_state(selected_hero: String, confirmed: bool, connected: bool, is_host: bool) -> void:
    confirm.disabled = selected_hero == "" or confirmed or not connected
    confirm.text = tr("ui.room.confirmed") if confirmed else tr("ui.room.confirm")
    dissolve.visible = is_host
