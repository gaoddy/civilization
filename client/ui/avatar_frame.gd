class_name AvatarFrame
extends Control
## 圆角头像框：头像图按 cover 铺满并裁到框的圆角，描边叠在头像之上。
##
## 框只负责头像、圆角描边与首字回退；底色与描边色由调用方按场景给出。
## 业务内容挂到 `content()`，位于描边之上，不参与头像裁切。
## 无头像素材时回退为首字；尺寸由调用方 `custom_minimum_size` 决定。

## 描边宽度。
const BORDER_WIDTH := 2
## 首字回退透明度。
const GLYPH_ALPHA := 0.55

var _clip: Panel
var _art: TextureRect = null
var _art_path := ""
var _glyph: Label
var _clip_content: Control
var _border: Panel
var _content: Control
var _cover_color := Color.TRANSPARENT
var _border_color := Color.WHITE


func _init() -> void:
	name = "AvatarFrame"

	_clip = Panel.new()
	_clip.name = "AvatarFrameClip"
	_clip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_clip.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	# 面板绘制形状是子节点的裁切遮罩，同时自身要绘制玩法色封面：头像四角落在圆角以内。
	_clip.clip_children = CanvasItem.CLIP_CHILDREN_AND_DRAW
	add_child(_clip)

	_glyph = UIControls.label("?", UITheme.font_size("font-hero"), false)
	_glyph.name = "AvatarFrameGlyph"
	_glyph.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_glyph.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_glyph.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_glyph.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_clip.add_child(_glyph)

	_clip_content = Control.new()
	_clip_content.name = "AvatarFrameClippedContent"
	_clip_content.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_clip_content.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_clip.add_child(_clip_content)

	_border = Panel.new()
	_border.name = "AvatarFrameBorder"
	_border.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_border.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(_border)

	_content = Control.new()
	_content.name = "AvatarFrameContent"
	_content.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_content.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(_content)

	_apply_styles()


## 设置封面底色与描边色；圆角与描边宽度取组件固定规则。
func configure(cover_color: Color, border_color: Color) -> void:
	_cover_color = cover_color
	_border_color = border_color
	_apply_styles()


## 换头像图：有素材时按 cover 铺满并隐藏首字，路径为空或素材缺失时回退首字。
func set_art(path: String) -> void:
	if path == _art_path and (_art != null or path == ""):
		_glyph.visible = _art == null
		return
	_art_path = path
	if _art != null:
		_clip.remove_child(_art)
		_art.queue_free()
		_art = null
	var art := UIArt.layer(path, "AvatarFrameArt", TextureRect.STRETCH_KEEP_ASPECT_COVERED)
	if art != null:
		art.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		_clip.add_child(art)
		_clip.move_child(art, 0)
		_art = art
	_glyph.visible = art == null


## 首字与颜色：无头像素材时显示。
func set_glyph(text: String, color: Color) -> void:
	_glyph.text = text
	_glyph.add_theme_color_override("font_color", color)


## 业务内容叠层：位于描边之上，不参与头像裁切。
func content() -> Control:
	return _content


## 裁切内内容叠层：位于头像之上、圆角描边之下，随头像一起裁到圆角。
## 满宽贴底的名字条等需要保留圆角的内容挂这里。
func clipped_content() -> Control:
	return _clip_content


func _apply_styles() -> void:
	var cover := StyleBoxFlat.new()
	cover.bg_color = _cover_color
	cover.border_color = _border_color
	cover.set_border_width_all(0)
	cover.set_corner_radius_all(UITheme.radius("radius-md"))
	cover.set_content_margin_all(0)
	_clip.add_theme_stylebox_override("panel", cover)

	var ring := StyleBoxFlat.new()
	ring.bg_color = UITheme.color("color-transparent")
	ring.draw_center = false
	ring.border_color = _border_color
	ring.set_border_width_all(BORDER_WIDTH)
	ring.set_corner_radius_all(UITheme.radius("radius-md"))
	ring.set_content_margin_all(0)
	_border.add_theme_stylebox_override("panel", ring)
