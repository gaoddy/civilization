class_name CardFrame
extends Control
## 卡框：三条嵌套描边 + 顶部一排星级，全部由代码绘制。
##
## 外线黄铜、中缝底色、内线弱描边；星级取卡牌等级（1–7），替代原稀有度徽章。

const OUTER_INSET := 1.2
const MID_INSET := 2.4
const INNER_INSET := 4.6
## 星点按卡宽/高整体缩放；半径与间距取手机 70 宽手牌仍能塞进顶行角标间隙的上限。
const STAR_RADIUS := 4.8
const STAR_GAP := 10.6
const STAR_Y := 10.5
## 星条底板的左右内边距与高度：把星星从卡面立绘上「托」出来，避免和底图混在一起。
const STAR_PLATE_PAD := 7.0
const STAR_PLATE_HEIGHT := 16.0
## 星槽总数等于上限等级；暗星把「还差几级」也一并标出来。
const STAR_SLOTS := 7

## 卡牌等级；导出以便 `Node.duplicate()` 复制：拖拽预览与战斗克隆都由
## `duplicate()` 生成卡面，非导出脚本变量在副本里会回退成 1 星。
@export var level := 1
## 时代 id 与三连标记：星点按时代合金色取值，三连换成更亮的金并加外发光。
@export var era := ""
@export var golden := false
var star_count: int:
	get:
		return clampi(level, 0, 7)


static func create(level_value: int, era_id := "", is_golden := false) -> CardFrame:
	var node := CardFrame.new()
	node.name = "CardFrame"
	node.level = level_value
	node.era = era_id
	node.golden = is_golden
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


## 固定 7 个槽位：点亮 level 个，其余留暗星。整行长度不变，数星星不用从头扫。
func _draw_stars(s: float) -> void:
	var count := star_count
	if count <= 0:
		return
	var total := float(STAR_SLOTS - 1) * STAR_GAP * s
	var center_y := STAR_Y * s
	var color := _star_color()
	_draw_star_plate(total, center_y, s)
	for i in STAR_SLOTS:
		var center := Vector2(size.x * 0.5 - total * 0.5 + i * STAR_GAP * s, center_y)
		if i < count:
			_draw_star(center, STAR_RADIUS * s, color)
		else:
			_draw_star_empty(center, STAR_RADIUS * s, color)


## 星条底板：一条深色胶囊把整行星点从立绘上分离出来。
func _draw_star_plate(total: float, center_y: float, s: float) -> void:
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.alpha("color-bg", 0.86)
	box.border_color = UITheme.alpha("color-accent-strong", 0.55)
	box.set_border_width_all(1)
	box.set_corner_radius_all(maxi(2, roundi(STAR_PLATE_HEIGHT * s * 0.5)))
	var width := total + STAR_RADIUS * 2.0 * s + STAR_PLATE_PAD * 2.0 * s
	var height := STAR_PLATE_HEIGHT * s
	draw_style_box(
		box,
		Rect2(size.x * 0.5 - width * 0.5, center_y - height * 0.5, width, height),
	)


## 星色跟卡框走：时代合金色，三连提亮到足金。
func _star_color() -> Color:
	if golden:
		return UITheme.color("color-star")
	return FrameMaterial.tint_for_era(era).lightened(0.38)


## 金属星：外发光 + 暗描边 + 星体 + 左上倒角高光，小尺寸下仍能读出厚度。
func _draw_star(center: Vector2, radius: float, color: Color) -> void:
	var glow_a := 0.50 if golden else 0.34
	var glow := UITheme.fade(color, glow_a)
	draw_colored_polygon(_star_points(center, radius * 1.85), glow)
	# 深色投影再往外压一圈，亮度再高也不会糊在浅色立绘上。
	draw_colored_polygon(
		_star_points(center + Vector2(0.0, radius * 0.35), radius * 1.12),
		UITheme.alpha("color-bg", 0.75),
	)
	draw_colored_polygon(_star_points(center, radius), color.darkened(0.72))
	draw_colored_polygon(_star_points(center, radius * 0.84), color)
	draw_colored_polygon(
		_star_points(center - Vector2(radius, radius) * 0.20, radius * 0.40),
		color.lightened(0.52),
	)


## 暗星：只有很淡的填充和一圈细边，作为「还没点亮」的槽位。
func _draw_star_empty(center: Vector2, radius: float, color: Color) -> void:
	var dim := UITheme.fade(color, 0.10)
	draw_colored_polygon(_star_points(center, radius), dim)
	var outline := _star_points(center, radius)
	outline.append(outline[0])
	draw_polyline(outline, UITheme.fade(color, 0.22), maxf(1.0, radius * 0.24), true)


## 四角星顶点：外角长、内角短，按顺时针生成。
func _star_points(center: Vector2, radius: float) -> PackedVector2Array:
	var points := PackedVector2Array()
	for i in 8:
		var angle := -PI * 0.5 + i * PI * 0.25
		var r := radius if i % 2 == 0 else radius * 0.32
		points.append(center + Vector2(cos(angle), sin(angle)) * r)
	return points
