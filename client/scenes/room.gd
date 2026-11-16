extends Control
## 选英雄页：顶部只显示其他玩家的名字与确认状态，底部是本人候选英雄卡。
##
## 本页不显示房间码，也没有离开房间入口；点卡选中、点确认一次完成「选英雄 + 准备」。
## 候选由服务端分配且只发给本人，因此这里从不展示别人的英雄。

const CARD_SIZE := Vector2(160, 200)

var _others: HBoxContainer
var _candidates: HFlowContainer
var _confirm: Button
var _status: Label
var _skill_layer: Control
var _skill_panel: VBoxContainer

var _selected_hero := ""
var _card_buttons: Dictionary = {}


func _ready() -> void:
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.game_started.connect(_on_game_started)
    GameStore.view_changed.connect(_on_view_changed)
    GameStore.error_received.connect(_on_error)
    _refresh()


func _build() -> void:
    var background := ColorRect.new()
    background.color = UITheme.color("color-bg")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(background)

    var root := VBoxContainer.new()
    root.add_theme_constant_override("separation", UITheme.space("space-md"))
    var margin := UI.margin(root, UITheme.space("space-lg"))
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(margin)

    _others = HBoxContainer.new()
    _others.name = "OtherPlayers"
    _others.add_theme_constant_override("separation", UITheme.space("space-md"))
    root.add_child(_others)

    var spacer := Control.new()
    spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
    root.add_child(spacer)

    root.add_child(UI.section_title("请选择英雄"))

    _candidates = HFlowContainer.new()
    _candidates.name = "HeroCandidates"
    _candidates.add_theme_constant_override("h_separation", UITheme.space("space-md"))
    _candidates.add_theme_constant_override("v_separation", UITheme.space("space-md"))
    root.add_child(_candidates)

    _confirm = UI.styled_button("确认", "primary", 36)
    _confirm.name = "ConfirmButton"
    _confirm.pressed.connect(_on_confirm_pressed)
    root.add_child(_confirm)

    _status = UI.label("", UITheme.font_size("font-body-sm"))
    _status.name = "StatusLabel"
    root.add_child(_status)

    _build_skill_layer()


func _build_skill_layer() -> void:
    _skill_layer = Control.new()
    _skill_layer.name = "SkillLayer"
    _skill_layer.set_anchors_preset(Control.PRESET_FULL_RECT)
    _skill_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _skill_layer.visible = false
    add_child(_skill_layer)

    var panel := UI.card_panel("raised")
    panel.set_anchors_preset(Control.PRESET_CENTER)
    panel.custom_minimum_size = Vector2(320, 0)
    _skill_layer.add_child(panel)

    _skill_panel = VBoxContainer.new()
    _skill_panel.add_theme_constant_override("separation", UITheme.space("space-sm"))
    panel.add_child(_skill_panel)


func _refresh() -> void:
    # 重连或晚到的 RoomUpdate：已确认时用服务端记下的英雄恢复选中态，再重绘。
    if _selected_hero == "" and _is_confirmed():
        _selected_hero = GameStore.my_hero_id()
    _render_others()
    _render_candidates()
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
        var text := "%s · %s" % [display, "已确认" if ready else "未确认"]
        var row := UI.pill_label(text, "success" if ready else "neutral")
        row.name = "OtherPlayer%d" % shown
        _others.add_child(row)
        shown += 1
    if shown == 0:
        _others.add_child(UI.empty_hint("等待其他玩家加入"))


## 底部：本人候选卡。选中态加描边，已确认后全部禁用。
func _render_candidates() -> void:
    UI.clear(_candidates)
    _card_buttons.clear()

    var candidates := GameStore.my_hero_candidates()
    if candidates.is_empty():
        _candidates.add_child(UI.empty_hint("正在分配英雄…"))
        return

    var locked := _is_confirmed()
    for hero_id in candidates:
        var id := str(hero_id)
        var card := _build_card(id, locked)
        _candidates.add_child(card)
        _card_buttons[id] = card


## 候选卡：部落色封面铺满、居中首字水印、底部居中名字。
func _build_card(hero_id: String, locked: bool) -> PanelContainer:
    var entry := CardDB.card(hero_id)
    var tribe := str(entry.get("tribe", ""))
    var card := PanelContainer.new()
    card.name = "HeroCard_%s" % hero_id
    card.custom_minimum_size = CARD_SIZE
    card.mouse_filter = Control.MOUSE_FILTER_STOP

    var box := UITheme.panel_style("raised")
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
        _selected_hero = ""
    else:
        _selected_hero = hero_id
    _render_candidates()
    _show_skill(hero_id if _selected_hero == hero_id else "")
    _sync_state()


func _show_skill(hero_id: String) -> void:
    UI.clear(_skill_panel)
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
    _confirm.text = "已确认" if confirmed else "确认"

    if confirmed:
        _status.text = "已确认英雄：%s" % CardDB.display_name(_selected_hero)
    elif not connected:
        _status.text = "未连接服务器"
    elif candidates.is_empty():
        _status.text = "正在分配英雄…"
    else:
        _status.text = "从你的 %d 张候选里选一张，确认后不可更改。" % candidates.size()


func _on_confirm_pressed() -> void:
    if _selected_hero == "":
        return
    Net.send(Protocol.MSG_SELECT_HERO_CARD, {"cardId": _selected_hero})
    Net.send(Protocol.MSG_READY, {"ready": true})
    _status.text = "已提交英雄：%s" % CardDB.display_name(_selected_hero)
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
    _status.text = UI.error_text(code)
    _status.tooltip_text = str(GameStore.last_error.get("message", ""))
    match _error_route(code):
        "lobby":
            GameStore.reset_room()
            UI.navigate(self, "res://scenes/lobby.tscn")
        "login":
            GameStore.reset_session()
            UI.navigate(self, "res://scenes/login.tscn")
