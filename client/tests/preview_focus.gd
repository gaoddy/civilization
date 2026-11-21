extends Control
## 预览聚焦层：在目标控件外侧画一圈描边，不改页面树与布局。

var target: Control = null


func _init() -> void:
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    set_anchors_preset(Control.PRESET_FULL_RECT)


func _process(_delta: float) -> void:
    if is_instance_valid(target):
        queue_redraw()


func _draw() -> void:
    if not is_instance_valid(target) or not target.is_visible_in_tree():
        return
    var rect := target.get_global_rect()
    var local := Rect2(rect.position - global_position, rect.size)
    draw_rect(local, UITheme.color("color-accent"), false, 3.0)
