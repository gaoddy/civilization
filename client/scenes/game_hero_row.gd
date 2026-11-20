class_name GameHeroRow
extends HBoxContainer
## 英雄行：高 84，英雄卡在左，金币与本回合技能在右侧窄列。
##
## 只发技能按钮信号；是否可点由 `refresh(state)` 按页面门控决定。

signal hero_power_pressed
## 技能按钮按下：页面据此判断是否需要进入瞄准；抬起仍由 `pressed` 或瞄准层处理。
signal hero_power_down

## 悬停详情层，由对局页注入；英雄卡悬停时登记。
var detail: GameDetailLayer = null

var _hero_card: GameHeroCard
var _side: VBoxContainer
var _gold_tile: Label
var _power_button: GameIconButton
var _eliminated_label: Label
var _has_skill := false
var _in_combat := false


func _init() -> void:
    name = "HeroRow"
    custom_minimum_size = Vector2(0, UITheme.stage_metric("hero-row-height"))
    add_theme_constant_override("separation", UITheme.space("space-md"))
    alignment = BoxContainer.ALIGNMENT_CENTER

    _hero_card = GameHeroCard.new()
    _hero_card.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    add_child(_hero_card)

    _side = VBoxContainer.new()
    _side.name = "HeroSide"
    _side.custom_minimum_size = Vector2(UITheme.stage_metric("hero-card-width"), 0)
    _side.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    _side.alignment = BoxContainer.ALIGNMENT_CENTER
    _side.add_theme_constant_override("separation", UITheme.space("space-xs"))
    add_child(_side)

    _gold_tile = UI.pill_label("", "accent", "font-caption")
    _gold_tile.name = "GoldTile"
    _gold_tile.custom_minimum_size = Vector2(0, 24)
    _gold_tile.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _side.add_child(_gold_tile)

    _power_button = GameIconButton.new("hero_power", "HeroPower")
    _power_button.name = "HeroPowerButton"
    _power_button.uses_visible = true
    _power_button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    _power_button.pressed.connect(func() -> void: hero_power_pressed.emit())
    _power_button.button_down.connect(func() -> void: hero_power_down.emit())
    _side.add_child(_power_button)

    _eliminated_label = UI.label(tr("ui.game.side_panel.eliminated"), UITheme.font_size("font-body-sm"), false)
    _eliminated_label.name = "EliminatedLabel"
    _eliminated_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _eliminated_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _eliminated_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    _eliminated_label.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    _eliminated_label.visible = false
    add_child(_eliminated_label)


func refresh(state: GameUiState) -> void:
    var self_state := state.self_state
    var hero_id := CardText.optional_text(self_state.get("heroCardId"))
    var attack := int(CardDB.card(hero_id).get("attack", 0))
    _hero_card.refresh(
        hero_id,
        attack,
        int(self_state.get("hp", 0)),
        state.hp_cap,
        int(self_state.get("era", 1)),
    )
    if hero_id != "" and detail != null and is_instance_valid(detail):
        detail.track(_hero_card, {"detail_kind": "hero", "heroId": hero_id})

    _gold_tile.text = "%d %s" % [
        int(self_state.get("gold", 0)), tr("ui.game.side_panel.gold")
    ]
    var eliminated := _self_eliminated(state.view)
    _hero_card.visible = not eliminated
    _side.visible = not eliminated
    _eliminated_label.visible = eliminated
    _refresh_power(state, eliminated)


## 技能按钮的全局矩形：瞄准弧线的起点。
func power_rect() -> Rect2:
    return _power_button.get_global_rect()


func set_combat_mode(in_combat: bool) -> void:
    _in_combat = in_combat
    _power_button.visible = _has_skill and not in_combat


func _refresh_power(state: GameUiState, eliminated: bool) -> void:
    var power: Dictionary = state.self_state.get("heroPower", {})
    var skill_id := CardText.optional_text(power.get("skillId"))
    _has_skill = skill_id != ""
    _power_button.visible = _has_skill and not _in_combat and not eliminated
    if not _has_skill:
        return

    var aff: Dictionary = state.self_state.get("affordances", {})
    var uses := int(power.get("usesLeft", 0))
    var cost := CardDB.skill_cost(skill_id)
    var blocked := state.stale or state.expired or state.target_pending
    _power_button.set_cost(cost)
    _power_button.set_uses(uses)
    _power_button.disabled = blocked or not bool(aff.get("canUseHeroPower", false))
    _power_button.set_active(false)
    var lines := PackedStringArray([
        tr("ui.game.side_panel.hero_power"),
        tr("ui.game.side_panel.hero_power_uses") % uses,
    ])
    if cost >= 0:
        lines.append(tr("ui.game.detail.cost_value") % cost)
    var reason = aff.get("heroPowerDisabledReason")
    if reason != null:
        lines.append(UI.error_text(str(reason)))
    _power_button.tooltip_text = "\n".join(lines)


func _self_eliminated(view: Dictionary) -> bool:
    for player in view.get("players", []):
        if not (player is Dictionary):
            continue
        if str(player.get("playerId", "")) == GameStore.player_id:
            return not bool(player.get("alive", true))
    return false
