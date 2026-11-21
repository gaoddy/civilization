class_name Icons
extends RefCounted
## 共享图标入口：在代码里绘制矢量路径，不依赖图片或字体资源。
##
## 所有图标在 24×24 网格上定义，按 `rect` 等比映射到目标区域；
## 只在 `_draw()` 或 `NOTIFICATION_DRAW` 内调用，未知图标不绘制、不报错。

const GRID := 24.0
const ARC_SEGMENTS := 24
## 时代象形在独立的 16×16 网格上定义，与 24 网格的通用图标分开映射。
const ERA_GRID := 16.0
const ERA_CURVE_SEGMENTS := 8


static func draw(canvas: CanvasItem, icon: String, rect: Rect2, color: Color, width: float = 2.0) -> void:
	var stroke := _stroke(rect, width)
	match icon:
		"upgrade":
			_draw_upgrade(canvas, rect, color, stroke)
		"reroll":
			_draw_reroll(canvas, rect, color, stroke)
		"freeze":
			_draw_freeze(canvas, rect, color, stroke)
		"hero_power":
			_draw_hero_power(canvas, rect, color)
		"passive_skill":
			_draw_passive_skill(canvas, rect, color, stroke)
		_:
			pass


## 向上箭头：竖轴加人字箭头，表示升级时代。
static func _draw_upgrade(canvas: CanvasItem, rect: Rect2, color: Color, stroke: float) -> void:
	_line(canvas, rect, Vector2(12, 20), Vector2(12, 5), color, stroke)
	_line(canvas, rect, Vector2(6.5, 10.5), Vector2(12, 5), color, stroke)
	_line(canvas, rect, Vector2(17.5, 10.5), Vector2(12, 5), color, stroke)


## 环形箭头：约 300° 圆弧加箭头，表示刷新商店。
static func _draw_reroll(canvas: CanvasItem, rect: Rect2, color: Color, stroke: float) -> void:
	var center := _map(rect, Vector2(12, 12))
	var radius := 7.2 * _scale(rect)
	var start := -PI * 0.5
	var end := start + TAU * 0.83
	canvas.draw_arc(center, radius, start, end, ARC_SEGMENTS, color, stroke, true)
	var direction := Vector2(cos(end), sin(end))
	var tangent := Vector2(-sin(end), cos(end))
	var tip := center + direction * radius
	var head := 3.4 * _scale(rect)
	var points := PackedVector2Array([
		tip,
		tip - tangent * head + direction * head * 0.8,
		tip - tangent * head - direction * head * 0.8,
	])
	canvas.draw_colored_polygon(points, color)


## 雪花：三条主轴加端点短枝，表示冻结。
static func _draw_freeze(canvas: CanvasItem, rect: Rect2, color: Color, stroke: float) -> void:
	var center := _map(rect, Vector2(12, 12))
	var radius := 8.5 * _scale(rect)
	var branch := 3.4 * _scale(rect)
	for index in 3:
		var angle := PI * index / 3.0
		var direction := Vector2(cos(angle), sin(angle))
		var first := center - direction * radius
		var second := center + direction * radius
		canvas.draw_line(first, second, color, stroke, true)
		for end in [first, second]:
			var inward := -direction if end == second else direction
			for sign_value in [-1.0, 1.0]:
				var twig := inward.rotated(0.7 * sign_value) * branch
				canvas.draw_line(end, end + twig, color, stroke, true)


## 四角星：实心菱形收腰，表示英雄技能。
static func _draw_hero_power(canvas: CanvasItem, rect: Rect2, color: Color) -> void:
	var center := _map(rect, Vector2(12, 12))
	var scale := _scale(rect)
	var points := PackedVector2Array()
	for index in 8:
		var angle := -PI * 0.5 + PI * index / 4.0
		var reach := 10.0 if index % 2 == 0 else 3.8
		points.append(center + Vector2(cos(angle), sin(angle)) * reach * scale)
	canvas.draw_colored_polygon(points, color)


## 同心圆：外环加实心圆心，表示常驻生效、不可点击的被动技能。
static func _draw_passive_skill(
	canvas: CanvasItem, rect: Rect2, color: Color, stroke: float
) -> void:
	var center := _map(rect, Vector2(12, 12))
	var scale := _scale(rect)
	canvas.draw_arc(center, 8.5 * scale, 0.0, TAU, ARC_SEGMENTS, color, stroke, true)
	canvas.draw_circle(center, 3.2 * scale, color)


static func _line(
	canvas: CanvasItem, rect: Rect2, from: Vector2, to: Vector2, color: Color, stroke: float
) -> void:
	canvas.draw_line(_map(rect, from), _map(rect, to), color, stroke, true)


static func _scale(rect: Rect2) -> float:
	return minf(rect.size.x, rect.size.y) / GRID


static func _stroke(rect: Rect2, width: float) -> float:
	return maxf(1.0, width * _scale(rect))


static func _map(rect: Rect2, point: Vector2) -> Vector2:
	var side := minf(rect.size.x, rect.size.y)
	var origin := rect.position + (rect.size - Vector2(side, side)) * 0.5
	return origin + point * (side / GRID)

# ── 时代象形 ──────────────────────────────────────────────────────────────
#
# 源坐标 16×16，按目标矩形等比映射；主形以 24% 不透明度填充，轮廓与细线用 color。
# 与 24 网格的通用图标分开映射，避免两套网格混算。未知 era 不绘制、不报错。


static func draw_era(
	canvas: CanvasItem, era_id: String, rect: Rect2, color: Color, width: float = 1.4
) -> void:
	var stroke := maxf(1.0, width * _era_scale(rect))
	var fill := color
	fill.a *= 0.24
	match era_id:
		"prehistoric":
			_era_prehistoric(canvas, rect, color, fill, stroke)
		"primitive":
			_era_primitive(canvas, rect, color, fill, stroke)
		"classical":
			_era_classical(canvas, rect, color, fill, stroke)
		"feudal":
			_era_feudal(canvas, rect, color, fill, stroke)
		"industrial":
			_era_industrial(canvas, rect, color, fill, stroke)
		"modern":
			_era_modern(canvas, rect, color, fill, stroke)
		"future":
			_era_future(canvas, rect, color, fill, stroke)
		_:
			pass


## 石斧：木柄 + 燧石斧头 + 绑扎线。
static func _era_prehistoric(
	canvas: CanvasItem, rect: Rect2, color: Color, fill: Color, stroke: float
) -> void:
	var head: Array[Vector2] = [Vector2(9, 6.6)]
	head.append_array(
		_era_cubic(Vector2(9, 6.6), Vector2(8.8, 3.4), Vector2(11.2, 1.4), Vector2(14.2, 1.8))
	)
	head.append_array(
		_era_cubic(Vector2(14.2, 1.8), Vector2(14.6, 4.6), Vector2(12.6, 7.2), Vector2(9.6, 7.8))
	)
	_era_fill(canvas, rect, head, color, fill, stroke)
	_era_polyline(canvas, rect, [Vector2(2.8, 13.6), Vector2(10, 6.2)], color, stroke)
	_era_polyline(canvas, rect, [Vector2(9.4, 7.4), Vector2(10.6, 5.2)], color, stroke)


## 篝火：外焰 + 内焰 + 交叉木柴。
static func _era_primitive(
	canvas: CanvasItem, rect: Rect2, color: Color, fill: Color, stroke: float
) -> void:
	var outer: Array[Vector2] = [Vector2(8, 12.8)]
	outer.append_array(
		_era_cubic(Vector2(8, 12.8), Vector2(4.8, 10.4), Vector2(5.4, 7), Vector2(8, 2.4))
	)
	outer.append_array(
		_era_cubic(Vector2(8, 2.4), Vector2(8.9, 6), Vector2(11.3, 6.9), Vector2(11.3, 9.5))
	)
	outer.append_array(
		_era_cubic(Vector2(11.3, 9.5), Vector2(11.3, 11.6), Vector2(9.7, 12.8), Vector2(8, 12.8))
	)
	_era_fill(canvas, rect, outer, color, fill, stroke)
	var inner: Array[Vector2] = [Vector2(8, 11.2)]
	inner.append_array(
		_era_cubic(Vector2(8, 11.2), Vector2(6.7, 9.8), Vector2(7.1, 8.2), Vector2(8, 6.4))
	)
	inner.append_array(
		_era_cubic(Vector2(8, 6.4), Vector2(8.5, 8), Vector2(9.8, 8.6), Vector2(9.8, 9.9))
	)
	inner.append_array(
		_era_cubic(Vector2(9.8, 9.9), Vector2(9.8, 10.8), Vector2(9, 11.2), Vector2(8, 11.2))
	)
	_era_polyline(canvas, rect, inner, color, stroke)
	_era_polyline(canvas, rect, [Vector2(3.2, 14), Vector2(12.8, 11.7)], color, stroke)
	_era_polyline(canvas, rect, [Vector2(12.8, 14), Vector2(3.2, 11.7)], color, stroke)


## 神庙：山花 + 额枋 + 三柱 + 基座。
static func _era_classical(
	canvas: CanvasItem, rect: Rect2, color: Color, fill: Color, stroke: float
) -> void:
	_era_fill(
		canvas, rect, [Vector2(2, 6.2), Vector2(8, 2.1), Vector2(14, 6.2)], color, fill, stroke
	)
	_era_polyline(canvas, rect, [Vector2(3, 7), Vector2(13, 7)], color, stroke)
	for x in [4.4, 8.0, 11.6]:
		_era_polyline(canvas, rect, [Vector2(x, 7.7), Vector2(x, 12.2)], color, stroke)
	_era_polyline(canvas, rect, [Vector2(2.8, 13.5), Vector2(13.2, 13.5)], color, stroke)


## 城堡：城垛 + 拱门。
static func _era_feudal(
	canvas: CanvasItem, rect: Rect2, color: Color, fill: Color, stroke: float
) -> void:
	var wall: Array[Vector2] = [
		Vector2(3, 13.6), Vector2(3, 6.4), Vector2(4.2, 6.4), Vector2(4.2, 4.5),
		Vector2(6, 4.5), Vector2(6, 6.4), Vector2(7.2, 6.4), Vector2(7.2, 4.5),
		Vector2(9, 4.5), Vector2(9, 6.4), Vector2(10.4, 6.4), Vector2(10.4, 4.5),
		Vector2(12.2, 4.5), Vector2(12.2, 6.4), Vector2(13.4, 6.4), Vector2(13.4, 13.6),
	]
	_era_fill(canvas, rect, wall, color, fill, stroke)
	var arch: Array[Vector2] = [Vector2(6.9, 13.6), Vector2(6.9, 11)]
	for index in range(1, ERA_CURVE_SEGMENTS + 1):
		var angle := PI - PI * index / float(ERA_CURVE_SEGMENTS)
		arch.append(Vector2(8.05 + cos(angle) * 1.15, 11.0 - sin(angle) * 1.15))
	arch.append(Vector2(9.2, 13.6))
	_era_polyline(canvas, rect, arch, color, stroke)


## 齿轮：六齿 + 轮轴。
static func _era_industrial(
	canvas: CanvasItem, rect: Rect2, color: Color, fill: Color, stroke: float
) -> void:
	var scale_factor := _era_scale(rect)
	var center := _era_map(rect, Vector2(8, 8))
	canvas.draw_circle(center, 3.4 * scale_factor, fill)
	canvas.draw_arc(center, 3.4 * scale_factor, 0.0, TAU, 32, color, stroke, true)
	for index in 6:
		var direction := Vector2.RIGHT.rotated(-PI * 0.5 + PI * index / 3.0)
		canvas.draw_line(
			center + direction * 4.05 * scale_factor,
			center + direction * 5.42 * scale_factor,
			color,
			stroke,
			true,
		)
	canvas.draw_circle(center, 0.9 * scale_factor, color)


## 原子：核 + 双轨道。
static func _era_modern(
	canvas: CanvasItem, rect: Rect2, color: Color, fill: Color, stroke: float
) -> void:
	var scale_factor := _era_scale(rect)
	var center := _era_map(rect, Vector2(8, 8))
	canvas.draw_circle(center, 1.5 * scale_factor, color)
	for sign_value in [1.0, -1.0]:
		var tilt: float = deg_to_rad(32.0) * float(sign_value)
		var ring := PackedVector2Array()
		for index in 40:
			var angle := TAU * index / 40.0
			var local := Vector2(cos(angle) * 6.2, sin(angle) * 2.5).rotated(tilt)
			ring.append(center + local * scale_factor)
		ring.append(ring[0])
		canvas.draw_polyline(ring, color, stroke, true)


## 火箭：箭身 + 舷窗 + 尾翼 + 焰。
static func _era_future(
	canvas: CanvasItem, rect: Rect2, color: Color, fill: Color, stroke: float
) -> void:
	var body: Array[Vector2] = [Vector2(8, 1.9)]
	body.append_array(
		_era_cubic(Vector2(8, 1.9), Vector2(10.3, 4.1), Vector2(10.3, 8.4), Vector2(8, 11.6))
	)
	body.append_array(
		_era_cubic(Vector2(8, 11.6), Vector2(5.7, 8.4), Vector2(5.7, 4.1), Vector2(8, 1.9))
	)
	_era_fill(canvas, rect, body, color, fill, stroke)
	canvas.draw_arc(
		_era_map(rect, Vector2(8, 5.4)),
		1.15 * _era_scale(rect),
		0.0,
		TAU,
		24,
		color,
		stroke,
		true,
	)
	_era_polyline(canvas, rect, [Vector2(6.1, 9.7), Vector2(3.9, 11.9), Vector2(6, 12.7)], color, stroke)
	_era_polyline(
		canvas, rect, [Vector2(9.9, 9.7), Vector2(12.1, 11.9), Vector2(10, 12.7)], color, stroke
	)
	var flame: Array[Vector2] = [Vector2(8, 12.6)]
	flame.append_array(
		_era_cubic(Vector2(8, 12.6), Vector2(7.3, 13.3), Vector2(7.6, 14.2), Vector2(8, 14.7))
	)
	flame.append_array(
		_era_cubic(Vector2(8, 14.7), Vector2(8.4, 14.2), Vector2(8.7, 13.3), Vector2(8, 12.6))
	)
	_era_fill(canvas, rect, flame, color, fill, stroke)


## 三次贝塞尔按固定段数采样，返回源坐标（不含起点、含终点）。
static func _era_cubic(
	from: Vector2, control_a: Vector2, control_b: Vector2, to: Vector2
) -> Array[Vector2]:
	var points: Array[Vector2] = []
	for index in range(1, ERA_CURVE_SEGMENTS + 1):
		var t := float(index) / float(ERA_CURVE_SEGMENTS)
		var u := 1.0 - t
		points.append(
			from * u * u * u
			+ control_a * 3.0 * u * u * t
			+ control_b * 3.0 * u * t * t
			+ to * t * t * t
		)
	return points


## 闭合主形：填充 24% 不透明度的 color，再描一圈轮廓。
static func _era_fill(
	canvas: CanvasItem, rect: Rect2, points: Array[Vector2], color: Color, fill: Color, stroke: float
) -> void:
	var mapped := PackedVector2Array()
	for point in points:
		mapped.append(_era_map(rect, point))
	if mapped.size() < 3:
		return
	canvas.draw_colored_polygon(mapped, fill)
	mapped.append(mapped[0])
	canvas.draw_polyline(mapped, color, stroke, true)


static func _era_polyline(
	canvas: CanvasItem, rect: Rect2, points: Array, color: Color, stroke: float
) -> void:
	var mapped := PackedVector2Array()
	for point in points:
		mapped.append(_era_map(rect, point))
	if mapped.size() >= 2:
		canvas.draw_polyline(mapped, color, stroke, true)


static func _era_scale(rect: Rect2) -> float:
	return minf(rect.size.x, rect.size.y) / ERA_GRID


static func _era_map(rect: Rect2, point: Vector2) -> Vector2:
	var side := minf(rect.size.x, rect.size.y)
	var origin := rect.position + (rect.size - Vector2(side, side)) * 0.5
	return origin + point * (side / ERA_GRID)
