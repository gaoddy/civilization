class_name CardFrame
extends Control
## 卡框：三条嵌套描边，全部由代码绘制。
##
## 外线黄铜、中缝底色、内线弱描边。等级星不在这里：由卡面顶栏 `CardTopRow`
## 承载，卡框只负责边框本身。

const OUTER_INSET := 1.2
const MID_INSET := 2.4
const INNER_INSET := 4.6


static func create() -> CardFrame:
	var node := CardFrame.new()
	node.name = "CardFrame"
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


func _draw_rule(inset: float, radius: float, color: Color) -> void:
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.color("color-transparent")
	box.draw_center = false
	box.border_color = color
	box.set_border_width_all(1)
	box.set_corner_radius_all(maxi(1, roundi(radius)))
	draw_style_box(box, Rect2(Vector2(inset, inset), size - Vector2(inset, inset) * 2.0))
