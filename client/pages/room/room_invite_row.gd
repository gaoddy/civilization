class_name RoomInviteRow
extends HBoxContainer
## 选英雄页的邀请入口：只列出好友；不在线或已在房间的好友置灰。

signal invite_pressed(player_id: String, friend_name: String)

var _buttons: HBoxContainer


func _init() -> void:
    name = "InviteRow"
    alignment = BoxContainer.ALIGNMENT_CENTER
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(UIControls.label(tr("ui.room.invite_section"), UITheme.font_size("font-body-sm"), false))
    _buttons = HBoxContainer.new()
    _buttons.name = "InviteFriends"
    _buttons.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _buttons.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(_buttons)


func render(friends: Array) -> void:
    UIControls.clear(_buttons)
    if friends.is_empty():
        _buttons.add_child(UIControls.empty_hint(tr("ui.room.no_friends_to_invite")))
        return
    for friend in friends:
        var player_id := str(friend.get("playerId", ""))
        var friend_name := str(friend.get("displayName", player_id))
        var button := UIControls.button(friend_name)
        button.name = "Invite_%s" % player_id
        button.disabled = str(friend.get("status", "")) != "online"
        button.pressed.connect(func(): invite_pressed.emit(player_id, friend_name))
        _buttons.add_child(button)
