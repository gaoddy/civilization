class_name GameHeroCard
extends AvatarFrame
## 英雄卡：方形头像铺满整卡，右下角只叠当前生命。
##
## 卡只承载身份与当前生命，不显示名字、时代与攻击力；名字只出现在 tooltip 与悬停详情，
## 生命取服务端视图的当前值。无头像素材时回退玩法色封面与名字首字。
## 头像、圆角描边与遮罩由 `AvatarFrame` 提供，生命牌挂在内容叠层。

var hero_id := ""
var hp := 0
var hp_cap := 0

var _hp_plate: PanelContainer
var _hp_label: Label


func _init() -> void:
	super()
	name = "HeroCard"
	var side := UITheme.stage_metric("hero-card-size")
	custom_minimum_size = Vector2(side, side)
	mouse_filter = Control.MOUSE_FILTER_STOP

	_build_hp_plate()
	refresh("", 0, 0)


func refresh(hero_id: String, hp: int, hp_cap: int) -> void:
	self.hero_id = hero_id
	self.hp = hp
	self.hp_cap = hp_cap
	var entry := CardDB.hero(hero_id)
	var playstyle := CardDB.primary_playstyle(entry) if hero_id != "" else ""
	var cover_color := (
		UITheme.playstyle_cover(playstyle)
		if playstyle != ""
		else UITheme.color("color-surface-sunken")
	)
	var border_color := (
		UITheme.playstyle_color(playstyle)
		if playstyle != ""
		else UITheme.color("color-border-strong")
	)
	configure(cover_color, border_color)
	set_art(CardDB.art_path(hero_id, "avatar") if hero_id != "" else "")
	set_glyph(
		CardText.initial(hero_id) if hero_id != "" else "?",
		UITheme.alpha("color-text", GLYPH_ALPHA),
	)
	_hp_label.text = str(hp)
	_hp_plate.visible = hero_id != "" and hp_cap > 0
	tooltip_text = CardDB.display_name(hero_id) if hero_id != "" else ""


## 非英雄头像：售货员等复用英雄卡的尺寸、圆角、描边与首字回退，隐藏生命牌。
## 不调用 `refresh()`，避免封面退回 `color-surface-sunken`；描边取色与
## `GamePortrait.setup()` 保持一致。
func present_portrait(display_name: String, glyph: String, tint: Color, art_path := "") -> void:
	hero_id = ""
	hp = 0
	hp_cap = 0
	var glyph_color := tint.lightened(0.5)
	glyph_color.a = GLYPH_ALPHA
	configure(tint.darkened(0.55), tint)
	set_art(art_path)
	set_glyph(glyph, glyph_color)
	_hp_plate.visible = false
	tooltip_text = display_name


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
	content().add_child(margin)

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


## 生命牌：深色半透明底保证压在头像上仍可读，白字只写当前值。
func _hp_style() -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = UITheme.alpha("color-bg", 0.72)
	box.border_color = UITheme.alpha("color-white", 0.25)
	box.set_border_width_all(1)
	box.set_corner_radius_all(UITheme.radius("radius-sm"))
	box.set_content_margin_all(UITheme.space("space-2xs"))
	return box
