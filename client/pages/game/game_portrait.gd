class_name GamePortrait
extends PanelContainer
## 头像块：可选贴图 + 玩法色底 + 名字首字回退，64×64；`show_name` 为真时底部附 18 高名字条。
##
## 传入 `art_path` 且资源存在时叠加头像纹理并隐藏首字；缺失或加载失败回退首字色块。
## 只做展示，不持有状态；详情登记由调用方通过 `GameDetailLayer.track` 完成。

const NAME_HEIGHT := 18
const GLYPH_ALPHA := 0.55

var _glyph: Label
var _name_label: Label = null


func _init() -> void:
	var side := UITheme.stage_metric("portrait-size")
	custom_minimum_size = Vector2(side, side)
	mouse_filter = Control.MOUSE_FILTER_STOP
	add_theme_stylebox_override("panel", _panel_style(UITheme.color("color-border-strong")))
	_glyph = UI.label("?", UITheme.font_size("font-hero"), false)
	_glyph.name = "PortraitGlyph"
	_glyph.set_anchors_preset(Control.PRESET_FULL_RECT)
	_glyph.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_glyph.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_glyph.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_glyph)


func setup(display_name: String, glyph: String, tint: Color, show_name := true, art_path := "") -> void:
	add_theme_stylebox_override("panel", _panel_style(tint))
	_glyph.text = glyph
	var glyph_color := tint.lightened(0.5)
	glyph_color.a = GLYPH_ALPHA
	_glyph.add_theme_color_override("font_color", glyph_color)
	if show_name:
		if _name_label == null:
			_build_name_label()
		_name_label.text = display_name
	elif _name_label != null:
		_name_label.queue_free()
		_name_label = null
	_apply_art(art_path)


## 有贴图则叠加并隐藏首字，始终置于最底层以免遮挡名字条；无贴图则回退首字。
func _apply_art(art_path: String) -> void:
	var existing := find_child("PortraitArt", true, false)
	if existing != null:
		remove_child(existing)
		existing.queue_free()
	var has_art := false
	if art_path != "" and ResourceLoader.exists(art_path):
		var texture := ResourceLoader.load(art_path) as Texture2D
		if texture != null:
			var art := TextureRect.new()
			art.name = "PortraitArt"
			art.texture = texture
			art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
			art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
			art.set_anchors_preset(Control.PRESET_FULL_RECT)
			art.mouse_filter = Control.MOUSE_FILTER_IGNORE
			add_child(art)
			move_child(art, 0)
			has_art = true
	_glyph.visible = not has_art


func _build_name_label() -> void:
	_name_label = UI.label("", UITheme.font_size("font-caption"), false)
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
	box.border_color = Color(0, 0, 0, 0)
	box.set_content_margin_all(0)
	_name_label.add_theme_stylebox_override("normal", box)
	_name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_name_label)


func _panel_style(tint: Color) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = tint.darkened(0.55)
	box.border_color = tint
	box.set_border_width_all(2)
	box.set_corner_radius_all(UITheme.radius("radius-md"))
	box.set_content_margin_all(0)
	return box
