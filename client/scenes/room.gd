extends Control
## 选英雄页：顶部只显示其他玩家的名字与确认状态，底部是本人候选英雄卡。
##
## 本页不显示房间码，也没有离开房间入口；点卡选中、点确认一次完成「选英雄 + 准备」。
## 候选由服务端分配且只发给本人，因此这里从不展示别人的英雄。

const CARD_SIZE := Vector2(160, 200)

var _others: HBoxContainer
var _candidates: HBoxContainer
var _confirm: Button
var _status: Label
var _dismiss: Control
var _skill_layer: Control
var _skill_panel: VBoxContainer

var _selected_hero := ""


func _ready() -> void:
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.game_started.connect(_on_game_started)
    GameStore.view_changed.connect(_on_view_changed)
    GameStore.error_received.connect(_on_error)
    # 断线不产生房间或视图消息：直接订阅连接状态，否则「未连接」永远不刷新。
    Net.connected.connect(_sync_state)
    Net.disconnected.connect(_sync_state)
    _refresh()


func _build() -> void:
    var background := ColorRect.new()
    background.color = UITheme.color("color-bg")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    background.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(background)

    # 点浮层外的空白关闭选中：铺满全屏、位于内容之下，候选卡仍然优先接收点击。
    _dismiss = Control.new()
    _dismiss.name = "SkillDismissLayer"
    _dismiss.set_anchors_preset(Control.PRESET_FULL_RECT)
    _dismiss.mouse_filter = Control.MOUSE_FILTER_STOP
    _dismiss.visible = false
    _dismiss.gui_input.connect(_on_dismiss_input)
    add_child(_dismiss)

    # 面板铺满整屏、尺寸不随内容变化；内容块在面板内水平与垂直居中。
    var panel := UI.card_panel("surface")
    panel.name = "DraftPanel"
    panel.set_anchors_preset(Control.PRESET_FULL_RECT)
    # 面板只做视觉容器：空白处的点击要落到 `SkillDismissLayer`。
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(panel)

    var center := CenterContainer.new()
    center.name = "ContentCenter"
    center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    panel.add_child(center)

    var root := VBoxContainer.new()
    root.name = "DraftColumn"
    root.add_theme_constant_override("separation", UITheme.space("space-md"))
    # 页面容器一律放行鼠标：空白处的点击要落到 `SkillDismissLayer`，候选卡自己处理点击。
    root.mouse_filter = Control.MOUSE_FILTER_IGNORE
    center.add_child(root)

    _others = HBoxContainer.new()
    _others.name = "OtherPlayers"
    _others.alignment = BoxContainer.ALIGNMENT_CENTER
    _others.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _others.add_theme_constant_override("separation", UITheme.space("space-md"))
    root.add_child(_others)

    var title := UI.section_title(tr("ui.room.select_hero"))
    title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    title.mouse_filter = Control.MOUSE_FILTER_IGNORE
    root.add_child(title)

    # 候选固定 4 张、单行展示：候选行按整行宽度居中，不决定面板尺寸。
    _candidates = HBoxContainer.new()
    _candidates.name = "HeroCandidates"
    _candidates.alignment = BoxContainer.ALIGNMENT_CENTER
    _candidates.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _candidates.add_theme_constant_override("separation", UITheme.space("space-md"))
    root.add_child(_candidates)

    _confirm = UI.styled_button(tr("ui.room.confirm"), "primary", 36)
    _confirm.name = "ConfirmButton"
    # 纵向容器默认拉满整行宽；确认键按自身宽度居中，与其他内容同轴。
    _confirm.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    _confirm.pressed.connect(_on_confirm_pressed)
    root.add_child(_confirm)

    _status = UI.label("", UITheme.font_size("font-body-sm"))
    _status.name = "StatusLabel"
    _status.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    root.add_child(_status)

    _build_skill_layer()


func _build_skill_layer() -> void:
    # 视觉层在最上层但整体忽略鼠标：浮层不得挡住候选卡与确认按钮。
    _skill_layer = Control.new()
    _skill_layer.name = "SkillLayer"
    _skill_layer.set_anchors_preset(Control.PRESET_FULL_RECT)
    _skill_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _skill_layer.visible = false
    add_child(_skill_layer)

    var panel := UI.card_panel("raised")
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    panel.name = "SkillPanel"
    panel.set_anchors_preset(Control.PRESET_CENTER)
    panel.custom_minimum_size = Vector2(320, 0)
    # 居中预设按控件当前位置摆放；尺寸由内容撑开时向两侧生长才是真正居中。
    panel.grow_horizontal = Control.GROW_DIRECTION_BOTH
    panel.grow_vertical = Control.GROW_DIRECTION_BOTH
    _skill_layer.add_child(panel)

    _skill_panel = VBoxContainer.new()
    _skill_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _skill_panel.add_theme_constant_override("separation", UITheme.space("space-sm"))
    panel.add_child(_skill_panel)


func _refresh() -> void:
    # 重连或晚到的 RoomUpdate：以服务端的已选英雄为准，避免多端不一致。
    if _is_confirmed():
        _selected_hero = GameStore.my_hero_id()
    _render_others()
    _render_candidates()
    _show_skill(_selected_hero)
    _sync_state()


## 顶部：其他玩家的名字与确认状态；不展示任何人的英雄选择。
func _render_others() -> void:
    UI.clear(_others)
    var shown := 0
    for player in GameStore.room_players:
        if str(player.get("playerId", "")) == GameStore.player_id:
            continue
        var display := str(player.get("displayName", ""))
        if display == "":
            display = str(player.get("playerId", ""))
        var ready := bool(player.get("ready", false))
        var status := tr("ui.room.confirmed") if ready else tr("ui.room.unconfirmed")
        var text := "%s · %s" % [display, status]
        var row := UI.pill_label(text, "success" if ready else "neutral")
        row.name = "OtherPlayer%d" % shown
        _others.add_child(row)
        shown += 1
    if shown == 0:
        _others.add_child(UI.empty_hint(tr("ui.room.waiting_for_players")))


## 底部：本人候选卡。选中态加描边，已确认后全部禁用。
func _render_candidates() -> void:
    UI.clear(_candidates)

    var candidates := GameStore.my_hero_candidates()
    if candidates.is_empty():
        _candidates.add_child(UI.empty_hint(tr("ui.room.assigning_hero")))
        return

    var locked := _is_confirmed()
    for hero_id in candidates:
        var id := str(hero_id)
        var card := _build_card(id, locked)
        _candidates.add_child(card)


## 候选卡：部落色封面铺满、居中首字水印、底部居中名字。
func _build_card(hero_id: String, locked: bool) -> PanelContainer:
    var entry := CardDB.card(hero_id)
    var tribe := str(entry.get("tribe", ""))
    var card := PanelContainer.new()
    card.name = "HeroCard_%s" % hero_id
    card.custom_minimum_size = CARD_SIZE
    card.mouse_filter = Control.MOUSE_FILTER_STOP

    var box := UITheme.panel_style("raised")
    # 封面要铺满整张卡：面板样式默认带 `space-lg` 内边距，会把封面缩进一圈。
    box.set_content_margin_all(0)
    if hero_id == _selected_hero:
        box.border_color = UITheme.color("color-accent")
        box.set_border_width_all(2)
    card.add_theme_stylebox_override("panel", box)

    var stack := Control.new()
    stack.custom_minimum_size = CARD_SIZE
    stack.mouse_filter = Control.MOUSE_FILTER_IGNORE
    card.add_child(stack)

    var cover := ColorRect.new()
    cover.color = UITheme.tribe_cover(tribe)
    cover.set_anchors_preset(Control.PRESET_FULL_RECT)
    cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_child(cover)

    var watermark := UI.label(CardDB.display_name(hero_id).substr(0, 1), 72, false)
    watermark.name = "Watermark"
    watermark.set_anchors_preset(Control.PRESET_FULL_RECT)
    watermark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    watermark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    watermark.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
    watermark.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_child(watermark)

    var name_label := UI.label(CardDB.display_name(hero_id), UITheme.font_size("font-title"), false)
    name_label.name = "Name"
    name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    name_label.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
    name_label.offset_top = -UITheme.space("space-2xl")
    name_label.offset_bottom = -UITheme.space("space-md")
    name_label.add_theme_color_override("font_color", UITheme.color("color-text"))
    name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_child(name_label)

    if locked:
        card.modulate = Color(1, 1, 1, UITheme.opacity("opacity-dim"))
    else:
        card.gui_input.connect(_on_card_input.bind(hero_id))
    return card


func _on_card_input(event: InputEvent, hero_id: String) -> void:
    if not (event is InputEventMouseButton):
        return
    var mouse := event as InputEventMouseButton
    if not mouse.pressed or mouse.button_index != MOUSE_BUTTON_LEFT:
        return
    if _is_confirmed():
        return
    if _selected_hero == hero_id:
        _select("")
    else:
        _select(hero_id)


## 点浮层以外的空白：取消选中并收起浮层。
func _on_dismiss_input(event: InputEvent) -> void:
    if not (event is InputEventMouseButton):
        return
    var mouse := event as InputEventMouseButton
    if mouse.pressed and mouse.button_index == MOUSE_BUTTON_LEFT:
        _select("")


## 切换选中态：重绘候选卡、换浮层内容并刷新按钮可用性。
func _select(hero_id: String) -> void:
    _selected_hero = hero_id
    _render_candidates()
    _show_skill(hero_id)
    _sync_state()


func _show_skill(hero_id: String) -> void:
    UI.clear(_skill_panel)
    _dismiss.visible = hero_id != ""
    if hero_id == "":
        _skill_layer.visible = false
        return
    var title := UI.label(CardDB.display_name(hero_id), UITheme.font_size("font-title"), false)
    _skill_panel.add_child(title)
    var skill := CardDB.display_text(hero_id)
    if skill != "":
        var body := UI.label(skill, UITheme.font_size("font-body"))
        body.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
        _skill_panel.add_child(body)
    _skill_layer.visible = true


func _is_confirmed() -> bool:
    return GameStore.my_ready()


func _sync_state() -> void:
    var confirmed := _is_confirmed()
    var candidates := GameStore.my_hero_candidates()
    var connected := Net.is_open()
    _confirm.disabled = _selected_hero == "" or confirmed or not connected
    _confirm.text = tr("ui.room.confirmed") if confirmed else tr("ui.room.confirm")

    if confirmed:
        _status.text = tr("ui.room.confirmed_hero") % CardDB.display_name(_selected_hero)
    elif not connected:
        _status.text = tr("ui.room.not_connected")
    elif candidates.is_empty():
        _status.text = tr("ui.room.assigning_hero")
    else:
        _status.text = tr("ui.room.candidate_hint") % candidates.size()


func _on_confirm_pressed() -> void:
    if _selected_hero == "":
        return
    Net.send(Protocol.MSG_SELECT_HERO_CARD, {"cardId": _selected_hero})
    Net.send(Protocol.MSG_READY, {"ready": true})
    _status.text = tr("ui.room.submitted_hero") % CardDB.display_name(_selected_hero)
    _confirm.disabled = true


func _on_room_changed() -> void:
    if GameStore.phase != "" and GameStore.phase != Protocol.PHASE_LOBBY and not GameStore.view.is_empty():
        _goto_game()
    else:
        _refresh()


func _on_game_started() -> void:
    _goto_game()


func _on_view_changed() -> void:
    # 重连或晚到的视图：只要对局已经开始，就直接进入对局。
    if not GameStore.view.is_empty() and GameStore.phase != Protocol.PHASE_LOBBY:
        _goto_game()


func _goto_game() -> void:
    UI.navigate(self, "res://scenes/game.tscn")


func _error_route(code: String) -> String:
    return UI.error_route(code)


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    match _error_route(code):
        "lobby":
            GameStore.reset_room()
            UI.navigate(self, "res://scenes/lobby.tscn")
            return
        "login":
            GameStore.reset_session()
            UI.navigate(self, "res://scenes/login.tscn")
            return
    _status.text = UI.error_text(code)
    _status.tooltip_text = UI.error_tooltip(code)
    # 请求被拒时没有新的 RoomUpdate：按当前状态重算按钮，否则会一直停在禁用。
    _sync_state()
