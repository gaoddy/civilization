class_name HpBar
extends RefCounted
## 血量条：固定宽度，默认不显示数字；`value` 超出 [0, cap] 时填充夹到两端。
## `corner_radius` 大于 0 时轨道与填充按圆角绘制，供叠在头像内的小胶囊使用。
## `height` 为 0 时取 `seat-hp-bar-height`；`label_text` 非空时居中叠在条上，
## 供座位详情等需要「当前/上限」数字的场景使用，座位行小血条保持不显示数字。


static func create(
    value: int,
    cap: int,
    width: int,
    corner_radius := 0,
    height := 0,
    label_text := ""
) -> Control:
    var bar_height := height if height > 0 else UITheme.stage_metric("seat-hp-bar-height")
    var node := Control.new()
    node.name = "HpBar"
    node.custom_minimum_size = Vector2(width, bar_height)
    node.size = Vector2(width, bar_height)
    node.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE

    node.add_child(_part("HpTrack", UITheme.color("color-surface-sunken"), width, bar_height, corner_radius))

    var safe_cap := maxi(cap, 1)
    var clamped := clampi(value, 0, safe_cap)
    var fill_width := clampi(int(round(float(width) * float(clamped) / float(safe_cap))), 0, width)
    node.add_child(_part("HpFill", UITheme.color("color-success"), fill_width, bar_height, corner_radius))
    if label_text != "":
        node.add_child(_label(label_text))
    return node


## 叠字：白字加 1px 深色阴影，压在填充段与轨道段上都可读。
static func _label(text: String) -> Label:
    var label := UIControls.label(text, UITheme.font_size("font-caption"), false)
    label.name = "HpLabel"
    label.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    label.add_theme_color_override("font_color", UITheme.color("color-white"))
    label.add_theme_color_override("font_shadow_color", UITheme.color("color-bg"))
    label.add_theme_constant_override("shadow_offset_x", 1)
    label.add_theme_constant_override("shadow_offset_y", 1)
    return label


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
