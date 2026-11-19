extends Node
## 启动时把窗口调整到当前屏幕可用区域的 90%，逻辑视口仍为 1280×720。
## 触控端窗口由系统托管，不参与调整。

const SCREEN_RATIO := 0.9


func _ready() -> void:
    if DisplayServer.get_name() == "headless":
        return
    if ScreenProfile.is_touch():
        return
    var usable := DisplayServer.screen_get_usable_rect(
        DisplayServer.window_get_current_screen()
    )
    if usable.size.x <= 0 or usable.size.y <= 0:
        return
    var target := Vector2i(
        int(usable.size.x * SCREEN_RATIO),
        int(usable.size.y * SCREEN_RATIO)
    )
    var window := get_window()
    window.size = target
    window.position = usable.position + (usable.size - target) / 2
