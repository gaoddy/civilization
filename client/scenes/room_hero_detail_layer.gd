class_name RoomHeroDetailLayer
extends GameDetailLayer
## 选英雄页悬停详情：复用详情层的悬停/长按状态机，只替换内容与定位。
##
## 桌面悬停或触控长按候选卡时，在候选行左右外侧显示立绘、名字、介绍与技能；
## 详情层与全部子控件忽略鼠标，点击仍落到候选卡上。

var row: Control


func _ready() -> void:
	name = "HeroDetailLayer"
	if _panel == null:
		return
	_panel.name = "RoomHeroDetail"
	_panel.custom_minimum_size = Vector2(UITheme.stage_metric("room-hero-detail-width"), 0)


## 重建候选卡后恢复悬停详情；`hero_id` 由页面从旧卡元数据取出。
func show_for(card: Control, hero_id: String) -> void:
	if card == null or hero_id == "":
		return
	_show_detail({"detail_kind": "hero", "heroId": hero_id}, card)


## 当前正在展示的英雄 id；未展示或锚点已释放时返回空串。
func visible_hero_id() -> String:
	if _panel == null or not _panel.visible:
		return ""
	if _hover_card == null or not is_instance_valid(_hover_card):
		return ""
	return str(_hover_card.get_meta("detail_data", {}).get("heroId", ""))


func _detail_text_width() -> int:
	return UITheme.stage_metric("room-hero-detail-width") - UITheme.space("space-lg") * 2


func _render_hero_detail(box: VBoxContainer, hero_id: Variant) -> void:
	var id := "" if hero_id == null else str(hero_id)
	# 详情顶部叠卡面：按 112:146 比例铺满，无图时只显示文字内容。
	var art := UI.art_layer(
		CardDB.art_path(id, "card_face"),
		"HeroPortrait",
		TextureRect.STRETCH_KEEP_ASPECT_COVERED,
	)
	if art != null:
		var width := UITheme.stage_metric("hero-detail-width")
		art.custom_minimum_size = Vector2(width, UITheme.card_face_height(width))
		art.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		box.add_child(art)
	box.add_child(_detail_name(CardDB.display_name(id)))
	var introduction := CardDB.hero_introduction(id)
	if introduction != "":
		var intro := _detail_text_block(introduction)
		intro.name = "HeroIntroduction"
		box.add_child(intro)
	UI.skill_list(box, CardDB.hero(id).get("skills", []))
	_ignore_mouse(box)


## 详情卡按候选行左右外侧挂载：卡片中心在行中心左侧就挂左外侧，否则右外侧。
func _place_detail(card_rect: Rect2, kind: String) -> void:
	if _panel == null:
		return
	if kind != "hero" or row == null or not is_instance_valid(row):
		super._place_detail(card_rect, kind)
		return

	var row_rect := row.get_global_rect()
	# 面板不在容器内，只靠最小尺寸不会在换到更矮的英雄时收缩；定位前按当前内容重设尺寸。
	_panel.reset_size()
	_layout_hero_detail(card_rect, row_rect, _panel.size)
	# 新内容的最小尺寸要到下一帧才结算：再校正一次，避免沿用上一张英雄的高度。
	_correct_panel_size.call_deferred()


## 内容换人后按结算完成的最小尺寸重新收缩并重新定位。
func _correct_panel_size() -> void:
	if _panel == null or not _panel.visible:
		return
	if _hover_card == null or not is_instance_valid(_hover_card) or row == null or not is_instance_valid(row):
		return
	_panel.reset_size()
	_layout_hero_detail(_hover_card.get_global_rect(), row.get_global_rect(), _panel.size)


func _layout_hero_detail(card_rect: Rect2, row_rect: Rect2, panel_size: Vector2) -> void:
	var layer_rect := get_global_rect()
	var margin := UITheme.space("space-xs")
	var gap := UITheme.space("space-sm")
	var side_left := card_rect.get_center().x < row_rect.get_center().x
	var pos := Vector2.ZERO
	if side_left:
		pos.x = row_rect.position.x - panel_size.x - gap
	else:
		pos.x = row_rect.end.x + gap
	pos.y = row_rect.get_center().y - panel_size.y * 0.5

	var min_x := layer_rect.position.x + margin
	var max_x := maxf(min_x, layer_rect.end.x - panel_size.x - margin)
	var min_y := layer_rect.position.y + margin
	var max_y := maxf(min_y, layer_rect.end.y - panel_size.y - margin)
	pos.x = clampf(pos.x, min_x, max_x)
	pos.y = clampf(pos.y, min_y, max_y)

	# 手机安全区夹紧后可能盖住锚点卡：改放另一侧，优先保证长按的卡可见。
	if Rect2(pos, panel_size).intersects(card_rect):
		var alt_x := card_rect.end.x + gap if side_left else card_rect.position.x - panel_size.x - gap
		alt_x = clampf(alt_x, min_x, max_x)
		var alt_pos := Vector2(alt_x, pos.y)
		if not Rect2(alt_pos, panel_size).intersects(card_rect):
			pos = alt_pos
	_panel.set_global_position(pos)


func _ignore_mouse(node: Node) -> void:
	for child in node.get_children():
		if child is Control:
			(child as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
		_ignore_mouse(child)
