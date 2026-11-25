class_name LobbyLeftActions
extends HBoxContainer
## 左栏底栏：好友入口在最左，图鉴与排行榜向右铺开，高度与大厅图分开固定。

signal friends_pressed
signal gallery_pressed
signal account_pressed
signal leaderboard_pressed

var friends_button: FriendsButton


func _init() -> void:
    name = "LobbyLeftActions"
    add_theme_constant_override("separation", UITheme.space("space-md"))

    friends_button = FriendsButton.new()
    friends_button.open_requested.connect(func() -> void: friends_pressed.emit())
    add_child(friends_button)
    # 大厅只保留「开始匹配」的引导流光，左栏入口保持静态边框。
    FrameMaterial.set_guide_preference(friends_button, false)

    add_child(_entry_button(tr("ui.gallery.entry"), "GalleryButton", gallery_pressed))
    if Platform.is_steam():
        add_child(_entry_button(tr("ui.account.title"), "AccountButton", account_pressed))
    add_child(
        _entry_button(tr("ui.leaderboard.entry"), "LeaderboardButton", leaderboard_pressed)
    )


func set_online_friends(count: int) -> void:
    friends_button.set_online_count(count)


func _entry_button(text: String, node_name: String, forwarded: Signal) -> Button:
    var button := UIControls.button(text)
    button.name = node_name
    button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    button.custom_minimum_size.y = UITheme.stage_metric("lobby-left-action-height")
    button.pressed.connect(func() -> void: forwarded.emit())
    FrameMaterial.set_guide_preference(button, false)
    return button
