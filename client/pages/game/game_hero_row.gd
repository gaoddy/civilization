class_name GameHeroRow
extends HBoxContainer
## 英雄行：高 84，英雄卡在左，金币、被动技能标记与全部主动技能在右侧窄列。
##
## 只发技能按钮信号（带技能 id）；是否可点由 `refresh(state)` 按页面门控决定。
## 被动技能只按英雄卡展示，不参与交互、不置灰，也不发信号。

## 技能按钮被点按：参数为该技能的 id。
signal hero_power_pressed(skill_id: String)
## 技能按钮按下：页面据此判断是否需要进入瞄准；抬起仍由 `pressed` 或瞄准层处理。
signal hero_power_down(skill_id: String)

## 悬停详情层，由对局页注入；英雄卡悬停时登记。
var detail: GameDetailLayer = null

var _hero_card: GameHeroCard
var _side: VBoxContainer
var _meta: HBoxContainer
var _gold_tile: Label
var _passives: HBoxContainer
var _passive_chips: Dictionary = {}
var _powers: HBoxContainer
var _power_buttons: Dictionary = {}
var _eliminated_label: Label
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

    _meta = HBoxContainer.new()
    _meta.name = "HeroMeta"
    _meta.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    _meta.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _side.add_child(_meta)

    _passives = HBoxContainer.new()
    _passives.name = "HeroPassives"
    _passives.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    _passives.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _meta.add_child(_passives)

    _gold_tile = UI.pill_label("", "accent", "font-caption")
    _gold_tile.name = "GoldTile"
    _gold_tile.custom_minimum_size = Vector2(
        0, UITheme.stage_metric("hero-meta-row-height")
    )
    _gold_tile.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _meta.add_child(_gold_tile)

    _powers = HBoxContainer.new()
    _powers.name = "HeroPowers"
    _powers.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    _powers.add_theme_constant_override("separation", UITheme.space("space-xs"))
    _side.add_child(_powers)

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
    _hero_card.refresh(
        hero_id,
        int(self_state.get("hp", 0)),
        state.hp_cap,
        int(self_state.get("era", 1)),
    )
    if hero_id != "" and detail != null and is_instance_valid(detail):
        detail.track(_hero_card, {"detail_kind": "hero", "heroId": hero_id})

    _gold_tile.text = "%d %s" % [
        int(self_state.get("gold", 0)), tr("ui.game.side_panel.gold")
    ]
    var eliminated := state.self_eliminated()
    _hero_card.visible = not eliminated
    _side.visible = not eliminated
    _eliminated_label.visible = eliminated
    _sync_passive_chips(hero_id)
    _refresh_powers(state, eliminated)


## 技能按钮的全局矩形：瞄准弧线的起点；未知技能返回空矩形。
func power_rect(skill_id: String) -> Rect2:
    var button: Control = _power_buttons.get(skill_id)
    return button.get_global_rect() if button != null else Rect2()


func set_combat_mode(in_combat: bool) -> void:
    _in_combat = in_combat
    for button in _power_buttons.values():
        button.visible = not in_combat


func _refresh_powers(state: GameUiState, eliminated: bool) -> void:
    var powers: Array = state.self_state.get("heroPowers", [])
    _sync_power_buttons(powers)
    var blocked := state.stale or state.expired or state.target_pending
    for power in powers:
        if not (power is Dictionary):
            continue
        var skill_id := CardText.optional_text(power.get("skillId"))
        var button: GameIconButton = _power_buttons.get(skill_id)
        if button == null:
            continue
        var uses := int(power.get("usesLeft", 0))
        var cost := CardDB.skill_cost(skill_id)
        button.visible = not _in_combat and not eliminated
        button.set_cost(cost)
        button.set_uses(uses)
        button.disabled = blocked or not bool(power.get("canUse", false))
        button.set_active(false)
        var lines := PackedStringArray([
            CardDB.skill_name(skill_id),
            tr("ui.game.side_panel.hero_power_uses") % uses,
        ])
        if cost >= 0:
            lines.append(tr("ui.game.detail.cost_value") % cost)
        var reason = power.get("disabledReason")
        if reason != null:
            lines.append(UI.error_text(str(reason)))
        button.tooltip_text = "\n".join(lines)


## 按英雄卡的被动技能顺序保持标记集合；英雄变化时重建。
func _sync_passive_chips(hero_id: String) -> void:
    var ids := PackedStringArray()
    for raw_id in CardDB.hero(hero_id).get("skills", []):
        var skill_id := str(raw_id)
        var skill := CardDB.skill(skill_id)
        if skill.is_empty() or skill.get("cost") != null:
            continue
        ids.append(skill_id)
    var current := PackedStringArray()
    for skill_id in _passive_chips:
        current.append(skill_id)
    if current == ids:
        return
    for chip in _passive_chips.values():
        _passives.remove_child(chip)
        chip.queue_free()
    _passive_chips.clear()
    for index in ids.size():
        var skill_id := ids[index]
        var chip := PassiveSkillChip.new()
        chip.name = "HeroPassive%d" % index
        chip.tooltip_text = _passive_tooltip(skill_id)
        _passives.add_child(chip)
        _passive_chips[skill_id] = chip


## 被动技能悬停文案：先标明被动，再给名称与完整效果。
func _passive_tooltip(skill_id: String) -> String:
    var label := tr("ui.game.side_panel.passive_skill")
    var skill_title := CardDB.skill_name(skill_id)
    var lines := PackedStringArray([
        label if skill_title == "" else "%s：%s" % [label, skill_title],
    ])
    var key := "skill.%s.text" % skill_id
    var text := tr(key)
    if text != key:
        lines.append(text)
    return "\n".join(lines)


## 按视图中主动技能的 id 与顺序保持按钮集合；列表变化时重建。
func _sync_power_buttons(powers: Array) -> void:
    var ids := PackedStringArray()
    for power in powers:
        if not (power is Dictionary):
            continue
        var skill_id := CardText.optional_text(power.get("skillId"))
        if skill_id != "":
            ids.append(skill_id)
    var current := PackedStringArray()
    for skill_id in _power_buttons:
        current.append(skill_id)
    if current == ids:
        return
    for button in _power_buttons.values():
        _powers.remove_child(button)
        button.queue_free()
    _power_buttons.clear()
    for index in ids.size():
        var skill_id := ids[index]
        var button := GameIconButton.new("hero_power", "HeroPower%d" % index)
        button.name = "HeroPowerButton%d" % index
        button.uses_visible = true
        button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
        button.pressed.connect(func() -> void: hero_power_pressed.emit(skill_id))
        button.button_down.connect(func() -> void: hero_power_down.emit(skill_id))
        _powers.add_child(button)
        _power_buttons[skill_id] = button


## 被动技能标记：与金币同高的静态格，只提供悬停文案，不发信号、不置灰。
class PassiveSkillChip extends Control:
    const GLYPH_INSET := 4.0
    const GLYPH_STROKE := 1.6

    var _box: StyleBoxFlat

    func _init() -> void:
        var side := float(UITheme.stage_metric("hero-meta-row-height"))
        custom_minimum_size = Vector2(side, side)
        size = Vector2(side, side)
        mouse_filter = Control.MOUSE_FILTER_STOP
        focus_mode = Control.FOCUS_NONE
        _box = UITheme.panel_style("sunken")
        _box.set_corner_radius_all(UITheme.radius("radius-sm"))
        _box.border_color = UITheme.color("color-border")
        _box.set_border_width_all(1)
        queue_redraw()

    func _draw() -> void:
        draw_style_box(_box, Rect2(Vector2.ZERO, size))
        Icons.draw(
            self,
            "passive_skill",
            Rect2(GLYPH_INSET, GLYPH_INSET, size.x - GLYPH_INSET * 2.0, size.y - GLYPH_INSET * 2.0),
            UITheme.color("color-text"),
            GLYPH_STROKE,
        )
