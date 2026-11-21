class_name GameChoiceOverlay
extends PopupLayer
## 对局页的选择浮层：发现待选显示一排候选卡面，进化待选显示一排选项块。
##
## 只负责呈现与点击；待选是否可操作由对局页的门槛判断控制，本组件不构造意图。
## 同一份待选重复下发时不重建节点，避免每次视图刷新丢失悬停状态；遮罩、居中
## 与显隐由 `PopupLayer` 承担。

## 点击第 `index` 个卡面或选项块；下标与该待选的候选 / 选项顺序一致。
signal chosen(index: int)

## 悬停与按下时的强调描边宽度。
const ACTIVE_BORDER_WIDTH := 2
## 卡面与按钮描边之间让出的距离，与 `raised-flush` 的内容内边距一致。
const FACE_INSET := 1
## 待选类型到提示文案的映射；未知类型不显示提示。
const HINT_KEYS := {
    "discover": "ui.game.overlay.discover_hint",
    "evolve": "ui.game.overlay.evolve_hint",
}

## 当前展示的待选签名；用于跳过重复构建。
var _signature := ""


func _init() -> void:
    super(
        PopupLayer.Placement.CENTER,
        UITheme.opacity("scrim-alpha-choice"),
        UITheme.color("color-bg"),
    )
    name = "ChooserOverlay"


## 展示一份待选；空候选项或目标待选不显示。
func show_choice(choice: Dictionary) -> void:
    var kind := str(choice.get("kind", ""))
    var items: Array = (
        choice.get("candidates", []) if kind == "discover" else choice.get("options", [])
    )
    var signature := JSON.stringify([kind, items])
    if visible and signature == _signature:
        return
    _clear()
    _signature = signature
    if items.is_empty() or not HINT_KEYS.has(kind):
        visible = false
        return
    visible = true
    var box := VBoxContainer.new()
    box.name = "ChoiceBox"
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    set_content(box)
    var hint := UIControls.label(tr(str(HINT_KEYS[kind])), UITheme.font_size("font-body"))
    hint.name = "ChoiceHint"
    hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    hint.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_child(hint)
    var row := HBoxContainer.new()
    row.name = "ChoiceRow"
    row.alignment = BoxContainer.ALIGNMENT_CENTER
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    box.add_child(row)
    var index := 0
    for item in items:
        row.add_child(_build_choice(kind, str(item), index))
        index += 1


## 隐藏并释放内容；签名一并清空，下次同内容仍会重建。
func hide_choice() -> void:
    visible = false
    _signature = ""
    _clear()


func _clear() -> void:
    clear_content()


## 单个候选项：按钮承接点击，内部放卡面或选项文字。
func _build_choice(kind: String, item: String, index: int) -> Button:
    var button := UIControls.button("")
    button.name = "Choice%d" % index
    button.custom_minimum_size = Vector2(
        UITheme.stage_metric("card-width"), UITheme.stage_metric("card-height")
    )
    button.focus_mode = Control.FOCUS_NONE
    button.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        button.add_theme_stylebox_override(state, _choice_style(state))
    button.pressed.connect(_on_choice_pressed.bind(index))
    if kind == "discover":
        _add_card_face(button, item)
        button.tooltip_text = CardDB.display_text(item)
    else:
        _add_option_label(button, item)
        button.tooltip_text = UIWording.enum_text("evolve", item)
    return button


## 发现候选：与手牌同源的卡面，身材取卡表基础值（服务端只下发 cardId）。
func _add_card_face(button: Button, card_id: String) -> void:
    var entry := CardDB.card(card_id)
    var face := HandCardFace.build({
        "cardId": card_id,
        "attack": entry.get("attack"),
        "health": entry.get("health"),
    }, self)
    face.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    face.offset_left = FACE_INSET
    face.offset_top = FACE_INSET
    face.offset_right = -FACE_INSET
    face.offset_bottom = -FACE_INSET
    face.mouse_filter = Control.MOUSE_FILTER_IGNORE
    button.add_child(face)


## 进化选项：不是卡牌，只用同尺寸底板与居中文字。
func _add_option_label(button: Button, option: String) -> void:
    var label := UIControls.label(UIWording.enum_text("evolve", option), UITheme.font_size("font-body"), true)
    label.name = "ChoiceLabel"
    label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    label.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    label.offset_left = UITheme.space("space-sm")
    label.offset_right = -UITheme.space("space-sm")
    label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    button.add_child(label)


func _on_choice_pressed(index: int) -> void:
    chosen.emit(index)


## 按钮各状态样式：常态沿用卡面底板，悬停与按下改为强调描边。
static func _choice_style(state: String) -> StyleBoxFlat:
    var style := UITheme.panel_style("raised-flush")
    if state in ["hover", "pressed"]:
        style.border_color = UITheme.color("color-accent")
        style.set_border_width_all(ACTIVE_BORDER_WIDTH)
    return style
