class_name FriendRow
extends HBoxContainer
## 好友行：显示好友与在线状态，提供删除入口。

signal removed(player_id: String)


func _init(friend: Dictionary) -> void:
    add_theme_constant_override("separation", UITheme.space("space-md"))
    var player_id := str(friend.get("playerId", ""))
    var status := UIWording.enum_text("friend_status", str(friend.get("status", "offline")))
    add_child(
        UIControls.label(
            tr("ui.lobby.friend_row") % [
                str(friend.get("displayName", player_id)),
                str(friend.get("playerCode", "")),
                status,
            ],
            UITheme.font_size("font-body"),
            false,
        )
    )
    var remove := UIControls.button(tr("ui.lobby.remove_friend"))
    remove.name = "Remove_%s" % player_id
    remove.pressed.connect(func(): removed.emit(player_id))
    add_child(remove)
