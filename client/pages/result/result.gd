extends Control
## 结算：展示服务端下发的排名、本人本局数据与对局种子。
##
## 对局结束时服务端已销毁房间，本页只读本局下发内容；内容区可滚动，
## 「确定」固定在底部，点一次即回大厅。

func _ready() -> void:
    _build()
    UILayout.safe_area(self)


func _build() -> void:
    add_child(UIControls.background())

    var center := CenterContainer.new()
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(center)

    var panel := UIControls.panel()
    panel.custom_minimum_size = Vector2(760, 0)
    panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    center.add_child(panel)

    var margin := MarginContainer.new()
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-2xl"))
    panel.add_child(margin)

    var layout := VBoxContainer.new()
    layout.add_theme_constant_override("separation", UITheme.space("space-lg"))
    margin.add_child(layout)

    # 内容列放进滚动区，按钮行留在滚动区之外，数据再多也不会把入口挤走。
    var scroll := ScrollContainer.new()
    scroll.name = "ResultScroll"
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
    scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    layout.add_child(scroll)

    var content := VBoxContainer.new()
    content.name = "ResultContent"
    content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    content.add_theme_constant_override("separation", 10)
    scroll.add_child(content)

    content.add_child(UIControls.label(tr("ui.result.title"), UITheme.font_size("font-hero")))
    content.add_child(UIControls.label(_my_place_text(), 20))
    content.add_child(HSeparator.new())
    content.add_child(ResultRanking.new(
        GameStore.ranking.duplicate(), GameStore.player_id, Callable(GameStore, "player_name")
    ))

    content.add_child(HSeparator.new())
    content.add_child(UIControls.label(tr("ui.result.stats_title"), 20))
    content.add_child(ResultStatsSection.new(_stats_data()))

    content.add_child(UIControls.label(tr("ui.result.seed") % GameStore.game_seed, UITheme.font_size("font-body")))

    var actions := HBoxContainer.new()
    actions.name = "ResultActions"
    actions.add_theme_constant_override("separation", 10)
    layout.add_child(actions)

    var confirm := UIControls.button(tr("ui.result.confirm"))
    confirm.name = "ConfirmButton"
    confirm.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    confirm.pressed.connect(_on_confirm_pressed)
    actions.add_child(confirm)


func _stats_data() -> Dictionary:
    var value = GameStore.self_stats
    return value if value is Dictionary else {}


func _my_place_text() -> String:
    for entry in GameStore.ranking:
        if str(entry.get("playerId", "")) == GameStore.player_id:
            return tr("ui.result.my_place") % int(entry.get("place", 0))
    return tr("ui.result.my_place_unrecorded")


func _on_confirm_pressed() -> void:
    # 房间已在对局结束时销毁，点「确定」只做客户端导航：清本地状态后回大厅，
    # 不再向服务端发送离开房间请求。
    GameStore.reset_room()
    Net.relogin()
    UINavigation.navigate(self, UINavigation.PAGE_LOBBY)
