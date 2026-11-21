class_name RoomHeroCard
extends PanelContainer
## 选英雄页候选卡：只负责呈现，选择由页面统一处理。
##
## 选中态用外扩的金色边框表达，不额外加徽标；锁定只压暗卡面内容，
## 选中框保持全亮，悬停详情仍可查看。

var _card_size := Vector2.ZERO

var hero_id := ""
var selected := false
var locked := false

var _stack: Control
var _frame: Panel
var _normal_style: StyleBoxFlat
var _hover_style: StyleBoxFlat
var _built := false
var _bound := false


func setup(id: String, is_selected: bool, is_locked: bool) -> void:
	hero_id = id
	name = "HeroCard_%s" % id
	# 候选卡按卡面比例：宽度取详情宽度令牌，高度按 112:146 等比。
	var width := UITheme.metric("hero-detail-width")
	_card_size = Vector2(width, UITheme.card_face_height(width))
	custom_minimum_size = _card_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build()
	set_selected(is_selected)
	set_locked(is_locked)
	if not ScreenProfile.is_touch() and not _bound:
		_bound = true
		mouse_entered.connect(_on_mouse_entered)
		mouse_exited.connect(_on_mouse_exited)


func set_selected(value: bool) -> void:
	selected = value
	if _frame != null:
		_frame.visible = value


func set_locked(value: bool) -> void:
	locked = value
	if _stack == null:
		return
	var alpha := UITheme.opacity("opacity-dim") if value else 1.0
	for child in _stack.get_children():
		if child == _frame:
			continue
		var item := child as CanvasItem
		if item != null:
			item.modulate = Color(1, 1, 1, alpha)


func _build() -> void:
	if _built:
		return
	_built = true

	_normal_style = UITheme.panel_style("raised")
	# 封面要铺满整张卡：面板样式默认带 `space-lg` 内边距，会把封面缩进一圈。
	_normal_style.set_content_margin_all(0)
	_hover_style = _normal_style.duplicate()
	_hover_style.border_color = UITheme.color("color-border-strong")
	add_theme_stylebox_override("panel", _normal_style)

	_stack = Control.new()
	_stack.name = "Stack"
	_stack.custom_minimum_size = _card_size
	_stack.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_stack)

	var entry := CardDB.hero(hero_id)
	var playstyle := CardDB.primary_playstyle(entry)

	var cover := ColorRect.new()
	cover.name = "Cover"
	cover.color = UITheme.playstyle_cover(playstyle)
	cover.set_anchors_preset(Control.PRESET_FULL_RECT)
	cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_stack.add_child(cover)

	# 有卡面时叠图并隐藏首字水印：方形卡面四角向内收，落在卡片圆角以内。
	var cover_art := UI.art_layer(CardDB.art_path(hero_id, "card_face"), "CoverArt")
	if cover_art != null:
		var inset := UITheme.metric("card-cover-inset")
		cover_art.offset_left = inset
		cover_art.offset_top = inset
		cover_art.offset_right = -inset
		cover_art.offset_bottom = -inset
		_stack.add_child(cover_art)
	else:
		var watermark := UI.label(CardDB.display_name(hero_id).substr(0, 1), 72, false)
		watermark.name = "Watermark"
		watermark.set_anchors_preset(Control.PRESET_FULL_RECT)
		watermark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		watermark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		watermark.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
		watermark.mouse_filter = Control.MOUSE_FILTER_IGNORE
		_stack.add_child(watermark)

	var name_label := UI.label(CardDB.display_name(hero_id), UITheme.font_size("font-title"), false)
	name_label.name = "Name"
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_label.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	name_label.offset_top = -UITheme.space("space-2xl")
	name_label.offset_bottom = -UITheme.space("space-md")
	name_label.add_theme_color_override("font_color", UITheme.color("color-text"))
	name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	# 名字叠在卡面上：半透明底保证有图时仍然可读。
	var name_style := StyleBoxFlat.new()
	name_style.bg_color = UITheme.alpha("color-bg", 0.72)
	name_style.set_content_margin_all(0)
	name_label.add_theme_stylebox_override("normal", name_style)
	_stack.add_child(name_label)

	_frame = Panel.new()
	_frame.name = "SelectedFrame"
	_frame.set_anchors_preset(Control.PRESET_FULL_RECT)
	var grow := UITheme.space("space-2xs")
	_frame.offset_left = -grow
	_frame.offset_top = -grow
	_frame.offset_right = grow
	_frame.offset_bottom = grow
	_frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var frame_style := StyleBoxFlat.new()
	frame_style.bg_color = Color(0, 0, 0, 0)
	frame_style.border_color = UITheme.color("color-accent")
	frame_style.set_border_width_all(2)
	frame_style.set_corner_radius_all(UITheme.radius("radius-lg"))
	_frame.add_theme_stylebox_override("panel", frame_style)
	_frame.visible = false
	_stack.add_child(_frame)


func set_hovered(value: bool) -> void:
	if _normal_style == null:
		return
	add_theme_stylebox_override("panel", _hover_style if value else _normal_style)


func _on_mouse_entered() -> void:
	set_hovered(true)


func _on_mouse_exited() -> void:
	set_hovered(false)
