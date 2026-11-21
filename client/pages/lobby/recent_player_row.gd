class_name RecentPlayerRow
extends HBoxContainer
## 最近队友行：显示最近同场的玩家，按玩家码回加好友。

signal add_requested(player_code: String)


func _init(player: Dictionary) -> void:
    add_theme_constant_override("separation", UITheme.space("space-md"))
    var player_id := str(player.get("playerId", ""))
    add_child(
        UIControls.label(
            tr("ui.lobby.recent_row") % [
                str(player.get("displayName", player_id)),
                str(player.get("playerCode", "")),
            ],
            UITheme.font_size("font-body"),
            false,
        )
    )
    var add := UIControls.button(tr("ui.lobby.add_friend"))
    add.name = "AddRecent_%s" % player_id
    var player_code := str(player.get("playerCode", ""))
    add.pressed.connect(func(): add_requested.emit(player_code))
    add_child(add)
