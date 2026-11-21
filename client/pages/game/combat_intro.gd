class_name CombatIntro
extends PopupLayer
## 战斗开场覆盖层：双方头像、名字与 VS 标记，约 1.2 秒后淡出。
##
## 与 `CombatFx` 一样按帧驱动，不用 `Tween`，便于无头冒烟按帧断言；只做视觉，
## 显示期间由 `GameCombatView` 暂停演示推进。覆盖层忽略鼠标，不抢详情与输入；
## 遮罩与居中由 `PopupLayer` 承担。
##
## 身份取法与 `GameOpponentView` 一致：`heroCardId` 决定名字、底色与头像；
## 幽灵战对手块压暗并追加「幽灵」角标；对手缺失时回退首字色块与「未知英雄」。

## 开场总时长。
const INTRO_MS := 1200.0

## 入场缩放淡入时长。
const ENTER_MS := 160.0

## 出场淡出时长（开场末尾）。
const EXIT_MS := 160.0

## 入场起始缩放。
const ENTER_SCALE := 0.94

## 出场结束时的轻微放大。
const EXIT_SCALE := 0.04

var _elapsed_ms := 0.0
var _shown := false
var _card: PanelContainer
var _column: VBoxContainer
var _own_block: VBoxContainer
var _opponent_block: VBoxContainer


func _init() -> void:
    super(PopupLayer.Placement.CENTER, 0.6, UITheme.color("color-bg"))
    name = "CombatIntro"
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    scrim.mouse_filter = Control.MOUSE_FILTER_IGNORE

    # 独立底板压住河道回合数与随从卡，避免 VS 列与底层文字互相重叠。
    _card = PanelContainer.new()
    _card.name = "CombatIntroCard"
    _card.add_theme_stylebox_override("panel", UITheme.panel_style("raised"))
    _card.mouse_filter = Control.MOUSE_FILTER_IGNORE
    set_content(_card)

    _column = VBoxContainer.new()
    _column.name = "CombatIntroColumn"
    _column.alignment = BoxContainer.ALIGNMENT_CENTER
    _column.add_theme_constant_override("separation", UITheme.space("space-md"))
    _column.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _card.add_child(_column)

    var row := HBoxContainer.new()
    row.name = "CombatIntroRow"
    row.alignment = BoxContainer.ALIGNMENT_CENTER
    row.add_theme_constant_override("separation", UITheme.space("space-xl"))
    row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _column.add_child(row)

    _own_block = _build_slot(row, "CombatIntroOwn")
    row.add_child(_vs_mark())
    _opponent_block = _build_slot(row, "CombatIntroOpponent")

    var caption := UIControls.label(tr("ui.game.combat.intro"), UITheme.font_size("font-body"), false)
    caption.name = "CombatIntroCaption"
    caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    caption.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    caption.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _column.add_child(caption)


## 用当前快照摆好双方身份并从头开始入场；重复调用会重新计时。
func present(state: GameUiState) -> void:
    _elapsed_ms = 0.0
    _shown = true
    var opponent_id := CardText.optional_text(state.view.get("opponentPlayerId"))
    var ghost := (
        GameStore.combat_is_current()
        and bool(GameStore.last_combat.get("mirror", false))
    )
    _fill_slot(_own_block, _player(state, GameStore.player_id), false)
    _fill_slot(_opponent_block, _player(state, opponent_id), ghost)
    visible = true
    modulate.a = 0.0
    _apply_progress()


## 推进开场；返回真表示本帧仍由开场占用，调用方不得推进战斗演示。
##
## 结束的那一帧同样返回真：首条动作从下一帧开始计数。
func advance(delta_ms: float) -> bool:
    if not _shown:
        return false
    _elapsed_ms += delta_ms
    if _elapsed_ms >= INTRO_MS:
        clear()
        return true
    _apply_progress()
    return true


## 收起开场并复位动画状态；进入非战斗阶段或下一场战斗时调用。
func clear() -> void:
    _shown = false
    _elapsed_ms = 0.0
    visible = false
    modulate.a = 1.0
    if _card != null:
        _card.scale = Vector2.ONE


## 入场缩放淡入、停留、末尾淡出放大。
func _apply_progress() -> void:
    var enter := clampf(_elapsed_ms / ENTER_MS, 0.0, 1.0)
    var exit_k := clampf(
        (_elapsed_ms - (INTRO_MS - EXIT_MS)) / EXIT_MS,
        0.0,
        1.0,
    )
    var scale := lerpf(ENTER_SCALE, 1.0, enter) + EXIT_SCALE * exit_k
    _card.pivot_offset = _card.size * 0.5
    _card.scale = Vector2(scale, scale)
    modulate.a = enter * (1.0 - exit_k)


## 一侧的纵向槽：头像、名字、英雄名，幽灵战追加角标并压暗。
func _fill_slot(block: VBoxContainer, player: Dictionary, ghost: bool) -> void:
    UIControls.clear(block)
    block.modulate = Color(1, 1, 1, 1)
    var hero_id := CardText.optional_text(player.get("heroCardId"))
    var hero_name := (
        CardDB.display_name(hero_id)
        if hero_id != ""
        else tr("ui.game.side_panel.unknown_hero")
    )
    var display := CardText.optional_text(player.get("displayName"))
    if display == "":
        display = CardText.optional_text(player.get("playerId"))
    if display == "":
        display = hero_name
    var tint := (
        UITheme.playstyle_color(CardDB.primary_playstyle(CardDB.hero(hero_id)))
        if hero_id != ""
        else UITheme.color("color-border-strong")
    )

    var portrait := GamePortrait.new()
    portrait.name = "CombatIntroPortrait"
    var art_path := CardDB.art_path(hero_id, "avatar") if hero_id != "" else ""
    var glyph := display.substr(0, 1) if display != "" else "?"
    portrait.setup(hero_name, glyph, tint, false, art_path)
    block.add_child(portrait)

    var name_text := display + (tr("ui.game.combat.ghost_suffix") if ghost else "")
    var name_label := UIControls.label(name_text, UITheme.font_size("font-title"), false)
    name_label.name = "CombatIntroName"
    name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    name_label.add_theme_color_override("font_color", UITheme.color("color-text"))
    block.add_child(name_label)

    var hero_label := UIControls.label(hero_name, UITheme.font_size("font-caption"), false)
    hero_label.name = "CombatIntroHero"
    hero_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    hero_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    block.add_child(hero_label)

    if ghost:
        var badge := UIControls.pill_label(tr("ui.game.side_panel.ghost"), "info", "font-caption")
        badge.name = "GhostBadge"
        badge.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
        badge.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
        block.add_child(badge)
        UIControls.apply_dim(block)
    _ignore_mouse(block)


func _build_slot(row: HBoxContainer, node_name: String) -> VBoxContainer:
    var block := VBoxContainer.new()
    block.name = node_name
    block.alignment = BoxContainer.ALIGNMENT_CENTER
    block.add_theme_constant_override("separation", UITheme.space("space-xs"))
    block.mouse_filter = Control.MOUSE_FILTER_IGNORE
    row.add_child(block)
    return block


## 中央 VS 标记：两侧金色短线，中间强调色大字。
func _vs_mark() -> HBoxContainer:
    var mark := HBoxContainer.new()
    mark.name = "CombatIntroVs"
    mark.alignment = BoxContainer.ALIGNMENT_CENTER
    mark.add_theme_constant_override("separation", UITheme.space("space-sm"))
    mark.mouse_filter = Control.MOUSE_FILTER_IGNORE
    for side in 2:
        var line := ColorRect.new()
        line.name = "CombatIntroVsLine"
        line.color = UITheme.alpha("color-accent", 0.6)
        line.custom_minimum_size = Vector2(UITheme.space("space-2xl"), UITheme.space("space-2xs"))
        line.size_flags_vertical = Control.SIZE_SHRINK_CENTER
        line.mouse_filter = Control.MOUSE_FILTER_IGNORE
        mark.add_child(line)
        if side == 0:
            var label := UIControls.label(tr("ui.game.combat.vs"), UITheme.font_size("font-hero"), false)
            label.name = "CombatIntroVsLabel"
            label.add_theme_color_override("font_color", UITheme.color("color-accent"))
            label.mouse_filter = Control.MOUSE_FILTER_IGNORE
            mark.add_child(label)
    return mark


## 在视图玩家列表里按 id 取条目；缺失返回空字典（渲染回退身份）。
func _player(state: GameUiState, player_id: String) -> Dictionary:
    if player_id == "":
        return {}
    for player in state.view.get("players", []):
        if player is Dictionary and str(player.get("playerId", "")) == player_id:
            return player
    return {}


## 整个覆盖层不接收鼠标，避免 child 的默认 STOP 抢走悬停与点击。
func _ignore_mouse(node: Node) -> void:
    if node is Control:
        (node as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
    for child in node.get_children():
        _ignore_mouse(child)
