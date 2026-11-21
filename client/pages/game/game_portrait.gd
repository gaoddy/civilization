class_name GamePortrait
extends AvatarFrame
## 方形头像块：继承 AvatarFrame 的圆角裁切与叠在头像之上的描边；`show_name` 为真时底部附 18 高名字条。
##
## 传入 `art_path` 且资源存在时叠加头像纹理并隐藏首字；缺失或加载失败回退首字色块。
## 名字条挂在裁切内叠层，随头像一起收进圆角。
## 只做展示，不持有状态；详情登记由调用方通过 `DetailLayer.track` 完成。

const NAME_HEIGHT := 18

var _name_label: Label = null


func _init() -> void:
	super()
	name = "GamePortrait"
	var side := UITheme.stage_metric("portrait-size")
	custom_minimum_size = Vector2(side, side)
	mouse_filter = Control.MOUSE_FILTER_STOP
	_apply_tint(UITheme.color("color-border-strong"))


func setup(display_name: String, glyph: String, tint: Color, show_name := true, art_path := "") -> void:
	_apply_tint(tint)
	var glyph_color := tint.lightened(0.5)
	glyph_color.a = GLYPH_ALPHA
	set_glyph(glyph, glyph_color)
	if show_name:
		if _name_label == null:
			_build_name_label()
		_name_label.text = display_name
	elif _name_label != null:
		_name_label.queue_free()
		_name_label = null
	set_art(art_path)


func _apply_tint(tint: Color) -> void:
	configure(tint.darkened(0.55), tint)


func _build_name_label() -> void:
	_name_label = UIControls.label("", UITheme.font_size("font-caption"), false)
	_name_label.name = "PortraitName"
	_name_label.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	_name_label.offset_top = -NAME_HEIGHT
	_name_label.offset_bottom = 0
	_name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_name_label.max_lines_visible = 1
	_name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	_name_label.add_theme_color_override("font_color", UITheme.color("color-text"))
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.alpha("color-bg", 0.72)
	box.border_color = UITheme.color("color-transparent")
	box.set_content_margin_all(0)
	_name_label.add_theme_stylebox_override("normal", box)
	_name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	clipped_content().add_child(_name_label)
