class_name StatPlate
extends Control
## 攻防底牌：攻击为黄铜圆、血量为红血滴，数字白字黑描边。
##
## 空值与「—」不画底牌，只留描边文字，效果牌与未知身材保持原排版。

const ATTACK := "attack"
const HEALTH := "health"

## 底牌类型。导出以便 `Node.duplicate()` 复制：拖拽预览与战斗克隆都由
## `duplicate()` 生成卡面，非导出脚本变量在副本里会回退成默认攻击圆。
@export var kind := ATTACK


static func create(kind_value: String, text: String, font_size: int, label_name := "StatLabel") -> StatPlate:
	var node := StatPlate.new()
	node.name = "StatPlate"
	node.kind = kind_value
	var side := font_size + 4
	node.custom_minimum_size = Vector2(side, side)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var label := UIControls.label(text, font_size, false)
	label.name = label_name
	label.set_anchors_preset(Control.PRESET_FULL_RECT)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.add_theme_color_override("font_color", UITheme.color("color-white"))
	label.add_theme_color_override("font_outline_color", UITheme.color("color-black"))
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


## 是否画底牌装饰：空值与「—」只留描边文字，效果牌与未知身材保持原排版。
func draws_chrome() -> bool:
	var label := get_child(0) as Label
	if label == null:
		return false
	var value := label.text.strip_edges()
	return value != "" and value != "—"


func _draw() -> void:
	if not draws_chrome():
		return
	if kind == HEALTH:
		_draw_drop()
	else:
		_draw_circle()


## 攻击：带厚度的铜环 + 深井，数字落在井底，比原来的三层圆更有实物感。
func _draw_circle() -> void:
	var center := size * 0.5
	var radius := minf(size.x, size.y) * 0.5
	# 投影：让徽章从卡面上浮起来。
	draw_circle(center + Vector2(0.0, radius * 0.10), radius * 0.98, UITheme.alpha("color-black", 0.55))
	# 铜环外圈
	draw_circle(center, radius * 0.94, UITheme.color("color-plate-ring"))
	# 环上受光：左上到右上一段亮弧。
	draw_arc(
		center,
		radius * 0.90,
		PI * 1.08,
		PI * 1.92,
		28,
		UITheme.color("color-accent"),
		maxf(1.0, radius * 0.18),
		true,
	)
	# 内井
	draw_circle(center, radius * 0.74, UITheme.color("color-plate-well"))
	draw_arc(center, radius * 0.74, 0.0, TAU, 32, UITheme.fade(UITheme.color("color-accent"), 0.55), 1.0, true)


func _draw_drop() -> void:
	var w := size.x
	var h := size.y
	var center := Vector2(w * 0.5, h * 0.62)
	var radius := w * 0.36
	var tip := Vector2(w * 0.5, h * 0.10)
	var left := Vector2(w * 0.14, h * 0.62)
	var right := Vector2(w * 0.86, h * 0.62)
	# 先画略大一圈的暗红剪影，再压上正红，形成贴边的收边。
	# 血滴：深红外缘当厚度，内面正红，左上一小块高光。
	var rim := UITheme.color("color-danger-deep")
	var body := UITheme.color("color-danger")
	draw_circle(center + Vector2(0.0, radius * 0.10), radius * 1.16, UITheme.alpha("color-black", 0.45))
	draw_circle(center, radius * 1.14, rim)
	draw_colored_polygon(
		PackedVector2Array([
			tip + Vector2(0, -h * 0.04), left + Vector2(-w * 0.05, 0), right + Vector2(w * 0.05, 0),
		]),
		rim,
	)
	draw_circle(center, radius, body)
	draw_colored_polygon(PackedVector2Array([tip, left, right]), body)
	draw_circle(
		center + Vector2(-radius * 0.34, -radius * 0.30),
		radius * 0.24,
		UITheme.fade(UITheme.color("color-danger").lightened(0.4), 0.55),
	)
