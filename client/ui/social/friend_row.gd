class_name FriendRow
extends HBoxContainer
## 好友行：显示好友与在线状态，提供删除入口。

signal removed(player_id: String)

var _player_id := ""
var _invite_button: Button


func _init(friend: Dictionary) -> void:
    add_theme_constant_override("separation", UITheme.space("space-md"))
    var player_id := str(friend.get("playerId", ""))
    _player_id = player_id
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


## 邀请模式：传入有效 handler 时增加邀请按钮；传入空 Callable 时移除。
func set_invite_handler(handler: Callable, enabled: bool) -> void:
    if not handler.is_valid():
        if _invite_button != null and is_instance_valid(_invite_button):
            _invite_button.queue_free()
        _invite_button = null
        return
    if _invite_button == null:
        _invite_button = UIControls.button(tr("ui.lobby.room_invite"))
        _invite_button.name = "Invite_%s" % _player_id
        _invite_button.pressed.connect(
            func() -> void:
                if handler.is_valid():
                    handler.call(_player_id)
        )
        add_child(_invite_button)
        move_child(_invite_button, 1)
    _invite_button.disabled = not enabled
