class_name GameOpponentView
extends HBoxContainer
## 战斗阶段顶部对手：64×64 头像 + 名字、英雄、HP 行与血量条。
##
## 只读展示，不发信号：对方为空显示等待配对；幽灵战只换身份块并去掉血量条。

## 卡片详情登记入口，由对局页注入；独立实例化时可为 null。
var detail: GameDetailLayer = null


func _init() -> void:
    name = "OpponentView"
    custom_minimum_size = Vector2(0, UITheme.stage_metric("portrait-size"))
    size_flags_horizontal = Control.SIZE_EXPAND_FILL
    alignment = BoxContainer.ALIGNMENT_CENTER
    add_theme_constant_override("separation", UITheme.space("space-md"))


func refresh(state: GameUiState) -> void:
    UIControls.clear(self)
    var opponent_id := CardText.optional_text(state.view.get("opponentPlayerId"))
    if opponent_id == "":
        add_child(UIControls.empty_hint(tr("ui.game.side_panel.waiting_match")))
        return
    var player := _player(state.view, opponent_id)
    var ghost := GameStore.combat_is_current() and bool(GameStore.last_combat.get("mirror", false))
    if ghost:
        add_child(_ghost_block(player, state.hp_cap))
    else:
        add_child(_normal_block(player, state.hp_cap))


## 普通对手：头像 + 身份列（名字、英雄、HP 行、血量条）。
func _normal_block(player: Dictionary, hp_cap: int) -> Control:
    var block := HBoxContainer.new()
    block.name = "OpponentIdentity"
    block.alignment = BoxContainer.ALIGNMENT_CENTER
    block.add_theme_constant_override("separation", UITheme.space("space-md"))
    block.add_child(_portrait(player))
    block.add_child(_identity_column(player, hp_cap, false))
    return block


## 幽灵战：身份列带幽灵角标、无血量条，整块压暗。
func _ghost_block(player: Dictionary, hp_cap: int) -> Control:
    var block := HBoxContainer.new()
    block.name = "GhostIdentity"
    block.alignment = BoxContainer.ALIGNMENT_CENTER
    block.add_theme_constant_override("separation", UITheme.space("space-md"))
    block.add_child(_portrait(player))
    block.add_child(_identity_column(player, hp_cap, true))
    UIControls.apply_dim(block)
    return block


func _portrait(player: Dictionary) -> GamePortrait:
    var hero_id := CardText.optional_text(player.get("heroCardId"))
    var display := CardDB.display_name(hero_id) if hero_id != "" else tr("ui.game.side_panel.unknown_hero")
    var tint := (
        UITheme.playstyle_color(CardDB.primary_playstyle(CardDB.hero(hero_id)))
        if hero_id != ""
        else UITheme.color("color-border-strong")
    )
    var portrait := GamePortrait.new()
    portrait.name = "OpponentPortrait"
    var art_path := CardDB.art_path(hero_id, "avatar") if hero_id != "" else ""
    portrait.setup(display, display.substr(0, 1) if display != "" else "?", tint, false, art_path)
    if hero_id != "" and detail != null and is_instance_valid(detail):
        detail.track(portrait, {"detail_kind": "hero", "heroId": hero_id})
    return portrait


func _identity_column(player: Dictionary, hp_cap: int, ghost: bool) -> VBoxContainer:
    var column := VBoxContainer.new()
    column.custom_minimum_size = Vector2(UITheme.metric("opponent-hp-width"), 0)
    column.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    column.add_theme_constant_override("separation", UITheme.space("space-2xs"))

    var name_row := HBoxContainer.new()
    name_row.add_theme_constant_override("separation", UITheme.space("space-xs"))
    column.add_child(name_row)
    var display_name := CardText.optional_text(player.get("displayName"))
    if display_name == "":
        display_name = CardText.optional_text(player.get("playerId"))
    var name_label := UIControls.label(display_name, UITheme.font_size("font-body-sm"), false)
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    name_row.add_child(name_label)
    if ghost:
        var ghost_badge := UIControls.pill_label(tr("ui.game.side_panel.ghost"), "info", "font-caption")
        ghost_badge.name = "GhostBadge"
        name_row.add_child(ghost_badge)
    var rank = player.get("rank")
    if rank != null:
        var rank_badge := UIControls.pill_label(tr("ui.game.side_panel.rank") % int(rank), "accent", "font-caption")
        rank_badge.name = "RankBadge"
        name_row.add_child(rank_badge)

    var hero_id := CardText.optional_text(player.get("heroCardId"))
    var hero_label := UIControls.label(
        CardDB.display_name(hero_id) if hero_id != "" else tr("ui.game.side_panel.unknown_hero"),
        UITheme.font_size("font-body-sm"),
        false,
    )
    hero_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    column.add_child(hero_label)

    var meta := UIControls.label(tr("ui.game.side_panel.hp_line") % [
        int(player.get("hp", 0)),
        maxi(hp_cap, 1),
        CardText.era_text(int(player.get("era", 1))),
        int(player.get("boardCount", 0)),
    ], UITheme.font_size("font-caption"), false)
    meta.clip_text = true
    meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    column.add_child(meta)

    if not ghost:
        column.add_child(
            HpBar.create(
                int(player.get("hp", 0)),
                hp_cap,
                UITheme.metric("opponent-hp-width"),
            )
        )
    return column


func _player(view: Dictionary, player_id: String) -> Dictionary:
    for player in view.get("players", []):
        if player is Dictionary and str(player.get("playerId", "")) == player_id:
            return player
    return {}
