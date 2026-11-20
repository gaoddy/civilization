class_name StatPlate
extends Control
## 攻防底牌：攻击为黄铜圆、血量为红血滴，数字白字黑描边。
##
## 空值与「—」不画底牌，只留描边文字，效果牌与未知身材保持原排版。

const ATTACK := "attack"
const HEALTH := "health"

var kind := ATTACK


static func create(kind_value: String, text: String, font_size: int, label_name := "StatLabel") -> StatPlate:
	var node := StatPlate.new()
	node.name = "StatPlate"
	node.kind = kind_value
	var side := font_size + 4
	node.custom_minimum_size = Vector2(side, side)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var label := UI.label(text, font_size, false)
	label.name = label_name
	label.set_anchors_preset(Control.PRESET_FULL_RECT)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.add_theme_color_override("font_color", Color.WHITE)
	label.add_theme_color_override("font_outline_color", Color.BLACK)
	label.add_theme_constant_override("outline_size", 2)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node.add_child(label)
	node.resized.connect(node.queue_redraw)
	return node


func set_text(text: String) -> void:
	var label := get_child(0) as Label
	if label != null and label.text != text:
		label.text = text
		queue_redraw()


func _draw() -> void:
	# 空值与「—」不画底牌，只留描边文字：效果牌、未知身材保持原排版。
	var label := get_child(0) as Label
	if label == null:
		return
	var value := label.text.strip_edges()
	if value == "" or value == "—":
		return
	if kind == HEALTH:
		_draw_drop()
	else:
		_draw_circle()


func _draw_circle() -> void:
	var center := size * 0.5
	var radius := minf(size.x, size.y) * 0.5
	draw_circle(center, radius * 0.96, UITheme.color("color-bg"))
	draw_circle(center, radius * 0.88, UITheme.color("color-accent-strong"))
	draw_circle(center, radius * 0.64, UITheme.color("color-accent"))
	draw_arc(
		center, radius * 0.88, 0.0, TAU, 48, UITheme.color("color-bg"), 1.0, true
	)


func _draw_drop() -> void:
	var w := size.x
	var h := size.y
	var center := Vector2(w * 0.5, h * 0.62)
	var radius := w * 0.36
	var tip := Vector2(w * 0.5, h * 0.10)
	var left := Vector2(w * 0.14, h * 0.62)
	var right := Vector2(w * 0.86, h * 0.62)
	# 先画略大一圈的暗红剪影，再压上正红，形成贴边的收边。
	var silhouette := UITheme.color("color-danger").darkened(0.45)
	var body := UITheme.color("color-danger")
	draw_circle(center, radius * 1.12, silhouette)
	draw_colored_polygon(
		PackedVector2Array([tip + Vector2(0, -h * 0.03), left + Vector2(-w * 0.03, 0), right + Vector2(w * 0.03, 0)]),
		silhouette,
	)
	draw_circle(center, radius, body)
	draw_colored_polygon(PackedVector2Array([tip, left, right]), body)
