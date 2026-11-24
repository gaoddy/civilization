extends "res://tests/smoke_case.gd"
## 连接弹窗冒烟：默认隐藏、断线提示可重连、重连中禁用按钮。


func _ready() -> void:
    var dialog := ConnectionDialog.new()
    add_child(dialog)
    await get_tree().process_frame
    _expect(not dialog.visible, "连接弹窗默认应隐藏")

    dialog.show_disconnected()
    await get_tree().process_frame
    _expect(dialog.visible, "断线后应显示连接弹窗")
    var title := dialog.find_child("ConnectionTitle", true, false)
    var button := dialog.find_child("ReconnectButton", true, false)
    _expect(
        title != null and title.text == tr("ui.lobby.connection_lost"),
        "断线弹窗应提示连接丢失"
    )
    _expect(
        button != null and button.text == tr("ui.lobby.reconnect") and not button.disabled,
        "断线弹窗应提供可用的重连按钮"
    )
    _expect(dialog.find_child("ConnectionCard", true, false) != null, "弹窗应有卡片")

    var pressed := [false]
    dialog.reconnect_requested.connect(func(): pressed[0] = true)
    if button != null:
        button.pressed.emit()
    _expect(pressed[0], "点击重连按钮应发出重连请求")

    dialog.show_reconnecting()
    await get_tree().process_frame
    _expect(
        title != null and title.text == tr("ui.lobby.reconnecting"),
        "重连中应切换文案"
    )
    _expect(button != null and button.disabled, "重连中按钮应禁用")

    dialog.hide_dialog()
    await get_tree().process_frame
    _expect(not dialog.visible, "重连成功应收起弹窗")
    dialog.queue_free()
    _finish()
