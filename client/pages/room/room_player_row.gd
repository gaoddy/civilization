class_name RoomPlayerRow
extends HBoxContainer
## 选英雄页的成员行：名字与确认状态 pill，房主在等待区时附带移除按钮。

signal kicked(player_id: String)


func _init(player: Dictionary, can_manage: bool) -> void:
    alignment = BoxContainer.ALIGNMENT_CENTER
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_theme_constant_override("separation", UITheme.space("space-sm"))
    var player_id := str(player.get("playerId", ""))
    var display := str(player.get("displayName", ""))
    if display == "":
        display = player_id
    var ready := bool(player.get("ready", false))
    var status := tr("ui.room.confirmed") if ready else tr("ui.room.unconfirmed")
    add_child(UIControls.pill_label("%s · %s" % [display, status], "success" if ready else "neutral"))
    if can_manage:
        var kick := UIControls.button(tr("ui.room.kick"))
        kick.name = "Kick_%s" % player_id
        kick.pressed.connect(func(): kicked.emit(player_id))
        add_child(kick)
