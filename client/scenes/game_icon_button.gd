class_name GameIconButton
extends Button
## 图标按钮：40×40，图标由 `Icons` 绘制，费用角标在右上、剩余次数角标在左上。
##
## 只发出继承的 `pressed` 信号，不构造意图；文案与小字由调用方设置 `tooltip_text`。

const BADGE_SIZE := 15.0
## 角标外扩量：费用角标越过按钮右上角，次数角标越过左上角。
const BADGE_OVERHANG := 6.0
const ICON_INSET := 10.0
const ICON_EXTENT := 20.0
const ICON_STROKE := 2.0
const DISABLED_ALPHA := 0.45

var icon_name := ""
var cost := -1
var uses := 1
var uses_visible := false:
	set(value):
		uses_visible = value
		_refresh_uses_badge()
var active := false

var _cost_badge: Label = null
var _uses_badge: Label = null


func _init(icon_name := "", badge_prefix := "", show_cost := true) -> void:
	self.icon_name = icon_name
	var side := UITheme.stage_metric("shop-icon-size")
	custom_minimum_size = Vector2(side, side)
	size = Vector2(side, side)
	focus_mode = Control.FOCUS_ALL
	text = ""
	_apply_styles()
	if show_cost:
		_cost_badge = _make_badge(
			badge_prefix + "CostBadge",
			UITheme.color("color-accent"),
			UITheme.color("color-bg"),
			true,
		)
	_uses_badge = _make_badge(
		badge_prefix + "UsesBadge",
		UITheme.color("color-surface-sunken"),
		UITheme.color("color-text"),
		false,
	)
	resized.connect(_layout_badges)
	_refresh_cost_badge()
	_refresh_uses_badge()


func set_cost(value: int) -> void:
	cost = value
	_refresh_cost_badge()


func set_uses(value: int) -> void:
	uses = value
	_refresh_uses_badge()


func set_active(value: bool) -> void:
	active = value
	_apply_styles()
	queue_redraw()


func _draw() -> void:
	_sync_disabled_visual()
	Icons.draw(
		self,
		icon_name,
		Rect2(ICON_INSET, ICON_INSET, ICON_EXTENT, ICON_EXTENT),
		_icon_color(),
		ICON_STROKE,
	)


func _icon_color() -> Color:
	if disabled:
		return UITheme.color("color-text-muted")
	if active:
		return UITheme.color("color-info")
	return UITheme.color("color-text")


## 禁用时整枚按钮压暗；由 `_draw` 前同步，避免依赖原生属性变更信号。
func _sync_disabled_visual() -> void:
	var target := Color(1, 1, 1, DISABLED_ALPHA) if disabled else Color.WHITE
	if modulate != target:
		modulate = target


func _apply_styles() -> void:
	for state in ["normal", "hover", "pressed", "focus", "disabled"]:
		var box := UITheme.button_style("slot", state)
		if active and (state == "normal" or state == "hover"):
			box.border_color = UITheme.color("color-info")
			box.bg_color = UITheme.alpha("color-info", 0.12)
		add_theme_stylebox_override(state, box)


func _refresh_cost_badge() -> void:
	if _cost_badge == null:
		return
	_cost_badge.text = str(cost) if cost >= 0 else ""
	_cost_badge.visible = cost >= 0


## 默认 1 次不显示；0 次与 2 次以上显示数字。
func _refresh_uses_badge() -> void:
	if _uses_badge == null:
		return
	_uses_badge.text = str(uses)
	_uses_badge.visible = uses_visible and uses != 1


func _make_badge(node_name: String, background: Color, text_color: Color, right_side: bool) -> Label:
	var badge := Label.new()
	badge.name = node_name
	badge.mouse_filter = Control.MOUSE_FILTER_IGNORE
	badge.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	badge.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	badge.add_theme_font_size_override("font_size", UITheme.font_size("font-micro"))
	badge.add_theme_color_override("font_color", text_color)
	var box := StyleBoxFlat.new()
	box.bg_color = background
	box.border_color = Color(0, 0, 0, 0)
	box.set_corner_radius_all(UITheme.radius("radius-sm"))
	box.set_content_margin_all(0)
	badge.add_theme_stylebox_override("normal", box)
	add_child(badge)
	_layout_badge(badge, right_side)
	return badge


func _layout_badges() -> void:
	if _cost_badge != null:
		_layout_badge(_cost_badge, true)
	if _uses_badge != null:
		_layout_badge(_uses_badge, false)


func _layout_badge(badge: Label, right_side: bool) -> void:
	badge.custom_minimum_size = Vector2(BADGE_SIZE, BADGE_SIZE)
	badge.size = Vector2(BADGE_SIZE, BADGE_SIZE)
	if right_side:
		badge.set_anchors_preset(Control.PRESET_TOP_RIGHT)
		badge.offset_left = BADGE_OVERHANG - BADGE_SIZE
		badge.offset_right = BADGE_OVERHANG
	else:
		badge.set_anchors_preset(Control.PRESET_TOP_LEFT)
		badge.offset_left = -BADGE_OVERHANG
		badge.offset_right = -BADGE_OVERHANG + BADGE_SIZE
	badge.offset_top = -BADGE_OVERHANG
	badge.offset_bottom = -BADGE_OVERHANG + BADGE_SIZE
