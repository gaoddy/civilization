class_name UILayout
extends RefCounted
## 页面安全区与内容框：16:9 内容框、逻辑画布与动态覆盖图层。
##
## 页面只调用入口，不关心包裹结构；视觉取值来自 ScreenProfile，不读取主题。

const SAFE_AREA_NAME := "SafeAreaMargin"
const SAFE_LAYER_NAME := "SafeLayer"
const PAGE_BACKGROUND_NAME := "PageBackground"
const CONTENT_FRAME_NAME := "ContentFrame"
const CONTENT_ROOT_NAME := "ContentRoot"


## 页面安全区包裹：在页面与内容之间插入 16:9 内容框与逻辑画布，再把内容移入安全区。
## 整屏背景留在内容框外并置底，安全区只约束内容；幂等：重复调用返回已存在的包裹层。
static func safe_area(page: Control) -> SafeAreaMargin:
    if page == null:
        return null
    var existing := _safe_area_of(page)
    if existing != null:
        return existing
    var frame := Control.new()
    frame.name = CONTENT_FRAME_NAME
    frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
    page.add_child(frame)
    var root := Control.new()
    root.name = CONTENT_ROOT_NAME
    root.size = Vector2(ScreenProfile.base_canvas())
    root.mouse_filter = Control.MOUSE_FILTER_IGNORE
    frame.add_child(root)
    var area := SafeAreaMargin.new()
    area.name = SAFE_AREA_NAME
    root.add_child(area)
    # 包裹层创建时页面可能已有尺寸：只设锚点会把当前零尺寸保留成负偏移。
    area.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    for child in page.get_children():
        if child == frame:
            continue
        if child.name == PAGE_BACKGROUND_NAME:
            page.move_child(child, 0)
            continue
        page.remove_child(child)
        # 重新挂入会触发 _exit_tree：依赖 _ready 重新建立订阅的组件需要再就绪一次。
        child.request_ready()
        area.add_child(child)
    _layout_content(page, frame, root)
    if not page.has_meta("content_layout_connected"):
        page.set_meta("content_layout_connected", true)
        page.resized.connect(_layout_content.bind(page, frame, root))
    return area


## 按页面尺寸把内容框收拢成基准比例，并缩放逻辑画布。
## 页面子树被提前释放时绑定对象会失效，回调按空操作处理。
static func _layout_content(page: Control, frame: Control, root: Control) -> void:
    if not is_instance_valid(frame) or not is_instance_valid(root):
        return
    var base := ScreenProfile.base_canvas()
    var rect := ScreenProfile.content_frame(page.size, base)
    if rect.size.x <= 0.0 or rect.size.y <= 0.0:
        return
    frame.position = rect.position
    frame.size = rect.size
    var scale := rect.size.y / float(base.y)
    root.scale = Vector2(scale, scale)


## 向上查找内容框逻辑画布的缩放；不在内容框内时返回 1。
static func root_scale(node: Node) -> float:
    var current := node
    while current != null:
        if current is Control and str(current.name) == CONTENT_ROOT_NAME:
            return (current as Control).scale.x
        current = current.get_parent()
    return 1.0


## 动态全屏覆盖层的挂载点：位于安全区内、内容之上，自身不拦截输入。
static func safe_layer(page: Control) -> Control:
    var area := safe_area(page)
    if area == null:
        return null
    var layer := area.get_node_or_null(SAFE_LAYER_NAME) as Control
    if layer == null:
        layer = Control.new()
        layer.name = SAFE_LAYER_NAME
        layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
        area.add_child(layer)
        layer.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    return layer


static func _safe_area_of(page: Control) -> SafeAreaMargin:
    return page.find_child(SAFE_AREA_NAME, true, false) as SafeAreaMargin
