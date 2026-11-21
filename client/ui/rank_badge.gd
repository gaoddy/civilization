class_name RankBadge
extends RefCounted
## 名次角标：叠在座位头像内的圆形深色底，白色数字；已淘汰也继续显示。


static func create(rank: int) -> PanelContainer:
    var size := UITheme.stage_metric("rank-badge-size")
    var node := PanelContainer.new()
    node.name = "RankBadge"
    node.custom_minimum_size = Vector2(size, size)
    node.size = Vector2(size, size)
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE

    var box := StyleBoxFlat.new()
    box.bg_color = UITheme.alpha("color-bg", 0.72)
    box.border_color = UITheme.alpha("color-white", 0.25)
    box.set_border_width_all(1)
    box.set_corner_radius_all(size / 2)
    node.add_theme_stylebox_override("panel", box)

    var text := UIControls.label(str(rank) if rank > 0 else "—", UITheme.font_size("font-caption"), false)
    text.name = "RankText"
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    text.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    text.mouse_filter = Control.MOUSE_FILTER_IGNORE
    text.add_theme_color_override("font_color", UITheme.color("color-white"))
    node.add_child(text)
    return node
