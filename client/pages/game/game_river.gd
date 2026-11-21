class_name GameRiver
extends PanelContainer
## 河道行：高 32，位于商店与棋盘之间、双方棋盘之间。
##
## 招募模式：中央回合数，右端倒计时（悬停翻成「结束招募」）。
## 战斗模式：中央回合数与结果，右端「战斗中」；不创建招募倒计时节点。

enum Mode { RECRUIT, COMBAT }

signal end_recruit_pressed

const COUNTDOWN_WIDTH := 120.0
const COUNTDOWN_HEIGHT := 28.0
const STATUS_PILL_WIDTH := 80.0

var mode := Mode.RECRUIT

var _turn_text := ""
var _detail := ""
var _deadline := ""
var _phase := ""
var _recruited := false
var _hovered := false

var _turn_label: Label = null
var _countdown_flip: Control = null
var _timer_label: Label = null
var _end_button: Button = null
var _banner: Label = null
var _status_pill: Label = null


func _init(river_mode := Mode.RECRUIT) -> void:
    mode = river_mode
    name = "RecruitRiver" if mode == Mode.RECRUIT else "CombatRiver"
    custom_minimum_size = Vector2(0, UITheme.stage_metric("river-height"))
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_theme_stylebox_override("panel", _river_style())
    var layout := Control.new()
    layout.name = "RiverLayout"
    layout.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(layout)
    if mode == Mode.RECRUIT:
        _build_recruit(layout)
    else:
        _build_combat(layout)


func _build_recruit(layout: Control) -> void:
    _turn_label = UI.label("", UITheme.font_size("font-body"), false)
    _turn_label.name = "RecruitTurnLabel"
    _turn_label.set_anchors_preset(Control.PRESET_FULL_RECT)
    _turn_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _turn_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    _turn_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    _turn_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    layout.add_child(_turn_label)

    # iPhone 基准下河道变矮，倒计时块按同一比例收窄，避免越出河道。
    var phone := ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
    var countdown_size := Vector2(96.0, 22.0) if phone else Vector2(COUNTDOWN_WIDTH, COUNTDOWN_HEIGHT)
    _countdown_flip = Control.new()
    _countdown_flip.name = "RecruitCountdownFlip"
    _countdown_flip.custom_minimum_size = countdown_size
    _countdown_flip.set_anchors_preset(Control.PRESET_CENTER_RIGHT)
    _countdown_flip.offset_left = -countdown_size.x
    _countdown_flip.offset_right = 0
    _countdown_flip.offset_top = -countdown_size.y * 0.5
    _countdown_flip.offset_bottom = countdown_size.y * 0.5
    _countdown_flip.mouse_filter = Control.MOUSE_FILTER_STOP
    _countdown_flip.mouse_entered.connect(_on_countdown_entered)
    _countdown_flip.mouse_exited.connect(_on_countdown_exited)
    _countdown_flip.gui_input.connect(_on_countdown_input)
    layout.add_child(_countdown_flip)

    _timer_label = UI.pill_label("—", "accent", "font-caption")
    _timer_label.name = "RecruitTimerLabel"
    _timer_label.set_anchors_preset(Control.PRESET_FULL_RECT)
    _timer_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _timer_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _countdown_flip.add_child(_timer_label)

    _end_button = UI.styled_button(tr("ui.game.side_panel.end_recruit"), "primary", 28)
    _end_button.name = "RecruitEndButton"
    _end_button.set_anchors_preset(Control.PRESET_FULL_RECT)
    _end_button.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _end_button.visible = false
    _countdown_flip.add_child(_end_button)


func _build_combat(layout: Control) -> void:
    _banner = UI.label("", UITheme.font_size("font-display"), false)
    _banner.name = "CombatBanner"
    _banner.set_anchors_preset(Control.PRESET_FULL_RECT)
    _banner.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _banner.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    _banner.add_theme_color_override("font_color", UITheme.color("color-accent"))
    _banner.mouse_filter = Control.MOUSE_FILTER_IGNORE
    layout.add_child(_banner)

    _status_pill = UI.pill_label(tr("ui.game.side_panel.combat"), "info", "font-caption")
    _status_pill.name = "CombatStatusPill"
    _status_pill.set_anchors_preset(Control.PRESET_CENTER_RIGHT)
    _status_pill.offset_left = -STATUS_PILL_WIDTH
    _status_pill.offset_right = 0
    var pill_height := 22.0 if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE else COUNTDOWN_HEIGHT
    _status_pill.offset_top = -pill_height * 0.5
    _status_pill.offset_bottom = pill_height * 0.5
    _status_pill.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _status_pill.mouse_filter = Control.MOUSE_FILTER_IGNORE
    layout.add_child(_status_pill)


func _process(_delta: float) -> void:
    if mode == Mode.RECRUIT:
        _update_countdown()


func refresh(state: GameUiState) -> void:
    # `MSG_PHASE_CHANGED` 先更新 `GameStore.deadline`，此时 `view.phaseDeadline`
    # 仍是上一阶段的旧值；因此以 `GameStore.deadline` 为准，视图字段仅作回退。
    _phase = state.phase
    var view_deadline := str(state.view.get("phaseDeadline", ""))
    _deadline = GameStore.deadline if GameStore.deadline != "" else view_deadline
    _recruited = state.recruited
    _turn_text = tr("ui.game.side_panel.turn") % int(state.view.get("turn", GameStore.turn))
    if mode == Mode.RECRUIT:
        _refresh_recruit(state)
    else:
        _refresh_combat()


func _refresh_recruit(state: GameUiState) -> void:
    _turn_label.text = _turn_text
    _end_button.text = (
        tr("ui.game.side_panel.recruited") if state.recruited else tr("ui.game.side_panel.end_recruit")
    )
    var aff: Dictionary = state.self_state.get("affordances", {})
    _end_button.disabled = (
        state.recruited
        or state.stale
        or state.expired
        or state.target_pending
        or not bool(aff.get("canEndRecruit", false))
    )
    _update_countdown()


func _refresh_combat() -> void:
    _banner.text = _turn_text if _detail == "" else "%s · %s" % [_turn_text, _detail]
    if _detail == "":
        _banner.add_theme_color_override("font_color", UITheme.color("color-accent"))


## 战斗结果追加到回合数之后；`detail` 为空串时回到只显示回合数。
func set_combat_detail(detail: String, font_color: Color) -> void:
    if mode != Mode.COMBAT:
        return
    _detail = detail
    _refresh_combat()
    if detail != "":
        _banner.add_theme_color_override("font_color", font_color)


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
    if _timer_label == null:
        return
    var text := "—"
    if _recruited:
        text = tr("ui.game.side_panel.recruited")
    else:
        var left := seconds_left()
        if left > 0:
            text = tr("ui.game.side_panel.remaining_seconds") % left
        elif left == 0:
            text = tr("ui.game.side_panel.waiting_settle")
    if _timer_label.text != text:
        _timer_label.text = text
    _apply_flip()


## 已结束招募时不翻转，悬停只保留剩余时间。
func _apply_flip() -> void:
    var flipped := _hovered and not _recruited and _phase == Protocol.PHASE_RECRUIT
    _timer_label.visible = not flipped
    _end_button.visible = flipped


func _on_countdown_entered() -> void:
    _hovered = true
    _apply_flip()


func _on_countdown_exited() -> void:
    _hovered = false
    _apply_flip()


func _on_countdown_input(event: InputEvent) -> void:
    if not (_phase == Protocol.PHASE_RECRUIT and _hovered and not _recruited):
        return
    var pressed := false
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        pressed = event.pressed
    elif event is InputEventScreenTouch:
        pressed = event.pressed
    if pressed and not _end_button.disabled:
        end_recruit_pressed.emit()


func _river_style() -> StyleBoxFlat:
    var box := StyleBoxFlat.new()
    box.bg_color = UITheme.alpha("color-surface-sunken", 0.35)
    box.border_color = UITheme.alpha("color-border", 0.6)
    box.border_width_top = 1
    box.border_width_bottom = 1
    box.set_content_margin_all(0)
    return box
