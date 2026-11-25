class_name CardTopRow
extends Control
## 卡面顶栏：时代在左、等级居中、费用在右，三者都压在封面立绘上。
##
## 只有等级星有底色胶囊；时代节点与费用旋钮直接落在立绘上，靠描边与投影保持可读。
## 等级默认居中在卡片水平中线上，窄卡带奖励角标放不下时改在时代与角标之间居中，
## 并收紧星径与胶囊内边距，保证三者互不重叠。
##
## 全部状态字段导出：拖拽预览与战斗克隆由 `Node.duplicate()` 生成卡面，
## 非导出变量在副本里会回退，导出后副本才能保持同一张卡的外观。

const MARGIN := 2.0
## 子节点名：外部测试与 `set_stats` 类逻辑按名查找。
const COST_NAME := "CostBadge"
const REWARD_NAME := "CardBadge"
const ERA_NAME := "EraGlyph"

## 等级（1–7）。棋盘卡同样有等级，由卡表取值。
@export var level := 1
## 时代 id；棋盘卡面不显示时代，传空串。
@export var era := ""
## 费用位文本：购买卡为展示费用，自己棋盘的随从为发动技能费用；空串不画旋钮。
@export var cost := ""
## 奖励牌锁定的发现星级；空串不画角标。金色不再单独出角标，由金框与金色星点表达。
@export var reward := ""
## 三连：星点换成足金。
@export var golden := false

var _rail := Rect2()
var _knob_center := Vector2.ZERO
var _knob_r := 0.0
var _star_center := Vector2.ZERO
var _star_r := 3.4
var _era_center := Vector2.ZERO
var _era_size := 14
var _chips: Array[Dictionary] = []


static func create(
    level_value: int,
    era_id := "",
    cost_text := "",
    reward_text := "",
    is_golden := false,
) -> CardTopRow:
    var node := CardTopRow.new()
    node.name = "CardTopRow"
    node.level = level_value
    node.era = era_id
    node.cost = cost_text
    node.reward = reward_text
    node.golden = is_golden
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    node.set_anchors_preset(Control.PRESET_FULL_RECT)
    node.resized.connect(node._relayout)
    return node


func _ready() -> void:
    _relayout()


func _relayout() -> void:
    _layout()
    queue_redraw()


## 顶栏实际占用的条带（局部坐标，含等级胶囊与两侧节点）：整条顶栏铺满卡面做输入穿透，
## 测试与页面避让需要的是这条可见带，而不是整张卡面。
func band_rect() -> Rect2:
    if _rail.size.x <= 0.0:
        return Rect2()
    var band := _rail
    if era != "":
        band = band.merge(Rect2(_era_center - Vector2(_era_size, _era_size) * 0.5, Vector2(_era_size, _era_size)))
    if cost != "":
        band = band.merge(Rect2(_knob_center - Vector2(_knob_r, _knob_r), Vector2(_knob_r, _knob_r) * 2.0))
    for chip in _chips:
        var chip_center: Vector2 = chip["center"]
        var chip_r: float = chip["r"]
        band = band.merge(Rect2(chip_center - Vector2(chip_r, chip_r), Vector2(chip_r, chip_r) * 2.0))
    return band


func _draw() -> void:
    if _rail.size.x <= 0.0:
        return
    _panel(_rail, _rail.size.y * 0.5, UITheme.alpha("color-bg", 0.88), UITheme.alpha("color-accent-strong", 0.55))
    # 胶囊顶沿一条暗金线：把星条从立绘上「托」出来，和旧星条保持同一材质语言。
    draw_rect(
        Rect2(
            _rail.position + Vector2(_rail.size.y * 0.5, 1.0),
            Vector2(maxf(_rail.size.x - _rail.size.y, 1.0), 1.0),
        ),
        UITheme.alpha("color-golden", 0.14),
    )
    if cost != "":
        draw_circle(_knob_center, _knob_r, UITheme.color("gold-600"))
        draw_circle(_knob_center, _knob_r - 1.0, UITheme.color("gold-400"))
        draw_circle(
            _knob_center - Vector2(_knob_r * 0.3, _knob_r * 0.35),
            _knob_r * 0.3,
            UITheme.alpha("color-golden", 0.7),
        )
    _draw_stars()
    for chip in _chips:
        _draw_chip(chip)


func _layout() -> void:
    var w := size.x
    if w <= 0.0 or size.y <= 0.0:
        return
    var s := w / UITheme.metric("card-width")
    var rail_h := maxf(roundf(18.0 * s), 16.0)
    var cy := MARGIN + rail_h * 0.5
    # 时代与费用是同一档节点：直径取同一个值，两端视觉等重；基准取时代象形令牌。
    var node_d := clampf(roundf(UITheme.stage_metric("era-glyph-size") * s), 13.0, 16.0)
    _era_size = int(node_d)
    var left := MARGIN + 4.0
    if era != "":
        _era_center = Vector2(MARGIN + node_d * 0.5, cy)
        _sync_era(node_d)
        left = _era_center.x + node_d * 0.5 + 4.0
    _knob_r = node_d * 0.5
    _knob_center = Vector2(w - MARGIN - node_d * 0.5, cy)
    _sync_cost(clampi(roundi(12.0 * s), 9, 12))
    # 奖励角标排在费用左侧，从右往左；金色与潜行都不再占顶栏。
    var chip_r := minf(rail_h * 0.5 - 2.0, 5.0)
    var right := _knob_center.x - _knob_r - 4.0
    _chips.clear()
    if reward != "":
        _chips.append({
            "center": Vector2(right - chip_r, cy),
            "r": chip_r,
            "border": UITheme.color("color-accent-strong"),
        })
        _sync_reward(Vector2(right - chip_r, cy))
        right -= chip_r * 2.0 + 3.0
    else:
        _sync_reward(Vector2.ZERO)
    # 等级先尝试卡片水平中线；放不下时退回时代与角标之间居中。
    _star_center = Vector2(w * 0.5, cy)
    var steps := float(maxi(level, 1) - 1)
    var denom := maxf(2.0 + 2.1 * steps, 1.0)
    var base_r := clampf(3.4 * s, 1.9, 3.4)
    var star_pad := maxf(roundf(7.0 * s), 4.0)
    var half := minf(_star_center.x - left, right - _star_center.x)
    if base_r * denom * 0.5 + star_pad > half:
        _star_center = Vector2((left + right) * 0.5, cy)
        half = maxf(right - left, 6.0) * 0.5
    _star_r = base_r
    var stars_w := _star_r * denom
    if stars_w * 0.5 + star_pad > half:
        star_pad = 2.0
        _star_r = clampf((half - star_pad) * 2.0 / denom, 1.4, base_r)
        stars_w = _star_r * denom
    _rail = Rect2(_star_center.x - stars_w * 0.5 - star_pad, MARGIN, stars_w + star_pad * 2.0, rail_h)


## 星级：只画点亮的四角星，未点亮槽位不再占宽度；星色取时代合金色，三连换成足金。
func _draw_stars() -> void:
    var count := maxi(level, 0)
    if count <= 0:
        return
    var gap := _star_r * 2.1
    var total := gap * float(count - 1) + _star_r * 2.0
    var origin := _star_center.x - total * 0.5 + _star_r
    var color := _star_color()
    for index in count:
        var point := Vector2(origin + gap * float(index), _star_center.y)
        draw_colored_polygon(_star_points(point, _star_r * 1.6), UITheme.fade(color, 0.28))
        draw_colored_polygon(_star_points(point, _star_r), color.darkened(0.55))
        draw_colored_polygon(_star_points(point, _star_r * 0.78), color)


func _draw_chip(chip: Dictionary) -> void:
    var chip_center: Vector2 = chip["center"]
    var radius: float = chip["r"]
    var border: Color = chip["border"]
    draw_circle(chip_center, radius, UITheme.alpha("color-bg", 0.92))
    draw_arc(chip_center, radius - 0.5, 0.0, TAU, 20, UITheme.fade(border, 0.9), 1.0, true)


func _panel(rect: Rect2, radius: float, bg: Color, border: Color) -> void:
    var box := StyleBoxFlat.new()
    box.bg_color = bg
    box.border_color = border
    box.set_border_width_all(1)
    box.set_corner_radius_all(maxi(1, roundi(radius)))
    draw_style_box(box, rect)


func _star_color() -> Color:
    if golden:
        return UITheme.color("color-star")
    return FrameMaterial.tint_for_era(era).lightened(0.38)


static func _star_points(center: Vector2, radius: float) -> PackedVector2Array:
    var points := PackedVector2Array()
    for index in 8:
        var angle := -PI * 0.5 + index * PI * 0.25
        var r := radius if index % 2 == 0 else radius * 0.32
        points.append(center + Vector2(cos(angle), sin(angle)) * r)
    return points


## 时代节点：复用共享 `EraGlyph`；棋盘卡 era 为空时不建节点。
func _sync_era(node_d: float) -> void:
    var glyph := get_node_or_null(ERA_NAME) as EraGlyph
    if glyph == null:
        glyph = EraGlyph.create(era, int(node_d))
        glyph.name = ERA_NAME
        add_child(glyph)
    glyph.position = _era_center - Vector2(node_d, node_d) * 0.5
    glyph.size = Vector2(node_d, node_d)


## 费用数字：深色数字压在金色旋钮上，靠深色描边保持边缘。
func _sync_cost(font_size: int) -> void:
    var label := get_node_or_null(COST_NAME) as Label
    if cost == "":
        if label != null:
            label.visible = false
        return
    if label == null:
        label = _make_label(COST_NAME, UITheme.color("color-bg"), 2)
        add_child(label)
    label.visible = true
    label.text = cost
    label.add_theme_font_size_override("font_size", font_size)
    _place_center(label, _knob_center)


## 奖励角标：深底圆牌 + 语义色描边 + 居中文字；没有奖励时隐藏。
func _sync_reward(center: Vector2) -> void:
    var label := get_node_or_null(REWARD_NAME) as Label
    if reward == "":
        if label != null:
            label.visible = false
        return
    if label == null:
        label = _make_label(REWARD_NAME, UITheme.color("color-text"), 0)
        add_child(label)
    label.visible = true
    label.text = reward
    label.add_theme_font_size_override("font_size", UITheme.font_size("font-micro"))
    _place_center(label, center)


func _make_label(label_name: String, color: Color, outline: int) -> Label:
    var label := UIControls.label("", UITheme.font_size("font-micro"), false)
    label.name = label_name
    label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    label.add_theme_color_override("font_color", color)
    if outline > 0:
        label.add_theme_constant_override("outline_size", outline)
        label.add_theme_color_override("font_outline_color", UITheme.color("color-bg"))
    label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return label


## 按文字自身最小尺寸居中：固定矩形小于行高时文字会被顶到矩形下沿，数字看起来偏下。
static func _place_center(label: Label, center: Vector2) -> void:
    var minimum := label.get_combined_minimum_size()
    label.size = minimum
    label.position = center - minimum * 0.5
