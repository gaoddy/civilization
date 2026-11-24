class_name HeroPickCard
extends SelectableBox
## 对局页选英雄候选卡：只负责呈现，选择由覆盖层统一处理。
##
## 选中态用外扩的金色边框表达；锁定只压暗卡面内容，选中框保持全亮，
## 悬停详情仍可查看。外扩选中框与悬停描边由 `SelectableBox` 提供。
## 宽度由候选行给出：桌面取 `hero-detail-width`，手机档位按同一令牌缩放。

var _card_size := Vector2.ZERO

var hero_id := ""
var locked := false
## 候选行按设备档位给出的展示宽度；小于等于 0 时按舞台令牌自行计算。
var display_width := -1

var _built := false


func setup(id: String, is_selected: bool, is_locked: bool) -> void:
	hero_id = id
	name = "HeroPickCard_%s" % id
	# 候选卡按卡面比例：宽度取候选行给出的值，高度按 112:146 等比。
	var width := display_width if display_width > 0 else UITheme.stage_metric("hero-detail-width")
	_card_size = Vector2(width, UITheme.card_face_height(width))
	custom_minimum_size = _card_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	_build()
	set_selected(is_selected)
	set_locked(is_locked)


func set_locked(value: bool) -> void:
	locked = value
	if content() == null:
		return
	var alpha := UITheme.opacity("opacity-dim") if value else 1.0
	for child in content().get_children():
		if child.name == "SelectedFrame":
			continue
		var item := child as CanvasItem
		if item != null:
			item.modulate = UITheme.alpha("color-white", alpha)


func _build() -> void:
	if _built:
		return
	_built = true

	var base_style := UITheme.panel_style("raised")
	# 封面要铺满整张卡：面板样式默认带 `space-lg` 内边距，会把封面缩进一圈。
	base_style.set_content_margin_all(0)
	# 候选卡保持直角，外扩选中框也随之走直角，不被铺满的卡面遮住四角。
	base_style.set_corner_radius_all(0)
	configure(base_style, SelectableBox.Outline.OUTER, not ScreenProfile.is_touch())

	var stack := content()
	stack.custom_minimum_size = _card_size

	var entry := CardDB.hero(hero_id)
	var playstyle := CardDB.primary_playstyle(entry)

	var cover := ColorRect.new()
	cover.name = "Cover"
	cover.color = UITheme.playstyle_cover(playstyle)
	cover.set_anchors_preset(Control.PRESET_FULL_RECT)
	cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
	stack.add_child(cover)

	# 有卡面时叠图并隐藏首字水印：方形卡面四角向内收，留出描边位置。
	var cover_art := UIArt.layer(CardDB.art_path(hero_id, "card_face"), "CoverArt")
	if cover_art != null:
		var inset := UITheme.stage_metric("card-cover-inset")
		cover_art.offset_left = inset
		cover_art.offset_top = inset
		cover_art.offset_right = -inset
		cover_art.offset_bottom = -inset
		stack.add_child(cover_art)
	else:
		var watermark := UIControls.label(
			CardDB.display_name(hero_id).substr(0, 1), UITheme.font_size("font-watermark"), false
		)
		watermark.name = "Watermark"
		watermark.set_anchors_preset(Control.PRESET_FULL_RECT)
		watermark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		watermark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		watermark.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
		watermark.mouse_filter = Control.MOUSE_FILTER_IGNORE
		stack.add_child(watermark)

	var name_label := UIControls.label(CardDB.display_name(hero_id), UITheme.font_size("font-title"), false)
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
	stack.add_child(name_label)
