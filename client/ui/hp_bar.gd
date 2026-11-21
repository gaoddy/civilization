class_name HpBar
extends RefCounted
## 血量条：固定宽度、不显示数字；`value` 超出 [0, cap] 时填充夹到两端。
## `corner_radius` 大于 0 时轨道与填充按圆角绘制，供叠在头像内的小胶囊使用。


static func create(value: int, cap: int, width: int, corner_radius := 0) -> Control:
    var height := UITheme.stage_metric("seat-hp-bar-height")
    var node := Control.new()
    node.name = "HpBar"
    node.custom_minimum_size = Vector2(width, height)
    node.size = Vector2(width, height)
    node.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE

    node.add_child(_part("HpTrack", UITheme.color("color-surface-sunken"), width, height, corner_radius))

    var safe_cap := maxi(cap, 1)
    var clamped := clampi(value, 0, safe_cap)
    var fill_width := clampi(int(round(float(width) * float(clamped) / float(safe_cap))), 0, width)
    node.add_child(_part("HpFill", UITheme.color("color-success"), fill_width, height, corner_radius))
    return node


## 轨道与填充共用：纯色块；圆角为 0 时与原来的直角色块外观一致。
static func _part(part_name: String, color: Color, width: int, height: int, corner_radius: int) -> Panel:
    var part := Panel.new()
    part.name = part_name
    var style := StyleBoxFlat.new()
    style.bg_color = color
    style.set_corner_radius_all(corner_radius)
    style.set_content_margin_all(0)
    part.add_theme_stylebox_override("panel", style)
    part.position = Vector2.ZERO
    part.size = Vector2(width, height)
    part.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return part
