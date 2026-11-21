class_name RankBadge
extends RefCounted
## 名次角标：头像左上角的金色小方块，已淘汰也继续显示数字。


static func create(rank: int) -> PanelContainer:
    var size := UITheme.stage_metric("rank-badge-size")
    var node := PanelContainer.new()
    node.name = "RankBadge"
    node.custom_minimum_size = Vector2(size, size)
    node.size = Vector2(size, size)
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE

    var box := StyleBoxFlat.new()
    box.bg_color = UITheme.color("color-accent-strong")
    box.border_color = UITheme.color("color-accent")
    box.set_border_width_all(1)
    box.set_corner_radius_all(UITheme.radius("radius-sm"))
    node.add_theme_stylebox_override("panel", box)

    var text := UIControls.label(str(rank) if rank > 0 else "—", UITheme.font_size("font-caption"), false)
    text.name = "RankText"
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    text.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    text.mouse_filter = Control.MOUSE_FILTER_IGNORE
    text.add_theme_color_override("font_color", UITheme.color("color-bg"))
    node.add_child(text)
    return node
