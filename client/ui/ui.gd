class_name UI
extends RefCounted
## 场景共用的小工具；只负责构建 Godot 控件。


static func clear(node: Node) -> void:
    for child in node.get_children():
        node.remove_child(child)
        child.queue_free()


static func label(text: String, size := 16) -> Label:
    var node := Label.new()
    node.text = text
    node.add_theme_font_size_override("font_size", size)
    node.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    return node


static func button(text: String) -> Button:
    var node := Button.new()
    node.text = text
    node.custom_minimum_size = Vector2(0, 34)
    return node


static func panel() -> PanelContainer:
    return PanelContainer.new()


static func margin(container: Control, value := 12) -> MarginContainer:
    var node := MarginContainer.new()
    node.add_theme_constant_override("margin_left", value)
    node.add_theme_constant_override("margin_right", value)
    node.add_theme_constant_override("margin_top", value)
    node.add_theme_constant_override("margin_bottom", value)
    node.add_child(container)
    return node
