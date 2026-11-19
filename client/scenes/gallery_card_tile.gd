class_name GalleryCardTile
extends PanelContainer
## 图鉴网格卡：复用对局卡面，点击选中并交给常驻详情。
##
## 只负责展示、悬停与选中描边；卡牌数据与筛选留在页面层。

signal card_selected(card_id: String)

var card_id := ""

var _selected := false
var _hovered := false


func _init(id: String, selected := false) -> void:
    card_id = id
    _selected = selected
    name = "GalleryCard_%s" % id
    custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
    mouse_filter = Control.MOUSE_FILTER_STOP
    mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
    tooltip_text = CardDB.display_name(id)

    add_child(GalleryCardFace.build(id, CardDB.card(id)))
    _apply_style()

    gui_input.connect(_on_gui_input)
    mouse_entered.connect(_on_hover_changed.bind(true))
    mouse_exited.connect(_on_hover_changed.bind(false))


func set_selected(value: bool) -> void:
    if _selected == value:
        return
    _selected = value
    _apply_style()


func _on_gui_input(event: InputEvent) -> void:
    if event is InputEventMouseButton:
        var mouse := event as InputEventMouseButton
        if mouse.button_index == MOUSE_BUTTON_LEFT and mouse.pressed:
            card_selected.emit(card_id)
            accept_event()


func _on_hover_changed(value: bool) -> void:
    _hovered = value
    _apply_style()


func _apply_style() -> void:
    var box := UITheme.panel_style("raised-tight")
    if _selected:
        box.border_color = UITheme.color("color-accent")
        box.set_border_width_all(2)
    elif _hovered:
        box.border_color = UITheme.color("color-border-strong")
        box.set_border_width_all(1)
    else:
        box.border_color = UITheme.color("color-border")
        box.set_border_width_all(1)
    add_theme_stylebox_override("panel", box)
