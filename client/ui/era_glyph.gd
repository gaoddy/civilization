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
	Icons.draw_era(
		self, era_id, Rect2(Vector2.ZERO, size), UITheme.alpha("color-accent", 0.62), 1.4
	)
