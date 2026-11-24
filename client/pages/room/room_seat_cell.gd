class_name RoomSeatCell
extends PanelContainer
## 房间等待区的一个座位格：头像圆盘 + 名字 + 徽标行。
##
## 空位显示「等待加入」并画虚线边框；本人座位用金色描边；房主与准备状态用胶囊徽标。
## 移除按钮只在页面传入可见标记时创建，非房主或空位不显示。

signal kick_pressed(player_id: String)

var seat := 0
var player_id := ""
var display_name := ""
var is_self := false
var occupied := false
var is_host := false
var is_ready := false

var _content: VBoxContainer
var _kick: Button
var _handler := Callable()


func _init() -> void:
    mouse_filter = Control.MOUSE_FILTER_STOP
    resized.connect(queue_redraw)


func setup(seat_number: int, player: Dictionary, self_seat: bool) -> void:
    seat = seat_number
    occupied = not player.is_empty()
    player_id = "" if not occupied else str(player.get("playerId", ""))
    display_name = "" if not occupied else str(player.get("displayName", player_id))
    if occupied and display_name == "":
        display_name = player_id
    is_self = self_seat
    is_host = occupied and bool(player.get("isHost", false))
    is_ready = occupied and bool(player.get("ready", false))
    _build()
    queue_redraw()


## 供页面设置移除按钮回调；非房主或空位传 visible = false，不创建按钮。
func set_kick_handler(handler: Callable, visible: bool) -> void:
    _handler = handler
    if not visible or not occupied or is_self:
        if _kick != null:
            _kick.queue_free()
            _kick = null
        return
    if _kick != null:
        return
    _kick = UIControls.styled_button(
        tr("ui.room.kick"), "micro", UITheme.stage_metric("action-button-height")
    )
    _kick.name = "Kick_%s" % player_id
    _kick.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    _kick.pressed.connect(_on_kick_pressed)
    _content.add_child(_kick)


func _build() -> void:
    UIControls.clear(self)
    add_theme_stylebox_override("panel", _panel_style())

    _content = VBoxContainer.new()
    _content.name = "SeatContent"
    _content.alignment = BoxContainer.ALIGNMENT_CENTER
    _content.add_theme_constant_override("separation", UITheme.space("space-xs"))
    _content.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(_content)

    _content.add_child(_avatar())
    if occupied:
        _content.add_child(_name_label())
        _content.add_child(_badges())
    else:
        _content.add_child(_empty_badge())


## 本人金色描边；空位透明底 + 虚线由 `_draw` 补；其余用普通抬升面板。
func _panel_style() -> StyleBoxFlat:
    if not occupied:
        var empty := StyleBoxFlat.new()
        empty.set_corner_radius_all(UITheme.radius("radius-md"))
        empty.set_content_margin_all(UITheme.space("space-md"))
        empty.bg_color = UITheme.color("color-transparent")
        empty.border_color = UITheme.color("color-transparent")
        empty.set_border_width_all(0)
        return empty
    var style := UITheme.panel_style("raised-tight")
    if is_self:
        style.bg_color = UITheme.alpha("color-accent", 0.08)
        style.border_color = UITheme.color("color-accent")
        style.set_border_width_all(2)
    return style


func _avatar() -> PanelContainer:
    var avatar := PanelContainer.new()
    avatar.name = "SeatAvatar"
    avatar.custom_minimum_size = Vector2(26, 26)
    avatar.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    avatar.mouse_filter = Control.MOUSE_FILTER_IGNORE
    avatar.add_theme_stylebox_override("panel", _avatar_style())

    var initial := "+" if not occupied else display_name.substr(0, 1)
    var text := UIControls.label(initial, UITheme.font_size("font-body-sm"), false)
    text.name = "SeatAvatarText"
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    text.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    text.mouse_filter = Control.MOUSE_FILTER_IGNORE
    if not occupied:
        text.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    elif is_self:
        text.add_theme_color_override("font_color", UITheme.color("color-accent"))
    else:
        text.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    avatar.add_child(text)
    return avatar


func _avatar_style() -> StyleBoxFlat:
    var style := StyleBoxFlat.new()
    style.set_corner_radius_all(13)
    style.set_content_margin_all(0)
    style.set_border_width_all(1)
    if not occupied:
        style.bg_color = UITheme.color("color-transparent")
        style.border_color = UITheme.color("color-border")
    elif is_self:
        style.bg_color = UITheme.alpha("color-accent", 0.12)
        style.border_color = UITheme.color("color-accent")
    else:
        style.bg_color = UITheme.color("color-surface-sunken")
        style.border_color = UITheme.color("color-border-strong")
    return style


func _name_label() -> Label:
    var label := UIControls.label(display_name, UITheme.font_size("font-body-sm"), false)
    label.name = "SeatName"
    label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return label


## 空位胶囊：方向 A 的空位卡用胶囊写明「等待加入」，触屏端同样可读。
func _empty_badge() -> Label:
    var pill := UIControls.pill_label(tr("ui.room.seat_empty"), "neutral", "font-caption")
    pill.name = "SeatEmptyBadge"
    pill.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    pill.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    pill.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return pill


func _badges() -> HBoxContainer:
    var badges := HBoxContainer.new()
    badges.name = "SeatBadges"
    badges.alignment = BoxContainer.ALIGNMENT_CENTER
    badges.add_theme_constant_override("separation", UITheme.space("space-xs"))
    badges.mouse_filter = Control.MOUSE_FILTER_IGNORE
    if is_host:
        var host := UIControls.pill_label(tr("ui.room.host"), "accent", "font-caption")
        host.name = "HostBadge"
        badges.add_child(host)
    var ready := is_ready
    var state := UIControls.pill_label(
        tr("ui.room.seat_ready") if ready else tr("ui.room.seat_unready"),
        "success" if ready else "neutral",
        "font-caption",
    )
    state.name = "ReadyBadge"
    badges.add_child(state)
    return badges


func _on_kick_pressed() -> void:
    if _handler.is_valid():
        _handler.call(player_id)
    else:
        kick_pressed.emit(player_id)


func _draw() -> void:
    if occupied or size.x <= 1.0 or size.y <= 1.0:
        return
    var inset := 0.5
    var top_left := Vector2(inset, inset)
    var top_right := Vector2(size.x - inset, inset)
    var bottom_left := Vector2(inset, size.y - inset)
    var bottom_right := Vector2(size.x - inset, size.y - inset)
    var color := UITheme.alpha("color-border-strong", 0.9)
    draw_dashed_line(top_left, top_right, color, 1.0, 4.0)
    draw_dashed_line(bottom_left, bottom_right, color, 1.0, 4.0)
    draw_dashed_line(top_left, bottom_left, color, 1.0, 4.0)
    draw_dashed_line(top_right, bottom_right, color, 1.0, 4.0)
