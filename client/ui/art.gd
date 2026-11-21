class_name UIArt
extends RefCounted
## 美术贴图加载：路径缺失或资源失败时返回 null，由调用方回退程序化绘制。


## 取美术贴图；路径为空、资源缺失或加载失败时返回 null，调用方回退程序化绘制。
static func texture(path: String) -> Texture2D:
    if path == "" or not ResourceLoader.exists(path):
        return null
    return ResourceLoader.load(path) as Texture2D


## 铺满父容器的美术层；无图时返回 null。`stretch` 决定裁切方式：
## 卡面与头像用 `COVERED`，竖版立绘用 `KEEP_ASPECT_CENTERED`。
static func layer(
    path: String,
    node_name := "Art",
    stretch := TextureRect.STRETCH_KEEP_ASPECT_COVERED,
) -> TextureRect:
    var loaded := texture(path)
    if loaded == null:
        return null
    var art := TextureRect.new()
    art.name = node_name
    art.texture = loaded
    art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
    art.stretch_mode = stretch
    art.set_anchors_preset(Control.PRESET_FULL_RECT)
    art.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return art
