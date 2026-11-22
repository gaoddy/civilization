class_name GalleryDetailLayer
extends Control
## 图鉴详情浮层：工具条以下的遮罩 + 右侧详情抽屉，承载 `GalleryDetail`。
##
## 点遮罩、关闭按钮或由页面按 Esc 收起；图层只管显隐与容器，内容与数据仍在
## `GalleryDetail` 与页面层。手机档位改为贴底的整宽抽屉。

signal closed

var _scrim: ColorRect
var _drawer: PanelContainer
var _detail: GalleryDetail
var _close_button: Button


## `top` 为遮罩与桌面抽屉的顶边，由 `CardGallery` 按头部下沿传入。
func _init(top: int) -> void:
    name = "GalleryDetailLayer"
    set_anchors_preset(Control.PRESET_FULL_RECT)
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    visible = false

    var edge := UITheme.space("space-2xl")

    _scrim = ColorRect.new()
    _scrim.name = "GalleryDetailScrim"
    _scrim.set_anchors_preset(Control.PRESET_FULL_RECT)
    _scrim.offset_top = top
    var scrim_color := UITheme.color("color-bg")
    scrim_color.a = UITheme.opacity("scrim-alpha-gallery")
    _scrim.color = scrim_color
    _scrim.gui_input.connect(_on_scrim_input)
    add_child(_scrim)

    _drawer = PanelContainer.new()
    _drawer.name = "GalleryDrawer"
    _drawer.mouse_filter = Control.MOUSE_FILTER_STOP
    var style := UITheme.panel_style("surface")
    style.set_corner_radius_all(UITheme.radius("radius-lg"))
    _drawer.add_theme_stylebox_override("panel", style)
    add_child(_drawer)

    var phone := ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
    if phone:
        _drawer.anchor_left = 0.0
        _drawer.anchor_right = 1.0
        _drawer.anchor_top = 0.3
        _drawer.anchor_bottom = 1.0
        _drawer.offset_left = UITheme.space("space-xl")
        _drawer.offset_right = -UITheme.space("space-xl")
        _drawer.offset_top = 0
        _drawer.offset_bottom = -UITheme.space("space-xl")
    else:
        _drawer.anchor_left = 1.0
        _drawer.anchor_right = 1.0
        _drawer.anchor_top = 0.0
        _drawer.anchor_bottom = 1.0
        _drawer.offset_left = -(UITheme.stage_metric("gallery-drawer-width") + edge)
        _drawer.offset_right = -edge
        # 顶部对齐头部下沿，底部贴到页面底边，抽屉纵向占满头部以下的整块空间。
        _drawer.offset_top = top
        _drawer.offset_bottom = 0

    var box := VBoxContainer.new()
    box.name = "GalleryDrawerBody"
    box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _drawer.add_child(box)

    var bar := HBoxContainer.new()
    bar.name = "GalleryDrawerBar"
    box.add_child(bar)
    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    bar.add_child(spacer)
    _close_button = UIControls.styled_button("×", "ghost", 24)
    _close_button.name = "GalleryDetailClose"
    _close_button.custom_minimum_size = Vector2(24, 24)
    _close_button.add_theme_font_size_override("font_size", UITheme.font_size("font-body"))
    _close_button.pressed.connect(close)
    bar.add_child(_close_button)

    _detail = GalleryDetail.new()
    _detail.size_flags_vertical = Control.SIZE_EXPAND_FILL
    box.add_child(_detail)


## 显示详情并打开抽屉；`hero` 为真时走英雄口径。
func open_card(card_id: String, hero: bool) -> void:
    if hero:
        _detail.show_hero(card_id)
    else:
        _detail.show_card(card_id)
    visible = true
    _drawer.modulate.a = 0.0
    var tween := create_tween()
    tween.tween_property(_drawer, "modulate:a", 1.0, 0.12)


func close() -> void:
    if not visible:
        return
    visible = false
    closed.emit()


func is_open() -> bool:
    return visible


func _on_scrim_input(event: InputEvent) -> void:
    if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
        close()
        accept_event()
