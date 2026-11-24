class_name RoomActionBar
extends HBoxContainer
## 房间等待区底部操作栏：左组准备/邀请/离开，右组补位/开始/解散，中间是状态提示。
##
## 只做呈现与按钮可用性；请求与提示文案由页面写入。

signal ready_pressed
signal start_pressed
signal bot_fill_pressed
signal leave_pressed
signal dissolve_pressed
signal steam_invite_pressed

var ready_button: Button
var start: Button
var bot_fill: Button
var leave: Button
var dissolve: Button
var steam_invite: Button
var status: Label

var _left: HBoxContainer
var _right: HBoxContainer


func _init() -> void:
    name = "RoomActionBar"
    size_flags_horizontal = Control.SIZE_EXPAND_FILL
    add_theme_constant_override("separation", UITheme.space("space-md"))

    _left = _group("LeftActions")
    add_child(_left)
    ready_button = UIControls.styled_button(tr("ui.room.ready"), "primary", 30)
    ready_button.name = "ReadyButton"
    ready_button.pressed.connect(func() -> void: ready_pressed.emit())
    _left.add_child(ready_button)

    # Steam 构建多一个好友邀请入口；其余构建不创建。
    if Platform.is_steam():
        steam_invite = UIControls.button(tr("ui.room.invite_steam.entry"))
        steam_invite.name = "SteamInviteButton"
        steam_invite.pressed.connect(func() -> void: steam_invite_pressed.emit())
        _left.add_child(steam_invite)

    leave = UIControls.styled_button(tr("ui.room.leave"), "ghost", 30)
    leave.name = "LeaveButton"
    leave.pressed.connect(func() -> void: leave_pressed.emit())
    _left.add_child(leave)

    status = UIControls.label("", UITheme.font_size("font-body-sm"), false)
    status.name = "StatusLabel"
    status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    status.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    status.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    status.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    add_child(status)

    _right = _group("RightActions")
    add_child(_right)
    bot_fill = _gold_button()
    bot_fill.name = "BotFillButton"
    bot_fill.pressed.connect(func() -> void: bot_fill_pressed.emit())
    _right.add_child(bot_fill)

    start = UIControls.styled_button(tr("ui.room.start"), "primary", 30)
    start.name = "StartButton"
    start.pressed.connect(func() -> void: start_pressed.emit())
    _right.add_child(start)

    dissolve = UIControls.styled_button(tr("ui.room.dissolve"), "ghost", 30)
    dissolve.name = "DissolveButton"
    dissolve.pressed.connect(func() -> void: dissolve_pressed.emit())
    _right.add_child(dissolve)


## state: {ready, is_host, connected, ready_count, human_count, bot_fill,
##         can_start, start_reason, bot_count}
func set_state(state: Dictionary) -> void:
    var connected := bool(state.get("connected", false))
    var is_host := bool(state.get("is_host", false))
    var bot_fill_on := bool(state.get("bot_fill", false))

    # 房主默认已准备由服务端保证，等待区不给房主准备按钮。
    ready_button.visible = not is_host
    ready_button.text = tr("ui.room.unready") if bool(state.get("ready", false)) else tr("ui.room.ready")
    ready_button.disabled = not connected

    start.visible = is_host
    start.disabled = not connected or not bool(state.get("can_start", false))
    var reason := str(state.get("start_reason", ""))
    start.tooltip_text = reason if start.disabled else ""

    dissolve.visible = is_host

    bot_fill.disabled = not is_host or not connected
    bot_fill.text = "%s %s" % [
        tr("ui.room.bot_fill"),
        tr("ui.room.bot_fill_on") if bot_fill_on else tr("ui.room.bot_fill_off"),
    ]
    bot_fill.tooltip_text = tr("ui.room.bot_fill_hint") % int(state.get("bot_count", 0))


## Steam 邀请入口的可用性与禁用原因；非 Steam 构建没有该控件。
func set_steam_available(available: bool, reason := "") -> void:
    if steam_invite == null:
        return
    steam_invite.disabled = not available
    steam_invite.tooltip_text = "" if available else reason


func _group(node_name: String) -> HBoxContainer:
    var node := HBoxContainer.new()
    node.name = node_name
    node.alignment = BoxContainer.ALIGNMENT_CENTER
    node.add_theme_constant_override("separation", UITheme.space("space-sm"))
    return node


## 补位开关使用金色描边：共享层没有 gold 变体，这里按令牌组合。
func _gold_button() -> Button:
    var node := UIControls.styled_button("", "default", 30)
    var accent := UITheme.color("color-accent")
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        var style := UITheme.button_style("default", state)
        match state:
            "disabled":
                style.bg_color = UITheme.color("color-surface")
                style.border_color = UITheme.color("color-border")
            "hover":
                style.bg_color = UITheme.alpha("color-accent", 0.2)
                style.border_color = accent
            "pressed":
                style.bg_color = UITheme.alpha("color-accent", 0.28)
                style.border_color = accent
            _:
                style.bg_color = UITheme.alpha("color-accent", 0.12)
                style.border_color = accent
        node.add_theme_stylebox_override(state, style)
    node.add_theme_color_override("font_color", accent)
    node.add_theme_color_override("font_hover_color", accent)
    node.add_theme_color_override("font_pressed_color", accent)
    node.add_theme_color_override("font_focus_color", accent)
    node.add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
    return node
