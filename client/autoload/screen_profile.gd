extends Node
## 设备与布局能力：设备类型、逻辑基准、逻辑点到画布换算、安全区与键盘边距。
##
## 只服务布局，不含业务规则。桌面端保持 1280 宽基准与零边距，高度随窗口扩展。
## 换算公式是纯函数，冒烟用固定输入直接断言，不依赖真实设备。

const BASE_CANVAS_PHONE := Vector2i(960, 540)
const BASE_CANVAS_TABLET := Vector2i(1280, 720)
const DEVICE_PHONE := "phone"
const DEVICE_TABLET := "tablet"
const DEVICE_DESKTOP := "desktop"

var _device := ""
var _forced := ""


func _ready() -> void:
    _apply_base_canvas()


## 当前设备类型：phone / tablet / desktop。测试可通过 set_device_for_test 覆盖。
func device_kind() -> String:
    if _forced != "":
        return _forced
    if _device == "":
        _device = _detect_device()
    return _device


func is_touch() -> bool:
    return device_kind() != DEVICE_DESKTOP


## 逻辑宽度基准：iPhone 为 960×540，iPad 与桌面为 1280×720；高度随窗口扩展。
func base_canvas() -> Vector2i:
    if device_kind() == DEVICE_PHONE:
        return BASE_CANVAS_PHONE
    return BASE_CANVAS_TABLET


## 逻辑点换算为画布单位。桌面端原样返回；触控端按设备缩放换算。
func hit_min(pt: float) -> int:
    if not is_touch():
        return int(round(pt))
    return canvas_hit_min(pt, DisplayServer.screen_get_scale(), base_canvas(), Vector2(DisplayServer.window_get_size()))


## 安全区四边边距，单位为画布单位，顺序为左、上、右、下。
## 已落在内容框外背景带内的边距记零，不重复扣减。
func safe_insets() -> Vector4i:
    if not is_touch():
        return Vector4i.ZERO
    return canvas_insets(
        Vector2(DisplayServer.window_get_size()),
        base_canvas(),
        DisplayServer.get_display_safe_area(),
        Vector2(DisplayServer.window_get_position())
    )


## 虚拟键盘高度换算为画布单位；桌面端与键盘收起时为零。
func keyboard_inset() -> int:
    if not is_touch():
        return 0
    var height := DisplayServer.virtual_keyboard_get_height()
    if height <= 0:
        return 0
    return keyboard_from_pixels(
        float(height), base_canvas(), Vector2(DisplayServer.window_get_size())
    )


## 测试入口：固定设备类型并同步基准视口；传空串恢复自动判定。
func set_device_for_test(kind: String) -> void:
    _forced = kind
    _apply_base_canvas()


## 每画布单位对应的屏幕像素数：等比缩放取宽高两个方向中较小的比例。
static func canvas_scale(window_size: Vector2, base: Vector2i) -> float:
    if window_size.x <= 0.0 or window_size.y <= 0.0 or base.x <= 0 or base.y <= 0:
        return 0.0
    return minf(window_size.x / float(base.x), window_size.y / float(base.y))


## 虚拟键盘侵入内容框的高度，换算为画布单位。
## 窄于基准比例时内容框上下留背景带，键盘先从背景带算起，只有越过内容框底边的部分才让位。
static func keyboard_from_pixels(pixels: float, base: Vector2i, window_size: Vector2) -> int:
    var scale := canvas_scale(window_size, base)
    if scale <= 0.0 or pixels <= 0.0:
        return 0
    var band := maxf(0.0, (window_size.y - float(base.y) * scale) * 0.5)
    return maxi(0, int(round((pixels - band) / scale)))


## 像素换算为画布单位。
static func canvas_from_pixels(pixels: float, base: Vector2i, window_size: Vector2) -> int:
    var scale := canvas_scale(window_size, base)
    if scale <= 0.0:
        return 0
    return maxi(0, int(round(pixels / scale)))


## 逻辑点换算为画布单位：逻辑点 × 每点像素数 ÷ 每画布单位像素数。
static func canvas_hit_min(pt: float, px_per_pt: float, base: Vector2i, window_size: Vector2) -> int:
    var scale := canvas_scale(window_size, base)
    if scale <= 0.0:
        return int(round(pt))
    return maxi(1, int(round(pt * px_per_pt / scale)))


## 逻辑视口内的基准比例内容框：宽或高按比例收拢后居中。
static func content_frame(viewport: Vector2, base: Vector2i) -> Rect2:
    if viewport.x <= 0.0 or viewport.y <= 0.0 or base.x <= 0 or base.y <= 0:
        return Rect2(Vector2.ZERO, viewport)
    var ratio := float(base.x) / float(base.y)
    var height := minf(viewport.y, viewport.x / ratio)
    var width := height * ratio
    return Rect2(Vector2((viewport.x - width) * 0.5, (viewport.y - height) * 0.5), Vector2(width, height))


## 启动窗口：可用区域的 90%，再收拢成基准比例；可用区域未知时回退基准尺寸。
static func launch_window_size(usable: Vector2i, base: Vector2i) -> Vector2i:
    if usable.x <= 0 or usable.y <= 0:
        return base
    var target := Vector2i(int(float(usable.x) * 0.9), int(float(usable.y) * 0.9))
    var ratio := float(base.x) / float(base.y)
    if float(target.x) / float(target.y) > ratio:
        target.x = int(round(float(target.y) * ratio))
    else:
        target.y = int(round(float(target.x) / ratio))
    return target


## 内容矩形与安全矩形的差，换算为画布单位；背景带内的边距为零。
## 内容框按基准比例 contain 居中，与 `content_frame` 同一模型。
static func canvas_insets(window_size: Vector2, base: Vector2i, safe: Rect2i, window_position: Vector2) -> Vector4i:
    var scale := canvas_scale(window_size, base)
    if scale <= 0.0:
        return Vector4i.ZERO
    var content := Vector2(base.x * scale, base.y * scale)
    var origin := (window_size - content) * 0.5
    var safe_position := Vector2(safe.position) - window_position
    var safe_end := Vector2(safe.end) - window_position
    return Vector4i(
        maxi(0, int(round(maxf(0.0, safe_position.x - origin.x) / scale))),
        maxi(0, int(round(maxf(0.0, safe_position.y - origin.y) / scale))),
        maxi(0, int(round(maxf(0.0, origin.x + content.x - safe_end.x) / scale))),
        maxi(0, int(round(maxf(0.0, origin.y + content.y - safe_end.y) / scale)))
    )


func _apply_base_canvas() -> void:
    if DisplayServer.get_name() == "headless":
        return
    var window := get_window()
    if window == null:
        return
    window.content_scale_size = base_canvas()


func _detect_device() -> String:
    if DisplayServer.get_name() != "iOS":
        return DEVICE_DESKTOP
    var model := OS.get_model_name()
    if model.begins_with("iPad"):
        return DEVICE_TABLET
    if model.begins_with("iPhone") or model.begins_with("iPod"):
        return DEVICE_PHONE
    # 机型名不可用时按屏幕比例兜底：4:3 一类的宽屏按平板处理。
    var screen := DisplayServer.screen_get_size()
    if screen.y > 0 and float(screen.x) / float(screen.y) >= 1.25:
        return DEVICE_TABLET
    return DEVICE_PHONE
