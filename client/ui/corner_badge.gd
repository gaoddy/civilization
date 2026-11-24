class_name CornerBadge
extends Control
## 卡牌左上角标：深底 + 铜边 + 顶部高光的斜角小牌。
##
## 替代原来的扁平胶囊：费用、星级与潜行标记共用同一枚小牌，
## 尺寸按文字内容撑开，和卡框共用一套「有厚度」的视觉语言。

## 语义色变体，与 `UITheme.pill_style` 的取值保持一致。
@export var variant := "accent"

## 角标文案：外部按文本读改，尺寸随内容重算，不需要知道内部标签结构。
var text: String:
	get:
		return "" if _badge_label == null else _badge_label.text
	set(value):
		if _badge_label == null:
			return
		_badge_label.text = value
		_fit()

const PAD_X := 6.0
const PAD_Y := 2.0
const RADIUS_INSET := 1.4

var _badge_label: Label = null


static func create(text: String, variant_value := "accent") -> CornerBadge:
	var node := CornerBadge.new()
	node.name = "CornerBadge"
	node.variant = variant_value
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node._build(text)
	return node


func _build(text: String) -> void:
	_badge_label = UIControls.label(text, UITheme.font_size("font-caption"), false)
	_badge_label.name = "BadgeLabel"
	_badge_label.add_theme_color_override("font_color", UITheme.color("color-text"))
	_badge_label.add_theme_constant_override("outline_size", 2)
	_badge_label.add_theme_color_override("font_outline_color", UITheme.color("color-bg"))
	_badge_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_badge_label)
	_fit()
	resized.connect(func() -> void:
		_badge_label.position = Vector2(PAD_X, (size.y - _badge_label.size.y) * 0.5)
	)


## 按文字内容撑开小牌：换文案后同步最小尺寸与标签位置。
func _fit() -> void:
	var content := _badge_label.get_combined_minimum_size()
	custom_minimum_size = Vector2(
		content.x + PAD_X * 2.0,
		float(UITheme.font_size("font-caption") + int(PAD_Y) * 2 + 2),
	)
	size = custom_minimum_size
	_badge_label.position = Vector2(PAD_X, (size.y - content.y) * 0.5)


func _draw() -> void:
	var accent := _accent()
	var radius := size.y * 0.5
	# 外牌：压深的底色，压住卡面立绘。
	var outer := StyleBoxFlat.new()
	outer.bg_color = accent.darkened(0.78)
	outer.border_color = accent.darkened(0.15)
	outer.set_border_width_all(1)
	outer.set_corner_radius_all(int(maxf(radius, 3.0)))
	draw_style_box(outer, Rect2(Vector2.ZERO, size))

	# 内面：抬亮的斜面，留出一圈深边当厚度。
	var inner := StyleBoxFlat.new()
	inner.bg_color = UITheme.fade(accent, 0.26)
	inner.set_corner_radius_all(int(maxf(radius - RADIUS_INSET, 2.0)))
	inner.set_border_width_all(0)
	draw_style_box(inner, Rect2(Vector2.ONE * RADIUS_INSET, size - Vector2.ONE * RADIUS_INSET * 2.0))

	# 顶部高光：一小段亮线，读成铜牌受光的那一面。
	draw_line(
		Vector2(radius * 0.9, RADIUS_INSET + 1.0),
		Vector2(size.x - radius * 0.9, RADIUS_INSET + 1.0),
		UITheme.fade(UITheme.color("color-accent").lightened(0.45), 0.40),
		1.0,
		true,
	)


func _accent() -> Color:
	match variant:
		"success":
			return UITheme.color("color-success")
		"danger":
			return UITheme.color("color-danger")
		"info":
			return UITheme.color("color-info")
		_:
			return UITheme.color("color-accent")
