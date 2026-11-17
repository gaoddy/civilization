class_name GameSidePanel
extends VBoxContainer
## 右栏：场景身份、回合与倒计时、自己的英雄与操作。
##
## 只发按钮信号，不发送意图；是否禁用由 `refresh(state)` 按页面门控决定。

signal hero_power_pressed
signal end_recruit_pressed

var _identity_box: VBoxContainer
var _turn_label: Label
var _timer_label: Label
var _hero_detail: VBoxContainer
var _hero_power_button: Button
var _end_button: Button

## 当前阶段截止时间；空串表示没有可展示的倒计时。
var _deadline := ""
var _time_left := -1


func _init() -> void:
    name = "SidePanel"
    custom_minimum_size = Vector2(UITheme.metric("side-width"), 0)
    add_theme_constant_override("separation", UITheme.space("space-sm"))

    var identity_panel := UI.card_panel("surface")
    identity_panel.name = "SideIdentity"
    _identity_box = VBoxContainer.new()
    _identity_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    identity_panel.add_child(_identity_box)
    add_child(identity_panel)

    var status_panel := UI.card_panel("surface")
    status_panel.name = "SideStatus"
    var status_box := VBoxContainer.new()
    status_box.add_theme_constant_override("separation", UITheme.space("space-xs"))
    status_panel.add_child(status_box)
    _turn_label = UI.label("", UITheme.font_size("font-title-lg"), false)
    _turn_label.name = "TurnLabel"
    status_box.add_child(_turn_label)
    _timer_label = UI.pill_label("—", "accent", "font-title")
    _timer_label.name = "TimerLabel"
    status_box.add_child(_timer_label)
    add_child(status_panel)

    var hero_panel := UI.card_panel("surface")
    hero_panel.name = "SideHero"
    hero_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    var hero_box := VBoxContainer.new()
    hero_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    hero_panel.add_child(hero_box)
    add_child(hero_panel)

    _hero_power_button = UI.styled_button("英雄技能", "default", 40)
    _hero_power_button.name = "HeroPowerButton"
    _hero_power_button.pressed.connect(func() -> void: hero_power_pressed.emit())
    hero_box.add_child(_hero_power_button)

    _end_button = UI.styled_button("结束招募", "primary", 46)
    _end_button.name = "EndButton"
    _end_button.pressed.connect(func() -> void: end_recruit_pressed.emit())
    hero_box.add_child(_end_button)

    _hero_detail = VBoxContainer.new()
    _hero_detail.name = "HeroDetail"
    _hero_detail.add_theme_constant_override("separation", UITheme.space("space-sm"))
    hero_box.add_child(_hero_detail)


func _process(_delta: float) -> void:
    _update_countdown()


func refresh(state: GameUiState) -> void:
    var view_deadline := str(state.view.get("phaseDeadline", ""))
    _deadline = view_deadline if view_deadline != "" else GameStore.deadline
    _refresh_identity(state)
    _refresh_status(state)
    _refresh_hero(state)
    _refresh_actions(state)
    _update_countdown()


func set_combat_mode(in_combat: bool) -> void:
    _hero_power_button.visible = not in_combat
    _end_button.visible = not in_combat


## 当前阶段剩余秒数；没有截止时间时返回 -1。
func seconds_left() -> int:
    return _seconds_left()


func _seconds_left() -> int:
    if _deadline == "":
        return -1
    var deadline := GameCombatView.unix_from_rfc3339(_deadline)
    if deadline <= 0.0:
        return -1
    return int(ceil(deadline - Time.get_unix_time_from_system()))


func _update_countdown() -> void:
    _time_left = _seconds_left()
    var text := "—"
    if _time_left > 0:
        text = "剩余 %d 秒" % _time_left
    elif _time_left == 0:
        text = "等待结算"
    if _timer_label.text != text:
        _timer_label.text = text


func _refresh_status(state: GameUiState) -> void:
    _turn_label.text = "第 %d 回合" % int(state.view.get("turn", GameStore.turn))


func _refresh_identity(state: GameUiState) -> void:
    UI.clear(_identity_box)
    if state.phase == Protocol.PHASE_RECRUIT:
        _identity_box.add_child(_seller_block())
        return

    var self_player := _player(state.view, GameStore.player_id)
    if not self_player.is_empty() and not bool(self_player.get("alive", true)):
        _identity_box.add_child(UI.label("已淘汰 · 观战中", UITheme.font_size("font-body"), false))
        return

    var opponent_id := GameCardFace.optional_text(state.view.get("opponentPlayerId"))
    var mirror := GameStore.combat_is_current() and bool(GameStore.last_combat.get("mirror", false))
    if mirror and opponent_id != "":
        _identity_box.add_child(_opponent_block(_player(state.view, opponent_id), state.hp_cap, true))
        return
    if opponent_id != "":
        _identity_box.add_child(_opponent_block(_player(state.view, opponent_id), state.hp_cap, false))
        return
    _identity_box.add_child(UI.label("等待配对", UITheme.font_size("font-body"), false))


## 招募阶段的售货员：唯一一个非玩家身份，用固定中性沙色块。
func _seller_block() -> Control:
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-md"))

    var badge := PanelContainer.new()
    badge.name = "SellerBadge"
    var size := UITheme.metric("avatar-lg")
    badge.custom_minimum_size = Vector2(size, size)
    var tint := UITheme.tribe_color("通用")
    var box := StyleBoxFlat.new()
    box.bg_color = tint.darkened(0.55)
    box.border_color = tint
    box.set_border_width_all(2)
    box.set_corner_radius_all(int(size * 0.3))
    badge.add_theme_stylebox_override("panel", box)
    var mark := UI.label("售", int(size * 0.42), false)
    mark.name = "SellerMark"
    mark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    mark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    mark.add_theme_color_override("font_color", tint.lightened(0.5))
    badge.add_child(mark)
    row.add_child(badge)

    var lines := VBoxContainer.new()
    lines.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    lines.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    row.add_child(lines)
    lines.add_child(UI.section_title("商店"))
    lines.add_child(UI.label("商店营业中", UITheme.font_size("font-body"), false))
    return row


## 战斗阶段的对手身份；幽灵战只换角标并去掉血量条。
func _opponent_block(player: Dictionary, hp_cap: int, ghost: bool) -> Control:
    var block := VBoxContainer.new()
    block.name = "GhostIdentity" if ghost else "OpponentIdentity"
    block.add_theme_constant_override("separation", UITheme.space("space-xs"))

    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-sm"))
    block.add_child(row)
    row.add_child(UI.avatar(GameCardFace.optional_text(player.get("heroCardId")), UITheme.metric("avatar-lg")))

    var info := VBoxContainer.new()
    info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    info.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    row.add_child(info)

    var name_row := HBoxContainer.new()
    name_row.add_theme_constant_override("separation", UITheme.space("space-xs"))
    info.add_child(name_row)
    var display_name := GameCardFace.optional_text(player.get("displayName"))
    if display_name == "":
        display_name = GameCardFace.optional_text(player.get("playerId"))
    var name_label := UI.label(display_name, UITheme.font_size("font-body-sm"), false)
    name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    name_row.add_child(name_label)
    if ghost:
        var ghost_badge := UI.pill_label("幽灵", "info", "font-caption")
        ghost_badge.name = "GhostBadge"
        name_row.add_child(ghost_badge)
    var rank = player.get("rank")
    if rank != null:
        var rank_badge := UI.pill_label("第 %d 名" % int(rank), "accent", "font-caption")
        rank_badge.name = "RankBadge"
        name_row.add_child(rank_badge)

    var hero_id := GameCardFace.optional_text(player.get("heroCardId"))
    var hero_label := UI.label(
        CardDB.display_name(hero_id) if hero_id != "" else "未知英雄",
        UITheme.font_size("font-body-sm"),
        false,
    )
    hero_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    info.add_child(hero_label)

    var meta := UI.label("HP %d/%d · %s · 随从 %d" % [
        int(player.get("hp", 0)),
        maxi(hp_cap, 1),
        GameCardFace.era_text(int(player.get("era", 1))),
        int(player.get("boardCount", 0)),
    ], UITheme.font_size("font-caption"), false)
    meta.clip_text = true
    meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    info.add_child(meta)

    if not ghost:
        var bar := ProgressBar.new()
        bar.name = "SideHpBar"
        bar.show_percentage = false
        bar.max_value = maxi(hp_cap, 1)
        bar.value = clampf(float(player.get("hp", 0)), 0.0, bar.max_value)
        bar.custom_minimum_size = Vector2(0, UITheme.metric("seat-hp-bar-height"))
        block.add_child(bar)
    else:
        UI.apply_dim(block)
    return block


func _player(view: Dictionary, player_id: String) -> Dictionary:
    if player_id == "":
        return {}
    for player in view.get("players", []):
        if str(player.get("playerId", "")) == player_id:
            return player
    return {}


func _refresh_hero(state: GameUiState) -> void:
    UI.clear(_hero_detail)
    var self_state := state.self_state
    var hero_id := GameCardFace.optional_text(self_state.get("heroCardId"))
    _hero_detail.add_child(
        UI.hero_tile(hero_id, UITheme.metric("side-width") - UITheme.space("space-2xl"))
    )

    var stat_line := UI.label("生命 %d/%d · %s · 金币 %d" % [
        int(self_state.get("hp", 0)),
        maxi(state.hp_cap, 1),
        GameCardFace.era_text(int(self_state.get("era", 1))),
        int(self_state.get("gold", 0)),
    ], UITheme.font_size("font-body-sm"), false)
    stat_line.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    _hero_detail.add_child(stat_line)


func _refresh_actions(state: GameUiState) -> void:
    var aff: Dictionary = state.self_state.get("affordances", {})
    var blocked := state.stale or state.expired or state.target_pending

    var power: Dictionary = state.self_state.get("heroPower", {})
    _hero_power_button.text = "英雄技能（剩 %d）" % int(power.get("usesLeft", 0))
    _hero_power_button.disabled = blocked or not bool(aff.get("canUseHeroPower", false))
    _hero_power_button.tooltip_text = "" if aff.get("heroPowerDisabledReason") == null else UI.error_text(str(aff.get("heroPowerDisabledReason")))

    _end_button.text = "已结束招募" if state.recruited else "结束招募"
    _end_button.disabled = blocked or not bool(aff.get("canEndRecruit", false))
