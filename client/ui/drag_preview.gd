class_name UIDrag
extends RefCounted
## 拖拽预览构造：让指针停在起手时的卡片位置，而不是 Godot 默认的预览左上角贴指针。


## Viewport 在拖拽中每帧把预览控件的位置设成指针位置，直接偏移预览会被覆盖；
## 这里用包裹层承载被拖卡片，并按起手点 `at_position` 反向偏移。
static func preview(card: Control, at_position: Vector2, context: Node = null) -> Control:
    var holder := Control.new()
    holder.name = "DragPreview"
    holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
    # 预览卡本身不在树上，缩放要从发起拖拽的节点取。
    var scale := UILayout.root_scale(context if context != null else card)
    holder.scale = Vector2(scale, scale)
    card.mouse_filter = Control.MOUSE_FILTER_IGNORE
    # 起手点是内容框局部单位；包裹层按同一比例缩放，子节点偏移不能再除缩放。
    card.position = -at_position
    holder.add_child(card)
    return holder
