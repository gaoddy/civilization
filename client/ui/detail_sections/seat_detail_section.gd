class_name SeatDetailSection
extends RefCounted
## 座位详情：顶部紧凑身份（名字、名次、英雄 · 时代 · 随从），
## 下方生命条（当前/上限数字叠在条上）与对战记录；对战记录不设标题，
## 一行一条，没有条目时只显示「无战报」。

const META_SEPARATOR := " · "


static func render(
    box: VBoxContainer,
    player: Dictionary,
    hp_cap: int,
    view: Dictionary,
    kit: DetailKit
) -> void:
    box.add_child(_header(player, kit))
    box.add_child(_meta_line(player, kit))
    box.add_child(_hp_bar(player, hp_cap, kit))
    box.add_child(_divider())
    box.add_child(_combat_log(player, view))


## 名字与名次同一行：名字占满剩余宽度，名次缺失时不显示。
static func _header(player: Dictionary, kit: DetailKit) -> HBoxContainer:
    var row := HBoxContainer.new()
    row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    var player_name := kit.name_label(CardText.optional_text(player.get("displayName")))
    player_name.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    player_name.custom_minimum_size = Vector2.ZERO
    player_name.clip_text = true
    row.add_child(player_name)
    if player.get("rank") != null:
        var rank := _caption(
            TranslationServer.translate("ui.game.side_panel.rank") % int(player.get("rank")),
            "color-text-muted",
        )
        rank.name = "DetailRank"
        rank.size_flags_vertical = Control.SIZE_SHRINK_CENTER
        row.add_child(rank)
    return row


## 英雄、时代、随从数压成一行元信息，不各自占一行。
static func _meta_line(player: Dictionary, kit: DetailKit) -> Label:
    var parts := PackedStringArray([
        CardDB.display_name(CardText.optional_text(player.get("heroCardId"))),
        CardDB.era_name(int(player.get("era", 1))),
        "%s %d" % [
            TranslationServer.translate("ui.game.detail.minions"),
            int(player.get("boardCount", 0)),
        ],
    ])
    var line := UIControls.label(META_SEPARATOR.join(parts), UITheme.font_size("font-caption"))
    line.name = "DetailMeta"
    line.custom_minimum_size = Vector2(kit.text_width, 0)
    line.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    line.mouse_filter = Control.MOUSE_FILTER_IGNORE
    line.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    return line


## 生命条占满面板内容宽度，圆角取高度一半成胶囊；当前/上限数字居中叠在条上，
## 不再另起「生命」行。
static func _hp_bar(player: Dictionary, hp_cap: int, kit: DetailKit) -> Control:
    var height := UITheme.stage_metric("seat-detail-hp-bar-height")
    var hp := int(player.get("hp", 0))
    var cap := maxi(hp_cap, 0)
    var bar := HpBar.create(
        hp,
        cap,
        kit.text_width,
        int(height * 0.5),
        height,
        "%d/%d" % [hp, cap],
    )
    bar.name = "DetailHpBar"
    return bar


## 分隔线：1px `color-border`，上下各留 `space-sm`，把身份区与对战记录分开。
static func _divider() -> Control:
    var margin := MarginContainer.new()
    margin.name = "DetailDivider"
    margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
    margin.add_theme_constant_override("margin_top", UITheme.space("space-sm"))
    margin.add_theme_constant_override("margin_bottom", UITheme.space("space-sm"))
    var line := ColorRect.new()
    line.color = UITheme.color("color-border")
    line.custom_minimum_size = Vector2(0, 1)
    line.mouse_filter = Control.MOUSE_FILTER_IGNORE
    margin.add_child(line)
    return margin


## 对战记录：不设标题，按回合升序逐条列出；一条一行，对手名过长时省略。
static func _combat_log(player: Dictionary, view: Dictionary) -> VBoxContainer:
    var log_box := VBoxContainer.new()
    log_box.name = "SeatCombatLog"
    log_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    log_box.add_theme_constant_override("separation", UITheme.space("space-2xs"))

    var player_id := str(player.get("playerId", ""))
    var entries := combat_entries(view, player_id)
    if entries.is_empty():
        log_box.add_child(_caption(
            TranslationServer.translate("ui.game.rail.no_report"),
            "color-text-muted",
        ))
        return log_box
    var names := player_names(view)
    for entry in entries:
        log_box.add_child(_entry_line(entry, player_id, names))
    return log_box


## 「第 N 回合 · 对手名 · 胜/负/平 · 伤害 X」拆成一行内的多段：
## 回合与伤害弱化，对手名占中间剩余宽度，胜负用语义色强调。
static func _entry_line(entry: Dictionary, player_id: String, names: Dictionary) -> HBoxContainer:
    var line := HBoxContainer.new()
    line.name = "SeatCombatEntry_%d" % int(entry.get("turn", 0))
    line.mouse_filter = Control.MOUSE_FILTER_IGNORE
    line.add_theme_constant_override("separation", UITheme.space("space-xs"))

    line.add_child(_caption(
        TranslationServer.translate("ui.game.rail.turn") % int(entry.get("turn", 0)),
        "color-text-muted",
    ))
    var opponent := _caption(_opponent_text(entry, player_id, names), "color-text-dim")
    opponent.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    opponent.custom_minimum_size = Vector2.ZERO
    opponent.clip_text = true
    opponent.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    line.add_child(opponent)

    var result := _result(entry, player_id)
    line.add_child(_caption(str(result["text"]), str(result["color"])))

    var damage := int(entry.get("damage", 0))
    if damage != 0:
        line.add_child(_caption(
            TranslationServer.translate("ui.game.rail.damage") % damage,
            "color-text-dim",
        ))
    return line


static func _caption(text: String, color_name: String) -> Label:
    var label := UIControls.label(text, UITheme.font_size("font-caption"), false)
    label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    label.add_theme_color_override("font_color", UITheme.color(color_name))
    return label


## 该座位参与过的战报，保持 `view.combatLog` 的回合升序。
static func combat_entries(view: Dictionary, player_id: String) -> Array:
    var entries: Array = []
    for entry in view.get("combatLog", []):
        if not (entry is Dictionary):
            continue
        for pid in entry.get("participantIds", []):
            if str(pid) == player_id:
                entries.append(entry)
                break
    return entries


static func player_names(view: Dictionary) -> Dictionary:
    var names := {}
    for player in view.get("players", []):
        if player is Dictionary:
            names[str(player.get("playerId", ""))] = CardText.optional_text(
                player.get("displayName")
            )
    return names


## 幽灵战对手写「幽灵（名字）」；`ghostId` 为空时只写「幽灵」，不写空括号。
static func _opponent_text(entry: Dictionary, player_id: String, names: Dictionary) -> String:
    if bool(entry.get("mirror", false)):
        var ghost_id := "" if entry.get("ghostId") == null else str(entry.get("ghostId"))
        if ghost_id == "":
            return TranslationServer.translate("ui.game.rail.ghost")
        return TranslationServer.translate("ui.game.rail.ghost_named") % str(
            names.get(ghost_id, ghost_id)
        )
    var opponent_id := ""
    for pid in entry.get("participantIds", []):
        if str(pid) != player_id:
            opponent_id = str(pid)
            break
    return str(names.get(opponent_id, opponent_id))


static func _result(entry: Dictionary, player_id: String) -> Dictionary:
    var winner_id := "" if entry.get("winnerId") == null else str(entry.get("winnerId"))
    var loser_id := "" if entry.get("loserId") == null else str(entry.get("loserId"))
    if winner_id == player_id:
        return {"text": TranslationServer.translate("ui.game.rail.win"), "color": "color-success"}
    if loser_id == player_id or winner_id != "":
        return {"text": TranslationServer.translate("ui.game.rail.lose"), "color": "color-danger"}
    return {"text": TranslationServer.translate("ui.game.rail.draw"), "color": "color-text-muted"}
