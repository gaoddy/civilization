extends Control
## 结算：展示服务端下发的排名、本人本局数据与对局种子。
##
## 对局结束时服务端已销毁房间，本页只读本局下发内容；内容区可滚动，
## 「确定」固定在底部，点一次即回大厅。

## 段位顺序，只用于判断晋级/降级方向；段位区间本身由服务端配置决定。
const TIER_ORDER := [
    "apprentice", "bronze", "silver", "gold", "platinum", "diamond", "master", "grandmaster",
]


func _ready() -> void:
    UILayout.safe_area(self)
    _build()


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
    content.add_theme_constant_override("separation", UITheme.space("space-md-plus"))
    scroll.add_child(content)

    content.add_child(UIControls.label(tr("ui.result.title"), UITheme.font_size("font-hero")))
    content.add_child(UIControls.label(_my_place_text(), UITheme.font_size("font-title-xl")))
    var rating_block := _rating_block()
    if rating_block != null:
        content.add_child(rating_block)
    content.add_child(HSeparator.new())
    content.add_child(ResultRanking.new(
        GameStore.ranking.duplicate(), GameStore.player_id, Callable(GameStore, "player_name")
    ))

    content.add_child(HSeparator.new())
    content.add_child(UIControls.label(tr("ui.result.stats_title"), UITheme.font_size("font-title-xl")))
    content.add_child(ResultStatsSection.new(_stats_data()))

    content.add_child(UIControls.label(tr("ui.result.seed") % GameStore.game_seed, UITheme.font_size("font-body")))

    var actions := HBoxContainer.new()
    actions.name = "ResultActions"
    actions.add_theme_constant_override("separation", UITheme.space("space-md-plus"))
    layout.add_child(actions)

    var confirm := UIControls.button(tr("ui.result.confirm"))
    confirm.name = "ConfirmButton"
    confirm.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    confirm.pressed.connect(_on_confirm_pressed)
    actions.add_child(confirm)


## 天梯结算块：房间局四项为 null 时不展示；积分与段位全部取服务端下发值。
func _rating_block() -> VBoxContainer:
    if GameStore.rating_delta == null:
        return null
    var block := VBoxContainer.new()
    block.name = "RatingBlock"
    block.add_theme_constant_override("separation", UITheme.space("space-sm"))
    block.add_child(UIControls.label(tr("ui.result.rating_title"), UITheme.font_size("font-title-xl")))

    var delta := int(GameStore.rating_delta)
    var signed_delta := ("+" if delta >= 0 else "") + str(delta)
    var after := 0 if GameStore.rating_after == null else int(GameStore.rating_after)
    var delta_row := UIControls.label(
        tr("ui.result.rating_delta") % [signed_delta, after],
        UITheme.font_size("font-title-lg"),
    )
    delta_row.name = "RatingDelta"
    block.add_child(delta_row)

    if GameStore.tier_before != "" and GameStore.tier_after != "":
        var before_text := UIWording.tier_text(GameStore.tier_before)
        var after_text := UIWording.tier_text(GameStore.tier_after)
        var tier_row := UIControls.label(
            tr("ui.result.tier_change") % [before_text, after_text],
            UITheme.font_size("font-title"),
        )
        tier_row.name = "TierChange"
        block.add_child(tier_row)
        var direction := _tier_direction(GameStore.tier_before, GameStore.tier_after)
        if direction != "":
            var hint := UIControls.label(tr(direction), UITheme.font_size("font-title"))
            hint.name = "TierTrend"
            hint.add_theme_color_override("font_color", UITheme.color("color-highlight"))
            block.add_child(hint)
    return block


## 段位方向：升序表内前后比较；不在表内或同级返回空串。
static func _tier_direction(before: String, after: String) -> String:
    var before_index := TIER_ORDER.find(before)
    var after_index := TIER_ORDER.find(after)
    if before_index < 0 or after_index < 0 or before_index == after_index:
        return ""
    return "ui.result.promoted" if after_index > before_index else "ui.result.demoted"


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
