extends Control
## 全服排行榜：展示服务端下发的积分前 100 名，点击任意一行进入个人主页。
##
## 页面只搬运服务端字段；名次、段位与积分不在本地重算。

var _list: VBoxContainer
var _status: Label
var _received := false


func _ready() -> void:
    UILayout.safe_area(self)
    _build()
    GameStore.leaderboard_changed.connect(_on_leaderboard_changed)
    _render()
    GameStore.request_leaderboard()


func _build() -> void:
    add_child(UIControls.background())

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-2xl"))
    add_child(margin)

    var layout := VBoxContainer.new()
    layout.add_theme_constant_override("separation", UITheme.space("space-lg"))
    margin.add_child(layout)

    var header := HBoxContainer.new()
    header.add_theme_constant_override("separation", UITheme.space("space-lg"))
    layout.add_child(header)
    header.add_child(UIControls.label(tr("ui.leaderboard.title"), UITheme.font_size("font-hero"), false))
    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    header.add_child(spacer)
    var back := UIControls.button(tr("ui.leaderboard.back"))
    back.name = "BackButton"
    back.pressed.connect(_on_back_pressed)
    header.add_child(back)

    _status = UIControls.label(tr("ui.leaderboard.loading"), UITheme.font_size("font-body"))
    _status.name = "LeaderboardStatus"
    layout.add_child(_status)

    var scroll := ScrollContainer.new()
    scroll.name = "LeaderboardScroll"
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    layout.add_child(scroll)

    _list = VBoxContainer.new()
    _list.name = "LeaderboardList"
    _list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _list.add_theme_constant_override("separation", UITheme.space("space-sm"))
    scroll.add_child(_list)


func _on_leaderboard_changed() -> void:
    _received = true
    _render()


func _render() -> void:
    UIControls.clear(_list)
    var entries = GameStore.leaderboard
    var list: Array = entries if entries is Array else []
    _status.visible = list.is_empty()
    _status.text = (
        tr("ui.leaderboard.empty") if _received else tr("ui.leaderboard.loading")
    )
    for entry in list:
        if not (entry is Dictionary):
            continue
        var row := UIControls.button(tr("ui.leaderboard.row") % [
            _int_of(entry, "rank"),
            str(entry.get("displayName", "")),
            UIWording.tier_text(str(entry.get("tier", ""))),
            _int_of(entry, "rating"),
        ])
        row.name = "LeaderboardRow"
        row.alignment = HORIZONTAL_ALIGNMENT_LEFT
        var player_id := str(entry.get("playerId", ""))
        if player_id != "":
            row.pressed.connect(func(): _open_profile(player_id))
        _list.add_child(row)


func _open_profile(player_id: String) -> void:
    GameStore.open_profile(player_id, UINavigation.PAGE_LEADERBOARD)
    UINavigation.navigate(self, UINavigation.PAGE_PROFILE)


func _on_back_pressed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_LOBBY)


## Android 系统返回：与页面返回按钮一致，回大厅。
func system_back() -> bool:
    _on_back_pressed()
    return true


## 服务端字段缺失或为空时按 0 处理。
static func _int_of(source: Dictionary, key: String) -> int:
    var value = source.get(key, 0)
    return 0 if value == null else int(value)
