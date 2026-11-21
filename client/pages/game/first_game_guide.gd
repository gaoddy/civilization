class_name FirstGameGuide
extends CanvasLayer
## 首局非阻塞引导：压暗当前步骤目标，底部提示条只覆盖自身，不拦截对局输入。
##
## 步骤表按「阶段 + 展示条件」门控；步骤 2/3 的条件不满足时自动跳到下一步。
## 当前步骤写回 `GameStore.guide_step`，供断线重连后的场景重建恢复；
## 首次展示记录在 `user://first_game_guide.cfg` 的 `seen/<playerId>`。

const CONFIG_PATH := "user://first_game_guide.cfg"
const SEEN_SECTION := "seen"
## 目标矩形外扩，避免压暗层贴边盖住目标。
const HIGHLIGHT_PADDING := 6.0
const SHADE_COLOR := Color(0, 0, 0, 0.55)

## 步骤表：id、阶段、目标节点名、文案键与展示条件。
const STEPS := [
    {
        "id": 1,
        "phase": Protocol.PHASE_RECRUIT,
        "targets": ["ShopPanel", "ShopRow", "GoldTile"],
        "title": "ui.guide.step1.title",
        "body": "ui.guide.step1.body",
        "condition": "",
    },
    {
        "id": 2,
        "phase": Protocol.PHASE_RECRUIT,
        "targets": ["HandRow", "BoardRow"],
        "title": "ui.guide.step2.title",
        "body": "ui.guide.step2.body",
        "condition": "hand",
    },
    {
        "id": 3,
        "phase": Protocol.PHASE_RECRUIT,
        "targets": ["RecruitEndButton", "RecruitTimerLabel"],
        "title": "ui.guide.step3.title",
        "body": "ui.guide.step3.body",
        "condition": "board",
    },
    {
        "id": 4,
        "phase": Protocol.PHASE_COMBAT,
        "targets": ["CenterPanel", "ReportLayer"],
        "title": "ui.guide.step4.title",
        "body": "ui.guide.step4.body",
        "condition": "",
    },
]

## 目标节点查找的根：由对局页传入自身；缺省时回退到当前场景。
var target_root: Node = null
## 当前步骤下标；-1 表示未在引导中。重连续接时从这里恢复。
var step := -1
## 手动重看：不读写 `seen`，关闭后只清会话步骤。
var manual := false

var _phase := ""
var _self_view: Dictionary = {}
var _root: Control
var _bars: Array = []
var _bar: PanelContainer
var _title: Label
var _body: Label
var _step_label: Label
var _next: Button
var _skip: Button


func _init() -> void:
    name = "FirstGameGuide"
    layer = 100
    _build()


func _build() -> void:
    _root = Control.new()
    _root.name = "GuideOverlay"
    _root.set_anchors_preset(Control.PRESET_FULL_RECT)
    _root.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(_root)

    # 四块压暗矩形围出高亮窗口；全部 IGNORE，点击穿透到底层对局控件。
    for index in 4:
        var shade := ColorRect.new()
        shade.name = "GuideShade%d" % index
        shade.color = SHADE_COLOR
        shade.mouse_filter = Control.MOUSE_FILTER_IGNORE
        shade.visible = false
        _root.add_child(shade)
        _bars.append(shade)

    var margin := MarginContainer.new()
    margin.name = "GuideBarAnchor"
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
    for side in ["left", "right"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-xl"))
    margin.add_theme_constant_override("margin_top", UITheme.space("space-sm"))
    # 手牌贴底后占据页面最下 146：提示条上移并留出悬停放大的余量。
    margin.add_theme_constant_override(
        "margin_bottom",
        UITheme.metric("card-height") + UITheme.space("space-md") + UITheme.space("space-xl"),
    )
    _root.add_child(margin)

    var column := VBoxContainer.new()
    column.mouse_filter = Control.MOUSE_FILTER_IGNORE
    margin.add_child(column)
    var spacer := Control.new()
    spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
    spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    column.add_child(spacer)

    _bar = UIControls.card_panel("raised")
    _bar.name = "GuideBar"
    # 只有提示条自身接收点击，外层锚点与压暗层都放行。
    _bar.mouse_filter = Control.MOUSE_FILTER_STOP
    column.add_child(_bar)

    var inner := VBoxContainer.new()
    inner.add_theme_constant_override("separation", UITheme.space("space-sm"))
    _bar.add_child(inner)

    var header := HBoxContainer.new()
    header.add_theme_constant_override("separation", UITheme.space("space-sm"))
    inner.add_child(header)
    _title = UIControls.label("", UITheme.font_size("font-title"), false)
    _title.name = "GuideTitle"
    _title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    header.add_child(_title)
    _step_label = UIControls.pill_label("", "accent", "font-caption")
    _step_label.name = "GuideStep"
    header.add_child(_step_label)

    _body = UIControls.label("", UITheme.font_size("font-body"), false)
    _body.name = "GuideBody"
    inner.add_child(_body)

    var actions := HBoxContainer.new()
    actions.alignment = BoxContainer.ALIGNMENT_END
    actions.add_theme_constant_override("separation", UITheme.space("space-sm"))
    inner.add_child(actions)
    _skip = UIControls.styled_button("", "micro", 30)
    _skip.name = "GuideSkip"
    _skip.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    _skip.pressed.connect(_on_skip_pressed)
    actions.add_child(_skip)
    _next = UIControls.styled_button("", "primary", 30)
    _next.name = "GuideNext"
    _next.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    _next.pressed.connect(_on_next_pressed)
    actions.add_child(_next)

    visible = false


func _process(_delta: float) -> void:
    if visible:
        _sync_shade()


## 自动展示：优先恢复会话步骤；否则按本地 `seen` 决定是否从第 1 步开始。
func start() -> void:
    manual = false
    if GameStore.guide_step >= 0:
        _set_step(clampi(GameStore.guide_step, 0, STEPS.size() - 1))
        return
    if GameStore.player_id == "" or _has_seen():
        _set_step(-1, false)
        return
    _set_step(0)


## 右栏「提示」：手动重看不读写 `seen`，也不受阶段门控——从当前阶段对应的
## 步骤开始，非招募/战斗阶段回退到第 1 步。
func show_manual() -> void:
    manual = true
    _set_step(_step_for_phase(_phase), false)


## 当前阶段对应的步骤下标；没有匹配步骤时回退到第 1 步。
func _step_for_phase(phase: String) -> int:
    for index in STEPS.size():
        if str(STEPS[index]["phase"]) == phase:
            return index
    return 0


## 对局页每次刷新同步阶段与本人视图，用于阶段门控与步骤 2/3 的条件跳过。
func set_phase(phase: String, self_view: Dictionary) -> void:
    _phase = phase
    _self_view = self_view
    _evaluate()


func _set_step(index: int, write_store := true) -> void:
    step = index
    if write_store:
        GameStore.guide_step = index
    _evaluate()


## 阶段不匹配时隐藏但保留步骤；条件不满足时跳过到下一步。
func _evaluate() -> void:
    if step < 0:
        _apply_visibility(false)
        return
    if manual:
        # 手动重看不按阶段隐藏：跟随刷新换到当前阶段对应的步骤，终局才收起。
        if step >= STEPS.size():
            step = 0
        if _phase == Protocol.PHASE_GAME_OVER:
            _apply_visibility(false)
            return
        if _phase != "" and str(STEPS[step]["phase"]) != _phase:
            step = _step_for_phase(_phase)
        _update_texts()
        _apply_visibility(true)
        _sync_shade()
        return
    while step < STEPS.size():
        var entry: Dictionary = STEPS[step]
        # 阶段未知（视图尚未到达）与阶段不匹配都隐藏，等下一次刷新再恢复。
        if _phase == "" or str(entry["phase"]) != _phase:
            _apply_visibility(false)
            return
        if not _condition_met(str(entry.get("condition", ""))):
            step += 1
            GameStore.guide_step = step
            continue
        break
    if step >= STEPS.size():
        _apply_visibility(false)
        return
    _update_texts()
    _apply_visibility(true)
    _sync_shade()


func _condition_met(condition: String) -> bool:
    match condition:
        "hand":
            var hand = _self_view.get("hand", [])
            return hand is Array and not hand.is_empty()
        "board":
            var board = _self_view.get("board", [])
            return board is Array and not board.is_empty()
        _:
            return true


func _update_texts() -> void:
    if step < 0 or step >= STEPS.size():
        return
    var entry: Dictionary = STEPS[step]
    _title.text = tr(str(entry["title"]))
    _body.text = tr(str(entry["body"]))
    _step_label.text = "%d/%d" % [step + 1, STEPS.size()]
    _next.text = tr("ui.guide.done") if step == STEPS.size() - 1 else tr("ui.guide.next")
    _skip.text = tr("ui.guide.skip")


func _apply_visibility(value: bool) -> void:
    visible = value
    if not value:
        _clear_shade()


func _on_next_pressed() -> void:
    if step < 0:
        return
    if step >= STEPS.size() - 1:
        _finish()
        return
    _set_step(step + 1)


func _on_skip_pressed() -> void:
    _finish()


## 完成/跳过：自动展示写 `seen`，手动重看不写；两种情况都清会话步骤。
func _finish() -> void:
    if not manual:
        _mark_seen()
    manual = false
    step = -1
    GameStore.guide_step = -1
    _apply_visibility(false)


func _has_seen() -> bool:
    var config := ConfigFile.new()
    var err := config.load(CONFIG_PATH)
    if err != OK:
        # 文件不存在或损坏都按「未看过」处理，不阻塞对局。
        return false
    return bool(config.get_value(SEEN_SECTION, GameStore.player_id, false))


func _mark_seen() -> void:
    if GameStore.player_id == "":
        return
    var config := ConfigFile.new()
    var loaded := config.load(CONFIG_PATH)
    if loaded != OK and loaded != ERR_FILE_NOT_FOUND:
        push_warning("首局引导读取失败：%s" % error_string(loaded))
    config.set_value(SEEN_SECTION, GameStore.player_id, true)
    var saved := config.save(CONFIG_PATH)
    if saved != OK:
        # 写入失败只影响下次是否再次展示，不影响本局。
        push_warning("首局引导写入失败：%s" % error_string(saved))


func _sync_shade() -> void:
    if step < 0 or step >= STEPS.size():
        _clear_shade()
        return
    var rect := _target_rect(Array(STEPS[step]["targets"]))
    if rect.size.x <= 0.0 or rect.size.y <= 0.0:
        _clear_shade()
        return
    var screen := _root.get_viewport_rect().size
    rect = rect.grow(HIGHLIGHT_PADDING).intersection(Rect2(Vector2.ZERO, screen))
    _set_bar(0, Rect2(0.0, 0.0, screen.x, maxf(rect.position.y, 0.0)))
    _set_bar(1, Rect2(0.0, rect.end.y, screen.x, maxf(screen.y - rect.end.y, 0.0)))
    _set_bar(2, Rect2(0.0, rect.position.y, maxf(rect.position.x, 0.0), rect.size.y))
    _set_bar(3, Rect2(rect.end.x, rect.position.y, maxf(screen.x - rect.end.x, 0.0), rect.size.y))


func _set_bar(index: int, rect: Rect2) -> void:
    var shade: ColorRect = _bars[index]
    shade.visible = rect.size.x > 0.0 and rect.size.y > 0.0
    if not shade.visible:
        return
    shade.position = rect.position
    shade.size = rect.size


func _clear_shade() -> void:
    for shade in _bars:
        shade.visible = false


## 多个目标取并集；缺失或不可见的目标直接跳过（只影响高亮，不影响文案）。
func _target_rect(names: Array) -> Rect2:
    var found := Rect2()
    var has_target := false
    for node_name in names:
        var control := _find_control(str(node_name))
        if control == null:
            continue
        var rect := control.get_global_rect()
        found = rect if not has_target else found.merge(rect)
        has_target = true
    return found


func _find_control(node_name: String) -> Control:
    var root := target_root
    if root == null or not is_instance_valid(root):
        root = get_tree().current_scene if is_inside_tree() else null
    if root == null:
        return null
    var found := root.find_child(node_name, true, false)
    if found is Control and found.is_visible_in_tree():
        return found
    return null
