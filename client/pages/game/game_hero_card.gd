class_name GameHeroCard
extends PanelContainer
## 英雄卡：64×84，玩法色封面、名字行与底部「时代 · 生命」两格。
##
## 英雄卡只承载身份与技能，不显示攻击力；生命取服务端视图的当前值与上限，
## 时代取玩家视图的当前时代。

const COVER_MARK_ALPHA := 0.22

var hero_id := ""
var hp := 0
var hp_cap := 0
var era := 0

var _cover: PanelContainer
var _cover_mark: Label
var _cover_art: TextureRect = null
var _cover_art_path := ""
var _name_label: Label
var _era_label: Label
var _hp_label: Label


func _init() -> void:
	name = "HeroCard"
	custom_minimum_size = Vector2(
		UITheme.stage_metric("hero-card-width"), UITheme.stage_metric("hero-card-height")
	)
	mouse_filter = Control.MOUSE_FILTER_STOP
	add_theme_stylebox_override("panel", _panel_style())
	var lines := VBoxContainer.new()
	lines.name = "HeroCardLines"
	lines.mouse_filter = Control.MOUSE_FILTER_IGNORE
	lines.add_theme_constant_override("separation", 0)
	add_child(lines)

	_cover = PanelContainer.new()
	_cover.name = "HeroCardCover"
	_cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_cover.custom_minimum_size = Vector2(0, UITheme.stage_metric("hero-cover-height"))
	_cover.add_theme_stylebox_override("panel", _cover_style(UITheme.color("color-surface-sunken")))
	lines.add_child(_cover)

	_cover_mark = UI.label("?", UITheme.font_size("font-hero"), false)
	_cover_mark.name = "HeroCardMark"
	_cover_mark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_cover_mark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_cover_mark.add_theme_color_override("font_color", UITheme.alpha("color-text", COVER_MARK_ALPHA))
	_cover_mark.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_cover.add_child(_cover_mark)

	_name_label = UI.label("", UITheme.font_size("font-micro"), false)
	_name_label.name = "HeroCardName"
	_name_label.custom_minimum_size = Vector2(0, UITheme.stage_metric("hero-name-height"))
	_name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_name_label.max_lines_visible = 1
	_name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	_name_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
	_name_label.add_theme_stylebox_override("normal", _name_style())
	_name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	lines.add_child(_name_label)

	var stats := HBoxContainer.new()
	stats.name = "HeroCardStats"
	stats.mouse_filter = Control.MOUSE_FILTER_IGNORE
	stats.custom_minimum_size = Vector2(0, UITheme.stage_metric("hero-stats-height"))
	stats.alignment = BoxContainer.ALIGNMENT_CENTER
	stats.add_theme_constant_override("separation", UITheme.space("space-2xs"))
	lines.add_child(stats)

	_era_label = _stat_label("HeroEra", Vector2(24, 16), Color(0, 0, 0, 0))
	_era_label.add_theme_color_override("font_color", UITheme.color("color-info"))
	stats.add_child(_era_label)
	_hp_label = _stat_label("HeroHp", Vector2(34, 16), UITheme.alpha("color-success", 0.55))
	stats.add_child(_hp_label)

	refresh("", 0, 0, 0)


func refresh(hero_id: String, hp: int, hp_cap: int, era: int) -> void:
	self.hero_id = hero_id
	self.hp = hp
	self.hp_cap = hp_cap
	self.era = era
	var entry := CardDB.hero(hero_id)
	var playstyle := CardDB.primary_playstyle(entry) if hero_id != "" else ""
	add_theme_stylebox_override("panel", _panel_style(playstyle))
	_cover.add_theme_stylebox_override(
		"panel", _cover_style(UITheme.playstyle_cover(playstyle) if hero_id != "" else UITheme.color("color-surface-sunken"))
	)
	_refresh_cover_art(hero_id)
	_cover_mark.text = CardText.initial(hero_id) if hero_id != "" else "?"
	_name_label.text = CardDB.display_name(hero_id) if hero_id != "" else TranslationServer.translate("ui.common.no_hero")
	_era_label.text = CardDB.era_name(era) if era > 0 else "—"
	_hp_label.text = ("%d/%d" % [hp, hp_cap]) if hp_cap > 0 else "—"
	tooltip_text = _name_label.text


## 封面头像：有素材时叠图并隐藏首字；方形头像四角向内收，落在封面圆角以内。
func _refresh_cover_art(hero_id: String) -> void:
	var path := CardDB.art_path(hero_id, "avatar") if hero_id != "" else ""
	if path == _cover_art_path and (_cover_art != null or path == ""):
		_cover_mark.visible = _cover_art == null
		return
	_cover_art_path = path
	if _cover_art != null:
		_cover.remove_child(_cover_art)
		_cover_art.queue_free()
		_cover_art = null
	var art := UI.art_layer(path, "HeroCoverArt")
	if art != null:
		var inset := UITheme.metric("card-cover-inset")
		art.offset_left = inset
		art.offset_top = inset
		art.offset_right = -inset
		art.offset_bottom = -inset
		_cover.add_child(art)
		_cover.move_child(art, 0)
		_cover_art = art
	_cover_mark.visible = art == null


func _stat_label(node_name: String, size: Vector2, border: Color) -> Label:
	var node := UI.label("", UITheme.font_size("font-micro"), false)
	node.name = node_name
	node.custom_minimum_size = size
	node.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	node.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	node.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	node.add_theme_color_override("font_color", UITheme.color("color-text"))
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.color("color-bg")
	box.border_color = border
	box.set_border_width_all(1)
	box.set_corner_radius_all(UITheme.radius("radius-sm"))
	box.set_content_margin_all(0)
	node.add_theme_stylebox_override("normal", box)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return node


func _panel_style(playstyle := "") -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.color("color-surface-raised")
	box.border_color = (
		UITheme.playstyle_color(playstyle)
		if playstyle != ""
		else UITheme.color("color-border-strong")
	)
	box.set_border_width_all(2)
	box.set_corner_radius_all(UITheme.radius("radius-md"))
	box.set_content_margin_all(0)
	return box


func _cover_style(background: Color) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = background
	box.border_color = UITheme.color("color-border")
	box.border_width_bottom = 1
	box.corner_radius_top_left = UITheme.radius("radius-md")
	box.corner_radius_top_right = UITheme.radius("radius-md")
	box.set_content_margin_all(0)
	return box


func _name_style() -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.alpha("color-surface-raised", 0.92)
	box.border_color = Color(0, 0, 0, 0)
	box.set_content_margin_all(0)
	return box
