class_name FriendsButton
extends Button
## 大厅好友入口：左侧好友图标，右侧在线好友数；点击请求展开好友面板。
##
## 按钮本身就是入口，不携带待处理申请数；申请与好友列表都在面板内。

signal open_requested

const ICON_SIZE := 18.0


func _init() -> void:
    name = "FriendsButton"
    alignment = HORIZONTAL_ALIGNMENT_RIGHT
    custom_minimum_size = Vector2(
        UITheme.stage_metric("lobby-friends-button-width"),
        UITheme.stage_metric("lobby-left-action-height"),
    )
    UIControls.apply_button_style(self)
    pressed.connect(func() -> void: open_requested.emit())


func set_online_count(count: int) -> void:
    text = tr("ui.lobby.friends_online") % count
    queue_redraw()


func _draw() -> void:
    var icon_side := minf(ICON_SIZE, maxf(0.0, size.y - UITheme.space("space-sm") * 2))
    var rect := Rect2(
        Vector2(UITheme.space("space-md"), (size.y - icon_side) * 0.5),
        Vector2(icon_side, icon_side),
    )
    Icons.draw(self, "friends", rect, UITheme.color("color-text"), 2.0)
