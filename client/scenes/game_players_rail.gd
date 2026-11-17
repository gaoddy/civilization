class_name GamePlayersRail
extends PanelContainer
## 左栏积分榜：按座位顺序列出所有玩家，本人行高亮、淘汰行压暗。
##
## 只读 `GameUiState.view`，不持有页面门控与意图。

var _rail: VBoxContainer


func _init() -> void:
    name = "PlayersPanel"
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
    custom_minimum_size = Vector2(UITheme.metric("rail-width"), 0)
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(box)
    box.add_child(UI.section_title("积分榜"))

    var scroll := ScrollContainer.new()
    scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    box.add_child(scroll)

    _rail = VBoxContainer.new()
    _rail.name = "PlayersRail"
    _rail.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _rail.add_theme_constant_override("separation", UITheme.space("space-xs"))
    scroll.add_child(_rail)


func refresh(state: GameUiState) -> void:
    UI.clear(_rail)
    var players: Array = _sorted_players(state.view)
    if players.is_empty():
        _rail.add_child(UI.empty_hint("暂无玩家"))
        return
    for player in players:
        _rail.add_child(_seat_row(player))


func _sorted_players(view: Dictionary) -> Array:
    var players: Array = view.get("players", []).duplicate()
    players.sort_custom(func(a, b): return int(a.get("seat", 0)) < int(b.get("seat", 0)))
    return players


func _seat_row(player: Dictionary) -> PanelContainer:
    var player_id := str(player.get("playerId", ""))
    var is_self := player_id == GameStore.player_id
    var alive := bool(player.get("alive", true))
    var panel := UI.card_panel("accent-row" if is_self else "row")
    panel.name = "SeatRow"
    panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    if not alive:
        UI.apply_dim(panel)

    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    panel.add_child(box)

    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-sm"))
    box.add_child(row)
    row.add_child(UI.avatar(GameCardFace.optional_text(player.get("heroCardId")), UITheme.metric("avatar-md")))

    var info := VBoxContainer.new()
    info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    info.add_theme_constant_override("separation", 0)
    row.add_child(info)

    var name_row := HBoxContainer.new()
    name_row.add_theme_constant_override("separation", UITheme.space("space-xs"))
    info.add_child(name_row)

    var name_label := UI.label(GameCardFace.optional_text(player.get("displayName")), UITheme.font_size("font-body-sm"), false)
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    if is_self:
        name_label.add_theme_color_override("font_color", UITheme.color("color-accent"))
    name_row.add_child(name_label)

    var marks := PackedStringArray()
    if is_self:
        marks.append("我")
    if player.get("placement") != null:
        marks.append("第 %d 名" % int(player.get("placement")))
    if not alive:
        marks.append("淘汰")
    elif bool(player.get("recruited", false)):
        marks.append("已招募")
    if not marks.is_empty():
        var mark := UI.label("、".join(marks), UITheme.font_size("font-caption"), false)
        mark.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
        name_row.add_child(mark)

    var meta := UI.label("HP %d · %s · 随从%d" % [
        int(player.get("hp", 0)),
        GameCardFace.era_text(int(player.get("era", 1))),
        int(player.get("boardCount", 0)),
    ], UITheme.font_size("font-caption"), false)
    meta.clip_text = true
    meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    info.add_child(meta)
    return panel
