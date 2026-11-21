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
        margin.add_theme_constant_override("margin_" + side, 24)
    panel.add_child(margin)

    var layout := VBoxContainer.new()
    layout.add_theme_constant_override("separation", 12)
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

    content.add_child(UIControls.label(tr("ui.result.title"), 28))
    content.add_child(UIControls.label(_my_place_text(), 20))
    content.add_child(HSeparator.new())
    content.add_child(_build_ranking())

    content.add_child(HSeparator.new())
    content.add_child(UIControls.label(tr("ui.result.stats_title"), 20))
    content.add_child(_build_stats_section())

    content.add_child(UIControls.label(tr("ui.result.seed") % GameStore.game_seed, 14))

    var actions := HBoxContainer.new()
    actions.name = "ResultActions"
    actions.add_theme_constant_override("separation", 10)
    layout.add_child(actions)

    var confirm := UIControls.button(tr("ui.result.confirm"))
    confirm.name = "ConfirmButton"
    confirm.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    confirm.pressed.connect(_on_confirm_pressed)
    actions.add_child(confirm)


func _build_ranking() -> VBoxContainer:
    var ranking := VBoxContainer.new()
    ranking.name = "RankingList"
    ranking.add_theme_constant_override("separation", 6)
    var entries: Array = GameStore.ranking.duplicate()
    entries.sort_custom(func(a, b): return int(a.get("place", 99)) < int(b.get("place", 99)))
    for entry in entries:
        var player_id := str(entry.get("playerId", ""))
        var row := UIControls.label(tr("ui.result.ranking_row") % [
            _int_of(entry, "place"),
            GameStore.player_name(player_id),
            _int_of(entry, "survivedTurns"),
            _int_of(entry, "damageDealt"),
        ], 18)
        if player_id == GameStore.player_id:
            row.modulate = Color("8fd3ff")
        ranking.add_child(row)
    return ranking


func _build_stats_section() -> VBoxContainer:
    var section := VBoxContainer.new()
    section.name = "StatsSection"
    section.add_theme_constant_override("separation", 6)
    var stats := _stats_data()

    section.add_child(_stat_row("ui.result.stat.survived_turns", _int_of(stats, "survivedTurns")))
    section.add_child(_stat_row("ui.result.stat.final_hp", _int_of(stats, "finalHp")))
    section.add_child(_stat_row(
        "ui.result.stat.final_era",
        CardDB.era_name(_int_of(stats, "finalEra")),
    ))
    section.add_child(_stat_row("ui.result.stat.remaining_gold", _int_of(stats, "remainingGold")))

    section.add_child(_group_label(tr("ui.result.stat.economy")))
    section.add_child(_stat_row("ui.result.stat.buys", _int_of(stats, "buys")))
    section.add_child(_stat_row("ui.result.stat.sells", _int_of(stats, "sells")))
    section.add_child(_stat_row("ui.result.stat.rerolls", _int_of(stats, "rerolls")))
    section.add_child(_stat_row("ui.result.stat.triples", _int_of(stats, "triples")))

    section.add_child(_group_label(tr("ui.result.stat.combat")))
    section.add_child(_stat_row("ui.result.stat.damage_dealt", _int_of(stats, "damageDealt")))
    section.add_child(_stat_row("ui.result.stat.damage_taken", _int_of(stats, "damageTaken")))
    section.add_child(_combat_row(stats))

    section.add_child(_group_label(tr("ui.result.stat.skills")))
    section.add_child(_stat_row("ui.result.stat.hero_power_uses", _int_of(stats, "heroPowerUses")))

    section.add_child(UIControls.label(tr("ui.result.board_title"), 18))
    section.add_child(_build_board(stats))
    return section


func _group_label(text: String) -> Label:
    var label := UIControls.label(text, 16)
    label.add_theme_color_override("font_color", Color("8fd3ff"))
    return label


func _stat_row(label_key: String, value) -> Label:
    var row := UIControls.label(tr("ui.result.stat_row") % [tr(label_key), str(value)], 16)
    row.name = "StatRow_" + label_key.get_slice(".", 3)
    return row


## 战斗胜负三项并排，避免在长列表里各占一行。
func _combat_row(stats: Dictionary) -> HBoxContainer:
    var row := HBoxContainer.new()
    row.name = "StatRow_combat"
    row.add_theme_constant_override("separation", 16)
    for entry in [
        ["ui.result.stat.combat_wins", "combatWins"],
        ["ui.result.stat.combat_draws", "combatDraws"],
        ["ui.result.stat.combat_losses", "combatLosses"],
    ]:
        var cell := UIControls.label(
            tr("ui.result.stat_row") % [tr(entry[0]), str(_int_of(stats, entry[1]))],
            16,
        )
        cell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        row.add_child(cell)
    return row


func _build_board(stats: Dictionary) -> VBoxContainer:
    var board := VBoxContainer.new()
    board.name = "BoardList"
    board.add_theme_constant_override("separation", 4)
    var items = stats.get("board", [])
    if not (items is Array) or items.is_empty():
        board.add_child(UIControls.label(tr("ui.result.board_empty"), 16))
        return board
    for item in items:
        if not (item is Dictionary):
            continue
        var card_id := str(item.get("cardId", ""))
        var card_name := CardDB.display_name(card_id)
        if bool(item.get("golden", false)):
            card_name += " ★"
        board.add_child(UIControls.label(tr("ui.result.board_item") % [
            card_name,
            _int_of(item, "attack"),
            _int_of(item, "health"),
        ], 16))
    return board


func _stats_data() -> Dictionary:
    var value = GameStore.self_stats
    return value if value is Dictionary else {}


## 结算字段一律直接取服务端下发值；缺失、空值按 0 处理。
func _int_of(source: Dictionary, key: String) -> int:
    var value = source.get(key, 0)
    if value == null:
        return 0
    return int(value)


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
