class_name CardFrame
extends Control
## 卡框：三条嵌套描边 + 顶部一排星级，全部由代码绘制。
##
## 外线黄铜、中缝底色、内线弱描边；星级取卡牌等级（1–7），替代原稀有度徽章。

const OUTER_INSET := 1.2
const MID_INSET := 2.4
const INNER_INSET := 4.6
const STAR_RADIUS := 2.15
const STAR_GAP := 5.0
const STAR_Y := 7.4

## 卡牌等级；导出以便 `Node.duplicate()` 复制：拖拽预览与战斗克隆都由
## `duplicate()` 生成卡面，非导出脚本变量在副本里会回退成 1 星。
@export var level := 1
var star_count: int:
	get:
		return clampi(level, 0, 7)


static func create(level_value: int) -> CardFrame:
	var node := CardFrame.new()
	node.name = "CardFrame"
	node.level = level_value
	node.set_anchors_preset(Control.PRESET_FULL_RECT)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node.resized.connect(node.queue_redraw)
	return node


func _draw() -> void:
	var base := Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
	var s := minf(size.x / base.x, size.y / base.y)
	_draw_rule(OUTER_INSET * s, 8.0 * s, UITheme.alpha("color-accent-strong", 0.60))
	_draw_rule(MID_INSET * s, 6.8 * s, UITheme.color("color-bg"))
	_draw_rule(INNER_INSET * s, 6.0 * s, UITheme.alpha("color-border-strong", 0.55))
	_draw_stars(s)


func _draw_rule(inset: float, radius: float, color: Color) -> void:
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.color("color-transparent")
	box.draw_center = false
	box.border_color = color
	box.set_border_width_all(1)
	box.set_corner_radius_all(maxi(1, roundi(radius)))
	draw_style_box(box, Rect2(Vector2(inset, inset), size - Vector2(inset, inset) * 2.0))


func _draw_stars(s: float) -> void:
	var count := star_count
	if count <= 0:
		return
	var total := (count - 1) * STAR_GAP * s
	var center_y := STAR_Y * s
	for i in count:
		var center := Vector2(size.x * 0.5 - total * 0.5 + i * STAR_GAP * s, center_y)
		_draw_star(center, STAR_RADIUS * s, UITheme.alpha("color-accent", 0.95))


func _draw_star(center: Vector2, radius: float, color: Color) -> void:
	var points: Array[Vector2] = []
	for i in 8:
		var angle := -PI * 0.5 + i * PI * 0.25
		var r := radius if i % 2 == 0 else radius * 0.32
		points.append(center + Vector2(cos(angle), sin(angle)) * r)
	for i in 8:
		draw_colored_polygon(
			PackedVector2Array([center, points[i], points[(i + 1) % 8]]), color
		)
