extends Node
## 启动时把窗口按当前屏幕可用区域收拢成基准比例并居中。
## 拖动缩放后不再调整窗口尺寸；全屏、最大化与触控端不介入。
## 环境变量 `CIVILIZATION_WINDOW_SIZE` 指定 `<宽>x<高>` 时改用该尺寸，供本地模拟不同设备。
## 桌面端模拟触控档位时同样按该尺寸开窗；真机触控端不介入窗口尺寸。


func _ready() -> void:
    if DisplayServer.get_name() == "headless":
        # 无头窗口默认 64×64；固定为逻辑基准尺寸，保证冒烟按 16:9 基准复现。
        get_window().size = ScreenProfile.base_canvas()
        return
    var target := ScreenProfile.parse_window_size(
        OS.get_environment("CIVILIZATION_WINDOW_SIZE")
    )
    if ScreenProfile.is_touch() and target == Vector2i.ZERO:
        return
    var usable := _usable_rect()
    if usable.size.x <= 0 or usable.size.y <= 0:
        return
    if target == Vector2i.ZERO:
        target = ScreenProfile.launch_window_size(usable.size, ScreenProfile.base_canvas())
    var window := get_window()
    window.size = target
    window.position = usable.position + (usable.size - target) / 2


func _usable_rect() -> Rect2i:
    return DisplayServer.screen_get_usable_rect(DisplayServer.window_get_current_screen())
