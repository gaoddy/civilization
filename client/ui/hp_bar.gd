class_name HpBar
extends RefCounted
## 血量条：固定宽度、不显示数字；`value` 超出 [0, cap] 时填充夹到两端。


static func create(value: int, cap: int, width: int) -> Control:
    var height := UITheme.stage_metric("seat-hp-bar-height")
    var node := Control.new()
    node.name = "HpBar"
    node.custom_minimum_size = Vector2(width, height)
    node.size = Vector2(width, height)
    node.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE

    var track := ColorRect.new()
    track.name = "HpTrack"
    track.color = UITheme.color("color-surface-sunken")
    track.position = Vector2.ZERO
    track.size = Vector2(width, height)
    track.mouse_filter = Control.MOUSE_FILTER_IGNORE
    node.add_child(track)

    var safe_cap := maxi(cap, 1)
    var clamped := clampi(value, 0, safe_cap)
    var fill_width := clampi(int(round(float(width) * float(clamped) / float(safe_cap))), 0, width)
    var fill := ColorRect.new()
    fill.name = "HpFill"
    fill.color = UITheme.color("color-success")
    fill.position = Vector2.ZERO
    fill.size = Vector2(fill_width, height)
    fill.mouse_filter = Control.MOUSE_FILTER_IGNORE
    node.add_child(fill)
    return node
