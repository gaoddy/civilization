class_name FirstGameGuide
extends CanvasLayer
## 首局非阻塞引导：由顶部「提示」手动打开，压暗当前步骤目标，
## 底部提示条只覆盖自身，不拦截对局输入。
##
## 步骤表按「阶段」门控：阶段不匹配时切换到当前阶段对应的步骤，终局收起。

## 目标矩形外扩，避免压暗层贴边盖住目标。
const HIGHLIGHT_PADDING := 6.0

## 步骤表：id、阶段、目标节点名与文案键。
const STEPS := [
    {
        "id": 1,
        "phase": Protocol.PHASE_RECRUIT,
        "targets": ["ShopPanel", "ShopRow", "RecruitGoldTile"],
        "title": "ui.guide.step1.title",
        "body": "ui.guide.step1.body",
    },
    {
        "id": 2,
        "phase": Protocol.PHASE_RECRUIT,
        "targets": ["HandRow", "BoardRow"],
        "title": "ui.guide.step2.title",
        "body": "ui.guide.step2.body",
    },
    {
        "id": 3,
        "phase": Protocol.PHASE_RECRUIT,
        "targets": ["RecruitEndButton", "RecruitTimerLabel"],
        "title": "ui.guide.step3.title",
        "body": "ui.guide.step3.body",
    },
    {
        "id": 4,
        "phase": Protocol.PHASE_COMBAT,
        "targets": ["CenterPanel"],
        "title": "ui.guide.step4.title",
        "body": "ui.guide.step4.body",
    },
]

## 目标节点查找的根：由对局页传入自身；缺省时回退到当前场景。
var target_root: Node = null
## 当前步骤下标；-1 表示引导未打开。
var step := -1

var _phase := ""
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
        shade.color = UITheme.alpha("color-black", UITheme.opacity("scrim-alpha"))
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
        UITheme.stage_metric("card-height") + UITheme.space("space-md") + UITheme.space("space-xl"),
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
    # 宽度随内容自适应并水平居中，不铺满整行。
    _bar.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
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

    # 单行不换行：文案宽度直接决定提示条宽度，保持内容自适应而不是撑满整行。
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


## 顶部「提示」：从当前阶段对应的步骤开始，不读写本地记录。
func show_manual() -> void:
    _set_step(_step_for_phase(_phase))


## 当前阶段对应的步骤下标；没有匹配步骤时回退到第 1 步。
func _step_for_phase(phase: String) -> int:
    for index in STEPS.size():
        if str(STEPS[index]["phase"]) == phase:
            return index
    return 0


## 对局页每次刷新同步阶段，用于阶段门控。
func set_phase(phase: String, _self_view: Dictionary) -> void:
    _phase = phase
    _evaluate()


func _set_step(index: int) -> void:
    step = index
    _evaluate()


## 阶段不匹配时切换到当前阶段对应的步骤，终局收起；未打开时保持隐藏。
func _evaluate() -> void:
    if step < 0:
        _apply_visibility(false)
        return
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


## 完成/跳过：只收起引导，不写本地记录。
func _finish() -> void:
    step = -1
    _apply_visibility(false)


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
