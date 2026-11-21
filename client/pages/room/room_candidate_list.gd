class_name RoomCandidateList
extends HBoxContainer
## 本人候选英雄卡：单行居中；负责卡片创建、选中与悬停态同步。
##
## 详情挂载与触点清理仍由页面负责：重建会丢弃旧卡引用，调用方需先重置详情层。

var _rendered_ids: Array[String] = []


func _init() -> void:
    name = "HeroCandidates"
    alignment = BoxContainer.ALIGNMENT_CENTER
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_theme_constant_override("separation", UITheme.space("space-md"))


## 候选是否与当前卡片不一致：不一致时需重建，否则只需同步状态。
func changed(ids: Array[String]) -> bool:
    return ids != _rendered_ids or get_child_count() != ids.size()


## 重建全部候选卡；调用方之后按 `cards()` 重新挂详情。
func rebuild(ids: Array[String], selected: String, locked: bool) -> void:
    UIControls.clear(self)
    for id in ids:
        var card := RoomHeroCard.new()
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


## 候选为空时显示分配中占位；返回 true 表示本次从有卡变为占位。
func show_empty() -> bool:
    var swapped := not _rendered_ids.is_empty() or get_child_count() == 0
    if swapped:
        UIControls.clear(self)
        add_child(UIControls.empty_hint(tr("ui.room.assigning_hero")))
        _rendered_ids = []
    return swapped


func cards() -> Array[RoomHeroCard]:
    var result: Array[RoomHeroCard] = []
    for child in get_children():
        var card := child as RoomHeroCard
        if card != null:
            result.append(card)
    return result


func card_at(point: Vector2) -> RoomHeroCard:
    for card in cards():
        if card.is_visible_in_tree() and card.get_global_rect().has_point(point):
            return card
    return null


func find_card(hero_id: String) -> RoomHeroCard:
    for card in cards():
        if card.hero_id == hero_id:
            return card
    return null
