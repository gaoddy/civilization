class_name GalleryCardTile
extends PanelContainer
## 图鉴网格卡：复用对局卡面，点击选中并交给常驻详情。
##
## 只负责展示、悬停与选中描边；卡牌数据与筛选留在页面层。
## 手机触屏上按下后拖动是滚动网格，不算点击，因此选中在松手时判定。

const TAP_SLOP := 12.0

signal card_selected(card_id: String)

var card_id := ""

var _selected := false
var _hovered := false
var _pressed := false
var _pressed_canvas := Vector2.ZERO


func _init(id: String, selected := false, hero := false) -> void:
    card_id = id
    _selected = selected
    name = "GalleryCard_%s" % id
    custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
    # PASS：按下继续冒泡到滚动区，ScrollContainer 才能起手触屏拖拽。
    mouse_filter = Control.MOUSE_FILTER_PASS
    mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
    tooltip_text = CardDB.display_name(id)

    if hero:
        add_child(GalleryCardFace.build_hero(id, CardDB.hero(id)))
    else:
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
        if mouse.button_index != MOUSE_BUTTON_LEFT:
            return
        if mouse.pressed:
            _pressed = true
            _pressed_canvas = _canvas_point(mouse.position)
            return
        if not _pressed:
            return
        _pressed = false
        # 比较画布坐标而不是卡面局部坐标：滚动时卡面会移动，局部坐标会把滚动量算进位移。
        if _canvas_point(mouse.position).distance_to(_pressed_canvas) <= TAP_SLOP:
            card_selected.emit(card_id)


## 卡面局部坐标 → 画布坐标，与视口分发事件时的换算互逆。
func _canvas_point(local: Vector2) -> Vector2:
    return get_global_transform_with_canvas() * local


func _on_hover_changed(value: bool) -> void:
    _hovered = value
    _apply_style()


func _apply_style() -> void:
    var box := UITheme.panel_style("raised-flush")
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
