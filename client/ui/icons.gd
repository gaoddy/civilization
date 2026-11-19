class_name Icons
extends RefCounted
## 共享图标入口：在代码里绘制矢量路径，不依赖图片或字体资源。
##
## 所有图标在 24×24 网格上定义，按 `rect` 等比映射到目标区域；
## 只在 `_draw()` 或 `NOTIFICATION_DRAW` 内调用，未知图标不绘制、不报错。

const GRID := 24.0
const ARC_SEGMENTS := 24


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
