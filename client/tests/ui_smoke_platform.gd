extends Node
## 设备换算夹具：用固定窗口、缩放与安全区断言换算结果，不依赖真实设备。

var _failures := 0


func _ready() -> void:
    _check_default_url()
    _check_stretch()
    _check_desktop()
    _check_phone()
    _check_tablet()
    ScreenProfile.set_device_for_test("")
    if _failures > 0:
        get_tree().quit(1)
        return
    print("UI_SMOKE_OK")
    get_tree().quit(0)


func _check_default_url() -> void:
    _check(Net.default_url_for(false) == "ws://127.0.0.1:8080/ws", "桌面默认调试地址")
    _check(Net.default_url_for(true) == "ws://cooter.local:8080/ws", "移动端默认调试地址")


## 窗口缩放策略：keep_width 保证窗口上下始终由内容铺满。
func _check_stretch() -> void:
    _check(
        str(ProjectSettings.get_setting("display/window/stretch/aspect")) == "keep_width",
        "窗口缩放应为 keep_width",
    )


func _check_desktop() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_DESKTOP)
    _check(ScreenProfile.base_canvas() == Vector2i(1280, 720), "桌面基准 1280×720")
    _check(not ScreenProfile.is_touch(), "桌面非触控")
    _check(ScreenProfile.hit_min(44) == 44, "桌面命中尺寸不换算")
    _check(ScreenProfile.safe_insets() == Vector4i.ZERO, "桌面无安全区")
    _check(ScreenProfile.keyboard_inset() == 0, "桌面无键盘边距")


func _check_phone() -> void:
    var window := Vector2(2556, 1179)
    var base := ScreenProfile.BASE_CANVAS_PHONE
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    _check(ScreenProfile.base_canvas() == Vector2i(960, 540), "iPhone 基准 960×540")
    _check(ScreenProfile.is_touch(), "iPhone 为触控")
    _check(is_equal_approx(ScreenProfile.canvas_scale(window, base), 2.183333), "iPhone 画布缩放")
    _check(ScreenProfile.canvas_hit_min(44, 3.0, base, window) == 60, "iPhone 44pt 换算")
    _check(ScreenProfile.canvas_hit_min(32, 3.0, base, window) == 44, "iPhone 32pt 换算")
    # 刘海在左右留白内、Home 指示条压在内容底部：只有底边生效。
    var insets := ScreenProfile.canvas_insets(
        window, base, Rect2i(177, 0, 2202, 1116), Vector2.ZERO
    )
    _check(insets == Vector4i(0, 0, 0, 29), "iPhone 安全区只在底部生效")
    _check(ScreenProfile.canvas_from_pixels(336.0, base, window) == 154, "iPhone 键盘边距")


func _check_tablet() -> void:
    var window := Vector2(2732, 2048)
    var base := ScreenProfile.BASE_CANVAS_TABLET
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_TABLET)
    _check(ScreenProfile.base_canvas() == Vector2i(1280, 720), "iPad 基准 1280×720")
    _check(ScreenProfile.is_touch(), "iPad 为触控")
    _check(is_equal_approx(ScreenProfile.canvas_scale(window, base), 2.134375), "iPad 画布缩放")
    _check(ScreenProfile.canvas_hit_min(44, 2.0, base, window) == 41, "iPad 44pt 换算")
    _check(ScreenProfile.canvas_hit_min(32, 2.0, base, window) == 30, "iPad 32pt 换算")
    # 4:3 窗口纵向扩展，内容高度铺满窗口；圆角只影响左右。
    var insets := ScreenProfile.canvas_insets(
        window, base, Rect2i(12, 0, 2708, 2048), Vector2.ZERO
    )
    _check(insets == Vector4i(6, 0, 6, 0), "iPad 左右安全区生效")
    # 上下内缩的安全区按窗口边换算，不再被等比留白吞掉。
    var vertical := ScreenProfile.canvas_insets(
        window, base, Rect2i(12, 24, 2708, 2000), Vector2.ZERO
    )
    _check(vertical == Vector4i(6, 11, 6, 11), "iPad 上下安全区按窗口边换算")


func _check(ok: bool, label: String) -> void:
    if not ok:
        _failures += 1
        push_error("PLATFORM_SMOKE_FAIL: " + label)
