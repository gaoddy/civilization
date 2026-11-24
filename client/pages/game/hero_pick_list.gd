class_name HeroPickList
extends HBoxContainer
## 本人候选英雄卡：单行居中；负责卡片创建、选中与悬停态同步。
##
## 详情挂载与触点清理由覆盖层负责：重建会丢弃旧卡引用，调用方需先重置详情层。
## 手机档位在这里把卡宽收到桌面宽度的 0.78 倍并收紧间距，不改共享令牌。

## 手机候选卡宽度相对桌面档位的倍率。
const PHONE_WIDTH_FACTOR := 0.78

var _rendered_ids: Array[String] = []


func _init() -> void:
	name = "HeroPickCandidates"
	alignment = BoxContainer.ALIGNMENT_CENTER
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_theme_constant_override(
		"separation",
		UITheme.space("space-xs") if ScreenProfile.is_touch() else UITheme.space("space-md"),
	)


## 候选是否与当前卡片不一致：不一致时需重建，否则只需同步状态。
func changed(ids: Array[String]) -> bool:
	return ids != _rendered_ids or get_child_count() != ids.size()


## 重建全部候选卡；调用方之后按 `cards()` 重新挂详情。
func rebuild(ids: Array[String], selected: String, locked: bool) -> void:
	UIControls.clear(self)
	var width := _card_width()
	for id in ids:
		var card := HeroPickCard.new()
		card.display_width = width
		card.setup(id, id == selected, locked)
		add_child(card)
	_rendered_ids = ids.duplicate()


## 候选未变化时只更新选中与锁定，不重建卡片：重建会丢掉正在悬停的详情。
func apply_state(selected: String, locked: bool) -> void:
	for card in cards():
		card.set_selected(card.hero_id == selected)
		card.set_locked(locked)


## 指针所在卡片显示悬停描边：重建后新卡收不到 `mouse_entered`，由页面按指针位置统一同步。
func sync_hover(point: Vector2) -> void:
	for card in cards():
		card.set_hovered(card.get_global_rect().has_point(point))


func cards() -> Array[HeroPickCard]:
	var result: Array[HeroPickCard] = []
	for child in get_children():
		var card := child as HeroPickCard
		if card != null:
			result.append(card)
	return result


func card_at(point: Vector2) -> HeroPickCard:
	for card in cards():
		if card.is_visible_in_tree() and card.get_global_rect().has_point(point):
			return card
	return null


func find_card(hero_id: String) -> HeroPickCard:
	for card in cards():
		if card.hero_id == hero_id:
			return card
	return null


## 手机档位按桌面宽度的 0.78 倍收紧卡宽；桌面沿用舞台令牌。
func _card_width() -> int:
	if ScreenProfile.is_touch():
		return roundi(float(UITheme.metric("hero-detail-width")) * PHONE_WIDTH_FACTOR)
	return UITheme.stage_metric("hero-detail-width")
