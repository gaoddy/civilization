class_name GameHeroCard
extends PanelContainer
## 英雄卡：方形头像铺满整卡，右下角只叠当前生命。
##
## 卡只承载身份与当前生命，不显示名字、时代与攻击力；名字只出现在 tooltip 与悬停详情，
## 生命取服务端视图的当前值。无头像素材时回退玩法色封面与名字首字。

const COVER_MARK_ALPHA := 0.55
const BORDER_WIDTH := 2

var hero_id := ""
var hp := 0
var hp_cap := 0

var _body: Control
var _cover_mark: Label
var _cover_art: TextureRect = null
var _cover_art_path := ""
var _hp_plate: PanelContainer
var _hp_label: Label


func _init() -> void:
	name = "HeroCard"
	var side := UITheme.stage_metric("hero-card-size")
	custom_minimum_size = Vector2(side, side)
	mouse_filter = Control.MOUSE_FILTER_STOP
	add_theme_stylebox_override("panel", _panel_style())

	_body = Control.new()
	_body.name = "HeroCardBody"
	_body.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_body)

	_cover_mark = UIControls.label("?", UITheme.font_size("font-hero"), false)
	_cover_mark.name = "HeroCardMark"
	_cover_mark.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_cover_mark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_cover_mark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_cover_mark.add_theme_color_override("font_color", UITheme.alpha("color-text", COVER_MARK_ALPHA))
	_cover_mark.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_body.add_child(_cover_mark)

	_build_hp_plate()
	refresh("", 0, 0)


func refresh(hero_id: String, hp: int, hp_cap: int) -> void:
	self.hero_id = hero_id
	self.hp = hp
	self.hp_cap = hp_cap
	var entry := CardDB.hero(hero_id)
	var playstyle := CardDB.primary_playstyle(entry) if hero_id != "" else ""
	add_theme_stylebox_override("panel", _panel_style(playstyle))
	_refresh_cover_art(hero_id)
	_cover_mark.text = CardText.initial(hero_id) if hero_id != "" else "?"
	_hp_label.text = str(hp)
	_hp_plate.visible = hero_id != "" and hp_cap > 0
	tooltip_text = CardDB.display_name(hero_id) if hero_id != "" else ""


## 生命牌贴右下角，与英雄行 84 高无关；只显示当前值，上限由悬停详情给出。
func _build_hp_plate() -> void:
	var margin := MarginContainer.new()
	margin.name = "HeroHpMargin"
	margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var inset := UITheme.space("space-2xs")
	margin.add_theme_constant_override("margin_left", inset)
	margin.add_theme_constant_override("margin_top", inset)
	margin.add_theme_constant_override("margin_right", inset)
	margin.add_theme_constant_override("margin_bottom", inset)
	_body.add_child(margin)

	var column := VBoxContainer.new()
	column.name = "HeroHpColumn"
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.alignment = BoxContainer.ALIGNMENT_END
	margin.add_child(column)

	var row := HBoxContainer.new()
	row.name = "HeroHpRow"
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.alignment = BoxContainer.ALIGNMENT_END
	column.add_child(row)

	_hp_plate = PanelContainer.new()
	_hp_plate.name = "HeroHpPlate"
	_hp_plate.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_hp_plate.add_theme_stylebox_override("panel", _hp_style())
	row.add_child(_hp_plate)

	_hp_label = UIControls.label("", UITheme.font_size("font-caption"), false)
	_hp_label.name = "HeroHp"
	_hp_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_hp_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_hp_label.add_theme_color_override("font_color", UITheme.color("color-white"))
	_hp_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_hp_plate.add_child(_hp_label)


## 封面头像：方形素材按 cover 裁切铺满整卡，四边不留玩法色；无素材时回退首字。
func _refresh_cover_art(hero_id: String) -> void:
	var path := CardDB.art_path(hero_id, "avatar") if hero_id != "" else ""
	if path == _cover_art_path and (_cover_art != null or path == ""):
		_cover_mark.visible = _cover_art == null
		return
	_cover_art_path = path
	if _cover_art != null:
		_body.remove_child(_cover_art)
		_cover_art.queue_free()
		_cover_art = null
	var art := UIArt.layer(
		path, "HeroCoverArt", TextureRect.STRETCH_KEEP_ASPECT_COVERED
	)
	if art != null:
		_body.add_child(art)
		_body.move_child(art, 0)
		_cover_art = art
	_cover_mark.visible = art == null


## 卡片面板：玩法色描边包住整块头像，内容边距等于描边宽，头像不被描边遮住。
func _panel_style(playstyle := "") -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = (
		UITheme.playstyle_cover(playstyle)
		if playstyle != ""
		else UITheme.color("color-surface-sunken")
	)
	box.border_color = (
		UITheme.playstyle_color(playstyle)
		if playstyle != ""
		else UITheme.color("color-border-strong")
	)
	box.set_border_width_all(BORDER_WIDTH)
	box.set_corner_radius_all(UITheme.radius("radius-md"))
	box.set_content_margin_all(BORDER_WIDTH)
	return box


## 生命牌：深色半透明底保证压在头像上仍可读，白字只写当前值。
func _hp_style() -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.alpha("color-bg", 0.72)
	box.border_color = UITheme.alpha("color-white", 0.25)
	box.set_border_width_all(1)
	box.set_corner_radius_all(UITheme.radius("radius-sm"))
	box.set_content_margin_all(UITheme.space("space-2xs"))
	return box
