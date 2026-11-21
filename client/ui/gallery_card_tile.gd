class_name GalleryCardTile
extends SelectableBox
## 图鉴网格卡：复用对局卡面，点击选中并交给常驻详情。
##
## 只负责展示；悬停与选中描边由 `SelectableBox` 提供，卡牌数据与筛选留在页面层。
## 手机触屏上按下后拖动是滚动网格，不算点击，因此选中在松手时判定。

signal card_selected(card_id: String)

var card_id := ""


func _init(id: String, selected := false, hero := false) -> void:
    card_id = id
    name = "GalleryCard_%s" % id
    custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
    # PASS：按下继续冒泡到滚动区，ScrollContainer 才能起手触屏拖拽。
    mouse_filter = Control.MOUSE_FILTER_PASS
    mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
    tooltip_text = CardDB.display_name(id)

    configure(UITheme.panel_style("raised-flush"), SelectableBox.Outline.INLINE)
    if hero:
        content().add_child(GalleryCardFace.build_hero(id, CardDB.hero(id)))
    else:
        content().add_child(GalleryCardFace.build(id, CardDB.card(id)))
    activated.connect(_on_activated)
    set_selected(selected)


func _on_activated() -> void:
    card_selected.emit(card_id)
