class_name ResultStatsSection
extends VBoxContainer
## 结算数据分区：生存、经济、战斗、技能与最终阵容。
##
## 只按服务端下发的本人本局数据排版，段内字段缺失按 0 处理。


func _init(stats: Dictionary) -> void:
    name = "StatsSection"
    add_theme_constant_override("separation", UITheme.space("space-sm"))

    add_child(_stat_row("ui.result.stat.survived_turns", _int_of(stats, "survivedTurns")))
    add_child(_stat_row("ui.result.stat.final_hp", _int_of(stats, "finalHp")))
    add_child(_stat_row(
        "ui.result.stat.final_era",
        CardDB.era_name(_int_of(stats, "finalEra")),
    ))
    add_child(_stat_row("ui.result.stat.remaining_gold", _int_of(stats, "remainingGold")))

    add_child(_group_label(tr("ui.result.stat.economy")))
    add_child(_stat_row("ui.result.stat.buys", _int_of(stats, "buys")))
    add_child(_stat_row("ui.result.stat.sells", _int_of(stats, "sells")))
    add_child(_stat_row("ui.result.stat.rerolls", _int_of(stats, "rerolls")))
    add_child(_stat_row("ui.result.stat.triples", _int_of(stats, "triples")))

    add_child(_group_label(tr("ui.result.stat.combat")))
    add_child(_stat_row("ui.result.stat.damage_dealt", _int_of(stats, "damageDealt")))
    add_child(_stat_row("ui.result.stat.damage_taken", _int_of(stats, "damageTaken")))
    add_child(_combat_row(stats))

    add_child(_group_label(tr("ui.result.stat.skills")))
    add_child(_stat_row("ui.result.stat.hero_power_uses", _int_of(stats, "heroPowerUses")))

    add_child(UIControls.label(tr("ui.result.board_title"), UITheme.font_size("font-title-lg")))
    var board = stats.get("board", [])
    add_child(ResultBoard.new(board if board is Array else []))


func _group_label(text: String) -> Label:
    var label := UIControls.label(text, UITheme.font_size("font-title"))
    label.add_theme_color_override("font_color", Color("8fd3ff"))
    return label


func _stat_row(label_key: String, value) -> Label:
    var row := UIControls.label(tr("ui.result.stat_row") % [tr(label_key), str(value)], UITheme.font_size("font-title"))
    row.name = "StatRow_" + label_key.get_slice(".", 3)
    return row


## 战斗胜负三项并排，避免在长列表里各占一行。
func _combat_row(stats: Dictionary) -> HBoxContainer:
    var row := HBoxContainer.new()
    row.name = "StatRow_combat"
    row.add_theme_constant_override("separation", UITheme.space("space-xl"))
    for entry in [
        ["ui.result.stat.combat_wins", "combatWins"],
        ["ui.result.stat.combat_draws", "combatDraws"],
        ["ui.result.stat.combat_losses", "combatLosses"],
    ]:
        var cell := UIControls.label(
            tr("ui.result.stat_row") % [tr(entry[0]), str(_int_of(stats, entry[1]))],
            UITheme.font_size("font-title"),
        )
        cell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        row.add_child(cell)
    return row


## 结算字段一律直接取服务端下发值；缺失、空值按 0 处理。
static func _int_of(source: Dictionary, key: String) -> int:
    var value = source.get(key, 0)
    if value == null:
        return 0
    return int(value)
