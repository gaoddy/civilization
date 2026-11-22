class_name GameOpponentView
extends HBoxContainer
## 战斗阶段顶部对手：84×84 英雄卡 + 84 宽身份列，与我方英雄行镜像。
##
## 只读展示，不发信号：对方为空显示等待配对；幽灵战压暗英雄卡、隐藏生命牌与血量条。

## 卡片详情登记入口，由对局页注入；独立实例化时可为 null。
var detail: DetailLayer = null

## 当前对手英雄卡：战斗结束对冲演出的对手方来源；等待配对时为 null。
var _hero_card: GameHeroCard = null


func _init() -> void:
    name = "OpponentView"
    custom_minimum_size = Vector2(0, UITheme.stage_metric("hero-card-size"))
    size_flags_horizontal = Control.SIZE_EXPAND_FILL
    alignment = BoxContainer.ALIGNMENT_CENTER
    add_theme_constant_override("separation", UITheme.space("space-md"))


func refresh(state: GameUiState) -> void:
    _hero_card = null
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


## 普通对手：英雄卡 + 身份列（名字/名次、HP 行、血量条）。
func _normal_block(player: Dictionary, hp_cap: int) -> Control:
    var block := HBoxContainer.new()
    block.name = "OpponentIdentity"
    block.alignment = BoxContainer.ALIGNMENT_CENTER
    block.add_theme_constant_override("separation", UITheme.space("space-md"))
    var portrait := _portrait(player, hp_cap, true)
    _hero_card = portrait
    block.add_child(portrait)
    block.add_child(_identity_column(player, hp_cap, false))
    return block


## 幽灵战：英雄卡隐藏生命牌，身份列带幽灵角标与时代/随从数，整块压暗。
func _ghost_block(player: Dictionary, hp_cap: int) -> Control:
    var block := HBoxContainer.new()
    block.name = "GhostIdentity"
    block.alignment = BoxContainer.ALIGNMENT_CENTER
    block.add_theme_constant_override("separation", UITheme.space("space-md"))
    var portrait := _portrait(player, hp_cap, false)
    _hero_card = portrait
    block.add_child(portrait)
    block.add_child(_identity_column(player, hp_cap, true))
    UIControls.apply_dim(block)
    return block


## 对手英雄卡：节点已释放或等待配对时返回 null。
func hero_card() -> Control:
    if _hero_card == null or not is_instance_valid(_hero_card):
        return null
    return _hero_card


func _portrait(player: Dictionary, hp_cap: int, show_hp: bool) -> GameHeroCard:
    var hero_id := CardText.optional_text(player.get("heroCardId"))
    var portrait := GameHeroCard.new()
    portrait.name = "OpponentPortrait"
    # 手机档位身份列换行后比英雄卡高；英雄卡按最小尺寸居中，不随容器拉伸成长方形。
    portrait.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    portrait.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    portrait.refresh(
        hero_id,
        int(player.get("hp", 0)) if show_hp else 0,
        hp_cap if show_hp else 0,
    )
    if hero_id != "" and detail != null and is_instance_valid(detail):
        detail.track(portrait, {"detail_kind": "hero", "heroId": hero_id})
    return portrait


func _identity_column(player: Dictionary, hp_cap: int, ghost: bool) -> VBoxContainer:
    var width := UITheme.stage_metric("opponent-hp-width")
    var column := VBoxContainer.new()
    column.custom_minimum_size = Vector2(width, 0)
    column.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    column.add_theme_constant_override("separation", UITheme.space("space-2xs"))

    var name_row := HBoxContainer.new()
    name_row.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    column.add_child(name_row)
    var display_name := CardText.optional_text(player.get("displayName"))
    if display_name == "":
        display_name = CardText.optional_text(player.get("playerId"))
    var name_label := UIControls.label(display_name, UITheme.font_size("font-body-sm"), false)
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    name_row.add_child(name_label)
    var rank = player.get("rank")
    if rank != null:
        var rank_badge := UIControls.pill_label(tr("ui.game.side_panel.rank") % int(rank), "accent", "font-caption")
        rank_badge.name = "RankBadge"
        name_row.add_child(rank_badge)

    if ghost:
        var ghost_badge := UIControls.pill_label(tr("ui.game.side_panel.ghost"), "info", "font-caption")
        ghost_badge.name = "GhostBadge"
        ghost_badge.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
        column.add_child(ghost_badge)
        column.add_child(_meta_label(tr("ui.game.side_panel.era_board") % [
            CardText.era_text(int(player.get("era", 1))),
            int(player.get("boardCount", 0)),
        ]))
    else:
        var meta := _meta_label(tr("ui.game.side_panel.hp_line") % [
            int(player.get("hp", 0)),
            maxi(hp_cap, 1),
            CardText.era_text(int(player.get("era", 1))),
            int(player.get("boardCount", 0)),
        ])
        # 84 宽身份列放不下整行 HP 文案，允许在词间换行，避免裁切丢失时代与随从数。
        meta.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
        column.add_child(meta)
        column.add_child(
            HpBar.create(
                int(player.get("hp", 0)),
                hp_cap,
                width,
            )
        )
    return column


func _meta_label(text: String) -> Label:
    var meta := UIControls.label(text, UITheme.font_size("font-caption"), false)
    meta.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    return meta


func _player(view: Dictionary, player_id: String) -> Dictionary:
    for player in view.get("players", []):
        if player is Dictionary and str(player.get("playerId", "")) == player_id:
            return player
    return {}
