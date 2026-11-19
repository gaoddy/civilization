class_name SafeAreaMargin
extends MarginContainer
## 页面安全区包裹：把内容限制在安全区内，触控端再让出虚拟键盘高度。
##
## 桌面端四边恒为 0，不引入任何额外内边距；触控端在视口尺寸变化时重算。
## 键盘边距只在页面内的 `LineEdit` 获得焦点期间逐帧读取，失焦后回到安全区值。

var _safe := Vector4i.ZERO
var _keyboard_watch := false
var _test_mode := false


func _ready() -> void:
    # 包裹层只负责排版，不拦截页面空白处的输入。
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    var viewport := get_viewport()
    if viewport != null:
        viewport.size_changed.connect(_on_viewport_resized)
    _recalculate()


## 测试入口：直接写入边距，不读取真实设备；底部取安全区与键盘的较大值。
func apply_for_test(insets: Vector4i, keyboard: int) -> void:
    _test_mode = true
    set_process(false)
    _apply(insets, keyboard)


func _process(_delta: float) -> void:
    if _test_mode or not ScreenProfile.is_touch():
        return
    var focus := get_viewport().gui_get_focus_owner()
    var editing := focus is LineEdit and is_ancestor_of(focus)
    if editing:
        _keyboard_watch = true
        _apply(_safe, ScreenProfile.keyboard_inset())
    elif _keyboard_watch:
        _keyboard_watch = false
        _apply(_safe, 0)


func _on_viewport_resized() -> void:
    _recalculate()


func _recalculate() -> void:
    if _test_mode:
        return
    if not ScreenProfile.is_touch():
        _safe = Vector4i.ZERO
        _keyboard_watch = false
        set_process(false)
        _apply(_safe, 0)
        return
    _safe = ScreenProfile.safe_insets()
    var focus := get_viewport().gui_get_focus_owner()
    _keyboard_watch = focus is LineEdit and is_ancestor_of(focus)
    set_process(true)
    _apply(_safe, ScreenProfile.keyboard_inset() if _keyboard_watch else 0)


## 底部取安全区与键盘的较大值，不叠加；其余三边直接使用安全区。
func _apply(insets: Vector4i, keyboard: int) -> void:
    var bottom := maxi(insets.w, keyboard)
    add_theme_constant_override("margin_left", maxi(0, insets.x))
    add_theme_constant_override("margin_top", maxi(0, insets.y))
    add_theme_constant_override("margin_right", maxi(0, insets.z))
    add_theme_constant_override("margin_bottom", maxi(0, bottom))
