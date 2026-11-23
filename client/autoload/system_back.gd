extends Node
## Android 系统返回键与返回手势路由。
##
## 处理顺序：可关闭的弹层 / 页内浮层先收起；没有被收起的浮层时，交给当前页面
## 声明的 `system_back() -> bool`（返回真表示已执行返回动作）；两者都没有时忽略，
## 不退出应用。`project.godot` 的 `application/config/quit_on_go_back=false` 关闭
## 引擎默认的「返回即退出」。
##
## 只收可安全关闭的浮层：好友面板、图鉴筛选面板与图鉴详情抽屉。邀请弹窗、
## 匹配排队浮层、发现 / 进化选择与状态模态都有待处理的业务状态，返回键不代替
## 用户做出选择。

## 允许返回键直接关闭的全局类名；这些类的 `close()` 只收起浮层。
const DISMISSIBLE_CLASSES := [&"FriendsPanel", &"GalleryFilterPanel", &"GalleryDetailLayer"]


func _notification(what: int) -> void:
    if what == NOTIFICATION_WM_GO_BACK_REQUEST and is_inside_tree():
        handle_back()


## 执行一次返回路由；返回是否消费。测试与页面可直接调用。
func handle_back() -> bool:
    if _dismiss_top_layer():
        return true
    var scene := _current_scene()
    if scene != null and scene.has_method("system_back"):
        return bool(scene.call("system_back"))
    return false


## 收起最上层的可关闭浮层；没有时返回假。
func _dismiss_top_layer() -> bool:
    var scene := _current_scene()
    if scene == null:
        return false
    var layers: Array[Node] = []
    _collect_dismissible(scene, layers)
    if layers.is_empty():
        return false
    var top: Node = layers[layers.size() - 1]
    top.call("close")
    return true


func _collect_dismissible(node: Node, layers: Array[Node]) -> void:
    for child in node.get_children():
        if child is CanvasItem and (child as CanvasItem).is_visible_in_tree() and _is_dismissible(child):
            layers.append(child)
        _collect_dismissible(child, layers)


func _is_dismissible(node: Node) -> bool:
    var script: Variant = node.get_script()
    if not (script is Script):
        return false
    return (script as Script).get_global_name() in DISMISSIBLE_CLASSES


func _current_scene() -> Node:
    var tree := get_tree()
    if tree == null:
        return null
    return tree.current_scene
