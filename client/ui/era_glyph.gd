class_name EraGlyph
extends Control
## 时代象形：无外框的代码绘制图标，卡面 14px、详情 24px。
##
## 只按 62% 金色低调出现，不参与悬停与点击。

## 时代标识。导出以便 `Node.duplicate()` 复制：拖拽预览与战斗克隆都由
## `duplicate()` 生成卡面，非导出脚本变量在副本里会丢失时代图标。
@export var era_id := ""


static func create(era_value: String, side: int) -> EraGlyph:
	var node := EraGlyph.new()
	node.name = "EraGlyph"
	node.era_id = era_value
	node.custom_minimum_size = Vector2(side, side)
	node.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node.resized.connect(node.queue_redraw)
	return node


func _draw() -> void:
	# 徽章底盘：深底 + 时代色铜边 + 顶部受光，图标不再直接浮在立绘上。
	var tint := FrameMaterial.tint_for_era(era_id)
	var center := size * 0.5
	var radius := minf(size.x, size.y) * 0.5
	draw_circle(center, radius, UITheme.alpha("color-bg", 0.88))
	draw_arc(center, radius - 0.5, 0.0, TAU, 32, UITheme.fade(tint, 0.85), 1.0, true)
	draw_arc(
		center,
		radius - 1.6,
		-PI * 0.85,
		-PI * 0.15,
		16,
		UITheme.fade(UITheme.color("color-accent").lightened(0.45), 0.42),
		1.0,
		true,
	)
	# 图标收进底盘里侧，按时代合金色提亮。
	var inset := size * 0.16
	Icons.draw_era(
		self,
		era_id,
		Rect2(inset, size - inset * 2.0),
		tint.lightened(0.35),
		1.8,
	)
