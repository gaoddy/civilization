class_name GameSidePanel
extends VBoxContainer
## 右栏：场景身份与商店操作、回合与倒计时、自己的英雄与操作。
##
## 只发按钮信号，不发送意图；是否禁用由 `refresh(state)` 按页面门控决定。
## 身份块与英雄块等高，中部状态块因此落在右栏垂直中心。

signal hero_power_pressed
signal end_recruit_pressed
signal reroll_pressed
signal upgrade_pressed
signal freeze_pressed
signal gallery_pressed
## 首局引导手动重看。
signal hint_pressed
## 对局中主动退出。
signal leave_game_pressed

## 悬停详情层，由对局页注入；售货员大图悬停时登记。
var detail: GameDetailLayer = null

var _identity_panel: PanelContainer
var _identity_box: VBoxContainer
var _identity_content: VBoxContainer
var _shop_actions: HBoxContainer
var _reroll_button: Button
var _upgrade_button: Button
var _freeze_button: Button
var _hero_panel: PanelContainer
var _turn_label: Label
var _timer_label: Label
var _countdown_flip: Control
var _countdown_hovered := false
var _hero_detail: VBoxContainer
var _stats_row: HBoxContainer
var _hero_power_button: Button
var _end_button: Button
var _hint_button: Button
var _leave_button: Button

## 当前阶段截止时间；空串表示没有可展示的倒计时。
var _deadline := ""

## 最近一次刷新所属阶段；战斗阶段状态行改为「战斗中」。
var _phase := ""


func _init() -> void:
    name = "SidePanel"
    custom_minimum_size = Vector2(UITheme.metric("side-width"), 0)
    add_theme_constant_override("separation", UITheme.space("space-sm"))

    var identity_panel := UI.card_panel("surface")
    identity_panel.name = "SideIdentity"
    # 与英雄块等高：中部状态块正好落在右栏垂直中心。
    identity_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _identity_panel = identity_panel
    _identity_box = VBoxContainer.new()
    _identity_box.alignment = BoxContainer.ALIGNMENT_CENTER
    _identity_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    identity_panel.add_child(_identity_box)
    add_child(identity_panel)

    # 身份内容随阶段重建；商店操作按钮常驻在售货员下方。
    _identity_content = VBoxContainer.new()
    _identity_content.name = "SideIdentityContent"
    _identity_content.alignment = BoxContainer.ALIGNMENT_CENTER
    _identity_content.add_theme_constant_override("separation", UITheme.space("space-sm"))
    _identity_box.add_child(_identity_content)
    _identity_box.add_child(_build_shop_actions())

    var status_panel := UI.card_panel("surface")
    status_panel.name = "SideStatus"
    var status_box := HBoxContainer.new()
    status_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    status_panel.add_child(status_box)
    _turn_label = UI.label("", UITheme.font_size("font-title-lg"), false)
    _turn_label.name = "SideTurnLabel"
    _turn_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    status_box.add_child(_turn_label)
    status_box.add_child(_build_countdown_flip())
    var gallery_button := UI.styled_button(tr("ui.gallery.entry_match"), "ghost", 28)
    gallery_button.name = "GalleryButton"
    gallery_button.pressed.connect(func() -> void: gallery_pressed.emit())
    status_box.add_child(gallery_button)
    # 「提示」与「退出对局」对局中常驻，仅连接状态影响可用性。
    _hint_button = _status_action_button("HintButton", tr("ui.guide.hint"))
    _hint_button.pressed.connect(func() -> void: hint_pressed.emit())
    status_box.add_child(_hint_button)
    _leave_button = _status_action_button("LeaveGameButton", tr("ui.room.exit_game"))
    _leave_button.pressed.connect(func() -> void: leave_game_pressed.emit())
    status_box.add_child(_leave_button)
    add_child(status_panel)

    var hero_panel := UI.card_panel("surface")
    hero_panel.name = "SideHero"
    hero_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _hero_panel = hero_panel
    var hero_box := VBoxContainer.new()
    hero_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    hero_panel.add_child(hero_box)
    add_child(hero_panel)

    _hero_detail = VBoxContainer.new()
    _hero_detail.name = "HeroDetail"
    _hero_detail.add_theme_constant_override("separation", UITheme.space("space-sm"))
    hero_box.add_child(_hero_detail)

    var hero_spacer := Control.new()
    hero_spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
    hero_box.add_child(hero_spacer)

    _hero_power_button = UI.styled_button(tr("ui.game.side_panel.hero_power"), "default", 40)
    _hero_power_button.name = "HeroPowerButton"
    _hero_power_button.pressed.connect(func() -> void: hero_power_pressed.emit())
    hero_box.add_child(_hero_power_button)

    _stats_row = HBoxContainer.new()
    _stats_row.name = "HeroStats"
    _stats_row.add_theme_constant_override("separation", UITheme.space("space-xs"))
    hero_box.add_child(_stats_row)


## 售货员下方的商店操作：三个按钮等宽横排，费用写在按钮文案里。
func _build_shop_actions() -> HBoxContainer:
    _shop_actions = HBoxContainer.new()
    _shop_actions.name = "ShopActions"
    _shop_actions.add_theme_constant_override("separation", UITheme.space("space-xs"))
    _reroll_button = _shop_action_button("RerollButton", tr("ui.game.side_panel.reroll"))
    _reroll_button.pressed.connect(func() -> void: reroll_pressed.emit())
    _upgrade_button = _shop_action_button("UpgradeButton", tr("ui.game.side_panel.upgrade"))
    _upgrade_button.pressed.connect(func() -> void: upgrade_pressed.emit())
    _freeze_button = _shop_action_button("FreezeButton", tr("ui.game.side_panel.freeze"))
    _freeze_button.pressed.connect(func() -> void: freeze_pressed.emit())
    return _shop_actions


## 状态行动作键：比商店操作更紧凑，避免挤占倒计时与回合文案。
func _status_action_button(node_name: String, text: String) -> Button:
    var button := UI.styled_button(text, "micro", 26)
    button.name = node_name
    button.add_theme_font_size_override("font_size", UITheme.font_size("font-caption"))
    return button


func _shop_action_button(node_name: String, text: String) -> Button:
    var button := UI.styled_button(text, "default", 28)
    button.name = node_name
    button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    button.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    _shop_actions.add_child(button)
    return button


## 中部倒计时：鼠标移入时翻成「结束招募」，移出恢复剩余时间。
func _build_countdown_flip() -> Control:
    _countdown_flip = Control.new()
    _countdown_flip.name = "CountdownFlip"
    _countdown_flip.custom_minimum_size = Vector2(0, 40)
    _countdown_flip.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _countdown_flip.mouse_filter = Control.MOUSE_FILTER_STOP
    _countdown_flip.mouse_entered.connect(_on_countdown_entered)
    _countdown_flip.mouse_exited.connect(_on_countdown_exited)
    _countdown_flip.gui_input.connect(_on_countdown_input)

    _timer_label = UI.pill_label("—", "accent", "font-title")
    _timer_label.name = "TimerLabel"
    _timer_label.set_anchors_preset(Control.PRESET_FULL_RECT)
    _timer_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _timer_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _countdown_flip.add_child(_timer_label)

    _end_button = UI.styled_button(tr("ui.game.side_panel.end_recruit"), "primary", 40)
    _end_button.name = "EndButton"
    _end_button.set_anchors_preset(Control.PRESET_FULL_RECT)
    _end_button.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _end_button.visible = false
    _countdown_flip.add_child(_end_button)
    return _countdown_flip


func _process(_delta: float) -> void:
    _update_countdown()


func refresh(state: GameUiState) -> void:
    # `MSG_PHASE_CHANGED` 先更新 `GameStore.deadline`，此时 `view.phaseDeadline`
    # 仍是上一阶段的旧值；因此以 `GameStore.deadline` 为准，视图字段仅作回退。
    _phase = state.phase
    var view_deadline := str(state.view.get("phaseDeadline", ""))
    _deadline = GameStore.deadline if GameStore.deadline != "" else view_deadline
    _refresh_identity(state)
    _refresh_status(state)
    _refresh_hero(state)
    _refresh_actions(state)
    _equalize_blocks()
    _update_countdown()


## 身份块与英雄块取相同高度：中部状态块因此垂直居中。
func _equalize_blocks() -> void:
    var block := maxf(
        _identity_panel.get_combined_minimum_size().y,
        _hero_panel.get_combined_minimum_size().y
    )
    _identity_panel.custom_minimum_size = Vector2(0, block)
    _hero_panel.custom_minimum_size = Vector2(0, block)


func set_combat_mode(in_combat: bool) -> void:
    _hero_power_button.visible = not in_combat
    _shop_actions.visible = not in_combat
    if in_combat:
        _countdown_hovered = false
        _timer_label.visible = true
        _end_button.visible = false


## 当前阶段剩余秒数：已成功解析的截止时间返回非负秒数（归零后保持 0），
## 空串或不可解析时才返回 -1，页面据此区分「没有截止时间」与「已过期」。
func seconds_left() -> int:
    if _deadline == "":
        return -1
    var deadline := UI.unix_from_rfc3339(_deadline)
    if deadline <= 0.0:
        return -1
    return maxi(int(ceil(deadline - Time.get_unix_time_from_system())), 0)


func has_deadline() -> bool:
    return _deadline != "" and UI.unix_from_rfc3339(_deadline) > 0.0


func _update_countdown() -> void:
    var text := "—"
    if _phase == Protocol.PHASE_COMBAT:
        text = tr("ui.game.side_panel.combat")
    else:
        var left := seconds_left()
        if left > 0:
            text = tr("ui.game.side_panel.remaining_seconds") % left
        elif left == 0:
            text = tr("ui.game.side_panel.waiting_settle")
    if _timer_label.text != text:
        _timer_label.text = text
    _apply_countdown_flip()


func _apply_countdown_flip() -> void:
    var flipped := _countdown_hovered and _phase == Protocol.PHASE_RECRUIT
    _timer_label.visible = not flipped
    _end_button.visible = flipped


func _on_countdown_entered() -> void:
    _countdown_hovered = true
    _apply_countdown_flip()


func _on_countdown_exited() -> void:
    _countdown_hovered = false
    _apply_countdown_flip()


func _on_countdown_input(event: InputEvent) -> void:
    if not (_phase == Protocol.PHASE_RECRUIT and _countdown_hovered):
        return
    var pressed := false
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        pressed = event.pressed
    elif event is InputEventScreenTouch:
        pressed = event.pressed
    if pressed and not _end_button.disabled:
        end_recruit_pressed.emit()


func _refresh_status(state: GameUiState) -> void:
    _turn_label.text = tr("ui.game.side_panel.turn") % int(state.view.get("turn", GameStore.turn))


## 身份判定以阶段优先：招募阶段一律显示售货员（即使自己已淘汰），
## 战斗阶段才按「自身已淘汰 → 幽灵 → 对手 → 等待配对」（product.md 验收 4）。
func _refresh_identity(state: GameUiState) -> void:
    UI.clear(_identity_content)
    if state.phase == Protocol.PHASE_RECRUIT:
        _identity_content.add_child(_seller_block())
        return

    var self_player := _player(state.view, GameStore.player_id)
    if not self_player.is_empty() and not bool(self_player.get("alive", true)):
        _identity_content.add_child(
            UI.label(tr("ui.game.side_panel.eliminated"), UITheme.font_size("font-body"), false)
        )
        return

    var opponent_id := GameCardFace.optional_text(state.view.get("opponentPlayerId"))
    var mirror := GameStore.combat_is_current() and bool(GameStore.last_combat.get("mirror", false))
    if mirror and opponent_id != "":
        _identity_content.add_child(_opponent_block(_player(state.view, opponent_id), state.hp_cap, true))
        return
    if opponent_id != "":
        _identity_content.add_child(_opponent_block(_player(state.view, opponent_id), state.hp_cap, false))
        return
    _identity_content.add_child(
        UI.label(tr("ui.game.side_panel.waiting_match"), UITheme.font_size("font-body"), false)
    )


## 招募阶段的售货员：大图 + 底部居中名字，营业信息放在悬停提示里。
func _seller_block() -> Control:
    var tile := _portrait_tile(
        "SellerTile",
        tr("ui.game.side_panel.seller"),
        tr("ui.game.side_panel.seller_glyph"),
        UITheme.playstyle_color("neutral"),
    )
    if detail != null and is_instance_valid(detail):
        detail.track(tile, {"detail_kind": "seller"})
    return tile


## 无图片资源时的大图：部落色封面 + 大字水印 + 底部居中名字。
func _portrait_tile(
    node_name: String,
    display_name: String,
    glyph: String,
    tint: Color,
) -> PanelContainer:
    var width := UITheme.metric("portrait-size")
    var size := Vector2(width, UITheme.metric("portrait-height"))
    var tile := PanelContainer.new()
    tile.name = node_name
    tile.custom_minimum_size = size
    tile.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    tile.mouse_filter = Control.MOUSE_FILTER_STOP
    var box := StyleBoxFlat.new()
    box.bg_color = tint.darkened(0.55)
    box.border_color = tint
    box.set_border_width_all(2)
    box.set_corner_radius_all(int(width * 0.12))
    box.set_content_margin_all(0)
    tile.add_theme_stylebox_override("panel", box)

    var stack := Control.new()
    stack.custom_minimum_size = size
    stack.mouse_filter = Control.MOUSE_FILTER_IGNORE
    tile.add_child(stack)

    var mark := UI.label(glyph, int(size.y * 0.48), false)
    mark.name = "PortraitMark"
    mark.set_anchors_preset(Control.PRESET_FULL_RECT)
    mark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    mark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    mark.add_theme_color_override("font_color", tint.lightened(0.5))
    mark.modulate.a = 0.55
    mark.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_child(mark)

    var name_height := UITheme.font_size("font-title-lg") + UITheme.space("space-lg") * 2
    var strip := PanelContainer.new()
    strip.add_theme_stylebox_override("panel", UITheme.panel_style("cover-info"))
    strip.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
    strip.offset_top = -name_height
    strip.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_child(strip)

    var name_label := UI.label(display_name, UITheme.font_size("font-title-lg"), false)
    name_label.name = "PortraitName"
    name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    strip.add_child(name_label)
    return tile


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
        var ghost_badge := UI.pill_label(tr("ui.game.side_panel.ghost"), "info", "font-caption")
        ghost_badge.name = "GhostBadge"
        name_row.add_child(ghost_badge)
    var rank = player.get("rank")
    if rank != null:
        var rank_badge := UI.pill_label(
            tr("ui.game.side_panel.rank") % int(rank), "accent", "font-caption"
        )
        rank_badge.name = "RankBadge"
        name_row.add_child(rank_badge)

    var hero_id := GameCardFace.optional_text(player.get("heroCardId"))
    var hero_label := UI.label(
        CardDB.display_name(hero_id) if hero_id != "" else tr("ui.game.side_panel.unknown_hero"),
        UITheme.font_size("font-body-sm"),
        false,
    )
    hero_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    info.add_child(hero_label)

    var meta := UI.label(tr("ui.game.side_panel.hp_line") % [
        int(player.get("hp", 0)),
        maxi(hp_cap, 1),
        GameCardFace.era_text(int(player.get("era", 1))),
        int(player.get("boardCount", 0)),
    ], UITheme.font_size("font-caption"), false)
    meta.clip_text = true
    meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    info.add_child(meta)

    if not ghost:
        block.add_child(
            UI.hp_bar(
                int(player.get("hp", 0)),
                hp_cap,
                UITheme.metric("side-width") - UITheme.space("space-2xl"),
            )
        )
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
    var display_name := CardDB.display_name(hero_id) if hero_id != "" else tr("ui.common.no_hero")
    var entry := CardDB.card(hero_id)
    var playstyle := str(entry.get("playstyle", ""))
    var tint := (
        UITheme.playstyle_color(playstyle)
        if hero_id != ""
        else UITheme.color("color-border-strong")
    )
    var glyph := "?" if display_name.is_empty() else display_name.substr(0, 1)
    var portrait := _portrait_tile("HeroPortrait", display_name, glyph, tint)
    if hero_id != "" and detail != null and is_instance_valid(detail):
        detail.track(portrait, {"detail_kind": "hero", "heroId": hero_id})
    _hero_detail.add_child(portrait)

    UI.clear(_stats_row)
    _stats_row.add_child(
        _stat_tile(
            tr("ui.game.side_panel.hp"),
            "%d/%d" % [int(self_state.get("hp", 0)), maxi(state.hp_cap, 1)],
            "color-success",
        )
    )
    _stats_row.add_child(
        _stat_tile(
            tr("ui.game.side_panel.era"),
            CardDB.era_name(int(self_state.get("era", 1))),
            "color-info",
        )
    )
    _stats_row.add_child(
        _stat_tile(
            tr("ui.game.side_panel.gold"),
            str(int(self_state.get("gold", 0))),
            "color-accent",
            "GoldTile",
        )
    )


## 贴底的单项数据格：小标题 + 高对比大数值，三项等宽。
## `node_name` 用于稳定定位（首局引导步骤 1 的金币格）。
func _stat_tile(
    title: String,
    value: String,
    color_name: String,
    node_name := "",
) -> PanelContainer:
    var panel := UI.card_panel("raised-tight")
    if node_name != "":
        panel.name = node_name
    panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    panel.add_child(box)

    var caption := UI.label(title, UITheme.font_size("font-caption"), false)
    caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    caption.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    box.add_child(caption)

    var number := UI.label(value, UITheme.font_size("font-title-lg"), false)
    number.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    number.add_theme_color_override("font_color", UITheme.color(color_name))
    box.add_child(number)
    return panel


func _refresh_actions(state: GameUiState) -> void:
    var aff: Dictionary = state.self_state.get("affordances", {})
    var blocked := state.stale or state.expired or state.target_pending

    _reroll_button.text = tr("ui.game.side_panel.reroll_cost") % int(state.self_state.get("rerollCost", 0))
    _reroll_button.disabled = blocked or not bool(aff.get("canReroll", false))

    var upgrade_cost = state.self_state.get("upgradeCost")
    if upgrade_cost != null:
        _upgrade_button.text = tr("ui.game.side_panel.upgrade_cost") % int(upgrade_cost)
    else:
        _upgrade_button.text = tr("ui.game.side_panel.era_max")
    _upgrade_button.disabled = blocked or not bool(aff.get("canUpgradeEra", false))
    _upgrade_button.tooltip_text = "" if aff.get("upgradeDisabledReason") == null else UI.error_text(str(aff.get("upgradeDisabledReason")))

    _freeze_button.disabled = blocked or state.phase != Protocol.PHASE_RECRUIT or state.recruited

    var power: Dictionary = state.self_state.get("heroPower", {})
    _hero_power_button.text = tr("ui.game.side_panel.hero_power_uses") % int(power.get("usesLeft", 0))
    _hero_power_button.disabled = blocked or not bool(aff.get("canUseHeroPower", false))
    _hero_power_button.tooltip_text = "" if aff.get("heroPowerDisabledReason") == null else UI.error_text(str(aff.get("heroPowerDisabledReason")))

    _end_button.text = (
        tr("ui.game.side_panel.recruited") if state.recruited else tr("ui.game.side_panel.end_recruit")
    )
    _end_button.disabled = blocked or not bool(aff.get("canEndRecruit", false))

    # 提示与退出对局不受回合门控影响，只跟随连接状态。
    _hint_button.disabled = not state.connected
    _leave_button.disabled = not state.connected
