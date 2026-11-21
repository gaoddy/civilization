extends Node
## 启动时把窗口调整到当前屏幕可用区域的 90%；宽度不小于高度按基准比例换算的下限。
## 拖动缩放时同样守住该下限；全屏、最大化与触控端不介入。

const SCREEN_RATIO := 0.9

var _adjusting := false
var _last_valid := Vector2i.ZERO


func _ready() -> void:
    if DisplayServer.get_name() == "headless":
        # 无头窗口默认 64×64；固定为逻辑基准尺寸，保证冒烟按 16:9 基准复现。
        get_window().size = ScreenProfile.base_canvas()
        return
    if ScreenProfile.is_touch():
        return
    var usable := _usable_rect()
    if usable.size.x <= 0 or usable.size.y <= 0:
        return
    var target := Vector2i(
        int(usable.size.x * SCREEN_RATIO),
        int(usable.size.y * SCREEN_RATIO)
    )
    target = ScreenProfile.fit_aspect_floor(target, usable.size, ScreenProfile.base_canvas())
    var window := get_window()
    window.size = target
    window.position = usable.position + (usable.size - target) / 2
    _last_valid = target
    window.size_changed.connect(_keep_aspect_floor)


## 窗口比基准比例更窄时：屏幕放得下就加宽回下限，放不下则回到上一次合法尺寸。
func _keep_aspect_floor() -> void:
    if _adjusting or ScreenProfile.is_touch():
        return
    var window := get_window()
    if window.mode != Window.MODE_WINDOWED:
        return
    var size := window.size
    var fitted := ScreenProfile.fit_aspect_floor(
        size, _usable_rect().size, ScreenProfile.base_canvas()
    )
    if fitted == size:
        _last_valid = size
        return
    if fitted.x == size.x and fitted.y < size.y:
        fitted = _last_valid
    if fitted.x <= 0 or fitted == size:
        return
    _last_valid = fitted
    _adjusting = true
    window.size = fitted
    _adjusting = false


func _usable_rect() -> Rect2i:
    return DisplayServer.screen_get_usable_rect(DisplayServer.window_get_current_screen())
