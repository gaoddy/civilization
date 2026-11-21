extends Node
## 设备换算夹具：用固定窗口、缩放与安全区断言换算结果，不依赖真实设备。

var _failures := 0


func _ready() -> void:
    _check_default_url()
    _check_prefer_ipv4_url()
    _check_stretch()
    _check_content_frame()
    _check_launch_window()
    _check_parse_window_size()
    _check_parse_device()
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


## 主机名先解析成 IPv4 再连接：双栈解析返回的第一条常是 IPv6，服务端只监听 IPv4。
func _check_prefer_ipv4_url() -> void:
    var resolve := func(host: String) -> String:
        return "192.168.31.3" if host == "cooter.local" else ""
    _check(
        Net.prefer_ipv4_url("ws://cooter.local:8080/ws", resolve)
        == "ws://192.168.31.3:8080/ws",
        "主机名解析为 IPv4 后连接",
    )
    _check(
        Net.prefer_ipv4_url("ws://unresolvable.local:8080/ws", resolve)
        == "ws://unresolvable.local:8080/ws",
        "解析失败时保留原地址",
    )
    _check(
        Net.prefer_ipv4_url("ws://192.168.31.3:8080/ws", func(_host: String) -> String: return "")
        == "ws://192.168.31.3:8080/ws",
        "IP 字面量不做解析",
    )
    _check(
        Net.prefer_ipv4_url("ws://cooter.local/ws", resolve) == "ws://192.168.31.3/ws",
        "省略端口时保留路径",
    )
    _check(
        Net.prefer_ipv4_url("ws://[::1]:8080/ws", resolve) == "ws://[::1]:8080/ws",
        "IPv6 字面量原样保留",
    )


## 窗口缩放策略：expand 让逻辑视口铺满整个窗口，比例不匹配时由页面背景填满背景带；
## 可见内容的比例与留白由 `ContentFrame` 的 contain 计算保证，不由此项拉伸。
func _check_stretch() -> void:
    _check(
        str(ProjectSettings.get_setting("display/window/stretch/aspect")) == "expand",
        "窗口缩放应为 expand",
    )


## 内容框：视口更宽时左右留白，更窄时上下留白，比例恒为基准比例。
func _check_content_frame() -> void:
    var base := ScreenProfile.BASE_CANVAS_TABLET
    var full: Rect2 = ScreenProfile.content_frame(Vector2(1280, 720), base)
    _check(full.is_equal_approx(Rect2(0, 0, 1280, 720)), "16:9 视口内容框应铺满")
    var wide: Rect2 = ScreenProfile.content_frame(Vector2(1280, 480), base)
    _check(is_equal_approx(wide.size.y, 480), "超宽视口内容框高应等于视口高")
    _check(is_equal_approx(wide.size.x, 480.0 * 16.0 / 9.0), "超宽视口内容框宽按比例")
    _check(is_equal_approx(wide.position.x, (1280.0 - wide.size.x) * 0.5), "超宽视口内容框应水平居中")
    var tall: Rect2 = ScreenProfile.content_frame(Vector2(1280, 960), base)
    _check(is_equal_approx(tall.size.x, 1280), "窄视口内容框宽应等于视口宽")
    _check(is_equal_approx(tall.size.y, 720), "窄视口内容框高按比例")
    _check(is_equal_approx(tall.position.y, 120), "窄视口内容框应垂直居中")


## 启动窗口：可用区域的 90% 收拢成 16:9。
func _check_launch_window() -> void:
    var base := ScreenProfile.BASE_CANVAS_TABLET
    _check(ScreenProfile.launch_window_size(Vector2i(1920, 1080), base) == Vector2i(1728, 972),
        "16:9 屏幕启动窗口保持 16:9")
    _check(ScreenProfile.launch_window_size(Vector2i(2560, 1600), base) == Vector2i(2304, 1296),
        "16:10 屏幕启动窗口按宽收拢")
    _check(ScreenProfile.launch_window_size(Vector2i(3440, 1440), base) == Vector2i(2304, 1296),
        "21:9 屏幕启动窗口按高收拢")
    _check(ScreenProfile.launch_window_size(Vector2i(1512, 982), base) == Vector2i(1360, 765),
        "真实 16:10 屏幕启动窗口收拢为 16:9")
    _check(ScreenProfile.launch_window_size(Vector2i.ZERO, base) == base,
        "可用区域未知时回退基准尺寸")
    _check(ScreenProfile.launch_window_size(Vector2i(1, 1), base) == base,
        "可用区域过小、取整为零时回退基准尺寸")


## 显式窗口尺寸：只接受 `<宽>x<高>` 的正整数，其余一律视为未指定。
func _check_parse_window_size() -> void:
    _check(ScreenProfile.parse_window_size("960x540") == Vector2i(960, 540), "解析显式窗口尺寸")
    _check(ScreenProfile.parse_window_size(" 1280x720 ") == Vector2i(1280, 720), "忽略两端空白")
    for text in ["", "960", "960x", "x540", "0x540", "960x0", "960x540x1", "宽x高"]:
        _check(ScreenProfile.parse_window_size(text) == Vector2i.ZERO, "非法尺寸回退默认：" + text)


## 设备档位覆盖值：只接受 phone / tablet / desktop，忽略大小写与空白。
func _check_parse_device() -> void:
    _check(ScreenProfile.parse_device("phone") == "phone", "解析 phone 档位")
    _check(ScreenProfile.parse_device(" Tablet ") == "tablet", "解析 tablet 档位并忽略空白与大小写")
    _check(ScreenProfile.parse_device("DESKTOP") == "desktop", "解析 desktop 档位")
    for text in ["", "watch", "ipad", "phone2"]:
        _check(ScreenProfile.parse_device(text) == "", "非法档位视为未指定：" + text)


func _check_desktop() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_DESKTOP)
    _check(ScreenProfile.base_canvas() == Vector2i(1280, 720), "桌面基准 1280×720")
    _check(not ScreenProfile.is_touch(), "桌面非触控")
    _check(not ScreenProfile.is_simulated_touch(), "桌面非模拟档位")
    _check(ScreenProfile.hit_min(44) == 44, "桌面命中尺寸不换算")
    _check(ScreenProfile.safe_insets() == Vector4i.ZERO, "桌面无安全区")
    _check(ScreenProfile.keyboard_inset() == 0, "桌面无键盘边距")


func _check_phone() -> void:
    var window := Vector2(2556, 1179)
    var base := ScreenProfile.BASE_CANVAS_PHONE
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    _check(ScreenProfile.base_canvas() == Vector2i(960, 540), "iPhone 基准 960×540")
    _check(ScreenProfile.is_touch(), "iPhone 为触控")
    _check(ScreenProfile.is_simulated_touch(), "非真机触控平台为模拟档位")
    # 非真机触控平台（headless 夹具与桌面模拟档位）不读系统屏幕安全区与虚拟键盘。
    _check(ScreenProfile.safe_insets() == Vector4i.ZERO, "非真机触控平台安全区为零")
    _check(ScreenProfile.keyboard_inset() == 0, "非真机触控平台无键盘边距")
    _check(is_equal_approx(ScreenProfile.canvas_scale(window, base), 2.183333), "iPhone 画布缩放")
    _check(ScreenProfile.canvas_hit_min(44, 3.0, base, window) == 60, "iPhone 44pt 换算")
    _check(ScreenProfile.canvas_hit_min(32, 3.0, base, window) == 44, "iPhone 32pt 换算")
    # 刘海在左右留白内、Home 指示条压在内容底部：只有底边生效。
    var insets := ScreenProfile.canvas_insets(
        window, base, Rect2i(177, 0, 2202, 1116), Vector2.ZERO
    )
    _check(insets == Vector4i(0, 0, 0, 29), "iPhone 安全区只在底部生效")
    _check(ScreenProfile.canvas_from_pixels(336.0, base, window) == 154, "iPhone 键盘边距")
    # 19.5:9 内容框贴满窗口高度，没有底部背景带，键盘高度全额换算。
    _check(ScreenProfile.keyboard_from_pixels(336.0, base, window) == 154, "iPhone 键盘无背景带抵扣")


func _check_tablet() -> void:
    var window := Vector2(2732, 2048)
    var base := ScreenProfile.BASE_CANVAS_TABLET
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_TABLET)
    _check(ScreenProfile.base_canvas() == Vector2i(1280, 720), "iPad 基准 1280×720")
    _check(ScreenProfile.is_touch(), "iPad 为触控")
    _check(is_equal_approx(ScreenProfile.canvas_scale(window, base), 2.134375), "iPad 画布缩放")
    _check(ScreenProfile.canvas_hit_min(44, 2.0, base, window) == 41, "iPad 44pt 换算")
    _check(ScreenProfile.canvas_hit_min(32, 2.0, base, window) == 30, "iPad 32pt 换算")
    # 4:3 视口：内容上下各留 256px 背景，圆角只影响左右。
    var insets := ScreenProfile.canvas_insets(
        window, base, Rect2i(12, 0, 2708, 2048), Vector2.ZERO
    )
    _check(insets == Vector4i(6, 0, 6, 0), "iPad 左右安全区生效")
    # 上下内缩的安全区整体落在背景带内，不再产生边距。
    var vertical := ScreenProfile.canvas_insets(
        window, base, Rect2i(12, 24, 2708, 2000), Vector2.ZERO
    )
    _check(vertical == Vector4i(6, 0, 6, 0), "iPad 上下安全区被背景带吸收")
    # 4:3 视口内容框上下各留约 256px 背景带；键盘先从背景带算起。
    _check(ScreenProfile.keyboard_from_pixels(336.0, base, window) == 38, "iPad 键盘抵扣背景带")
    _check(ScreenProfile.keyboard_from_pixels(200.0, base, window) == 0, "键盘未越过背景带时为零")


func _check(ok: bool, label: String) -> void:
    if not ok:
        _failures += 1
        push_error("PLATFORM_SMOKE_FAIL: " + label)
