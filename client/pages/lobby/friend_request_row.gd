class_name FriendRequestRow
extends HBoxContainer
## 好友申请行：显示申请人与玩家码，提供接受与拒绝。
##
## 只发出结果信号；请求由浮层发。

signal responded(player_id: String, accept: bool)


func _init(request: Dictionary) -> void:
    add_theme_constant_override("separation", UITheme.space("space-md"))
    var player_id := str(request.get("playerId", ""))
    add_child(
        UIControls.label(
            tr("ui.lobby.request_incoming") % [
                str(request.get("displayName", player_id)),
                str(request.get("playerCode", "")),
            ],
            UITheme.font_size("font-body"),
            false,
        )
    )
    var accept := UIControls.button(tr("ui.lobby.accept"))
    accept.name = "Accept_%s" % player_id
    accept.pressed.connect(func(): responded.emit(player_id, true))
    add_child(accept)
    var decline := UIControls.button(tr("ui.lobby.decline"))
    decline.name = "Decline_%s" % player_id
    decline.pressed.connect(func(): responded.emit(player_id, false))
    add_child(decline)
