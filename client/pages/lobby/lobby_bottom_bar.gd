class_name LobbyBottomBar
extends HBoxContainer
## 大厅底部两角入口：好友管理与卡牌图鉴，中间留给开局操作。
## Steam 构建在好友右侧多一个「账号」入口，其余构建保持两个按钮。

signal friends_pressed
signal gallery_pressed
signal account_pressed

var friends_button: Button
var account_button: Button


func _init() -> void:
    name = "LobbyBottom"
    add_theme_constant_override("separation", UITheme.space("space-md"))
    friends_button = UIControls.button("")
    friends_button.name = "FriendsButton"
    friends_button.custom_minimum_size = Vector2(200, friends_button.custom_minimum_size.y)
    friends_button.pressed.connect(func(): friends_pressed.emit())
    add_child(friends_button)

    if Platform.is_steam():
        account_button = UIControls.button(tr("ui.account.title"))
        account_button.name = "AccountButton"
        account_button.custom_minimum_size = Vector2(200, account_button.custom_minimum_size.y)
        account_button.pressed.connect(func(): account_pressed.emit())
        add_child(account_button)

    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    add_child(spacer)
    var gallery := UIControls.button(tr("ui.gallery.entry"))
    gallery.name = "GalleryButton"
    gallery.custom_minimum_size = Vector2(200, gallery.custom_minimum_size.y)
    gallery.pressed.connect(func(): gallery_pressed.emit())
    add_child(gallery)
