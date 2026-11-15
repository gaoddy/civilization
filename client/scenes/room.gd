extends Control
## 房间：成员、准备与选英雄；开局完全由服务端决定。

var _status: Label
var _players: VBoxContainer
var _hero_picker: OptionButton
var _ready_button: Button
var _code_label: Label


func _ready() -> void:
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.game_started.connect(_on_game_started)
    GameStore.view_changed.connect(_on_view_changed)
    GameStore.error_received.connect(_on_error)
    _refresh()


func _build() -> void:
    var background := ColorRect.new()
    background.color = Color("10141c")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(background)

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, 24)
    add_child(margin)

    var root := VBoxContainer.new()
    root.add_theme_constant_override("separation", 12)
    margin.add_child(root)

    var top := HBoxContainer.new()
    _code_label = UI.label("房间", 24)
    top.add_child(_code_label)
    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    top.add_child(spacer)
    var leave := UI.button("离开房间")
    leave.pressed.connect(_on_leave_pressed)
    top.add_child(leave)
    root.add_child(top)
    root.add_child(HSeparator.new())

    var body := HBoxContainer.new()
    body.size_flags_vertical = Control.SIZE_EXPAND_FILL
    body.add_theme_constant_override("separation", 24)
    root.add_child(body)

    var left := VBoxContainer.new()
    left.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    left.add_child(UI.label("成员", 18))
    _players = VBoxContainer.new()
    _players.name = "PlayerList"
    _players.add_theme_constant_override("separation", 6)
    left.add_child(_players)
    body.add_child(left)

    var right := VBoxContainer.new()
    right.custom_minimum_size = Vector2(320, 0)
    right.add_theme_constant_override("separation", 10)
    right.add_child(UI.label("英雄", 18))
    _hero_picker = OptionButton.new()
    right.add_child(_hero_picker)
    _populate_heroes()
    var pick := UI.button("选为英雄")
    pick.pressed.connect(_on_pick_hero)
    right.add_child(pick)
    _ready_button = UI.button("准备")
    _ready_button.pressed.connect(_on_ready_pressed)
    right.add_child(_ready_button)
    body.add_child(right)

    _status = UI.label("", 14)
    root.add_child(_status)


func _populate_heroes() -> void:
    var index := 0
    for hero in CardDB.heroes():
        var id := str(hero.get("id", ""))
        var label := "%s · %s" % [CardDB.display_name(id), str(hero.get("tribe", ""))]
        _hero_picker.add_item(label, index)
        _hero_picker.set_item_metadata(index, id)
        index += 1


func _refresh() -> void:
    if GameStore.room_id == "":
        _code_label.text = "未加入房间"
        _status.text = "请返回大厅创建或加入房间。"
        _ready_button.disabled = true
        return

    _code_label.text = "房间码：%s" % GameStore.room_code
    _ready_button.disabled = false

    UI.clear(_players)
    var my_hero := ""
    var my_ready := false
    for player in GameStore.room_players:
        var player_id := str(player.get("playerId", ""))
        var hero_id := _optional_text(player.get("heroCardId"))
        var is_host := bool(player.get("isHost", false))
        var is_ready := bool(player.get("ready", false))
        var hero_name := CardDB.display_name(hero_id) if hero_id != "" else "未选英雄"
        var marks := []
        if is_host:
            marks.append("房主")
        if player_id == GameStore.player_id:
            marks.append("我")
        var display := _optional_text(player.get("displayName"))
        if display == "":
            display = player_id
        var suffix := "（%s）" % _join_cn(marks) if not marks.is_empty() else ""
        var line := "%s%s  %s  %s" % [
            display,
            suffix,
            hero_name,
            "已准备" if is_ready else "未准备",
        ]
        var row := UI.label(line, 16)
        if is_ready:
            row.modulate = Color("8fe388")
        _players.add_child(row)
        if player_id == GameStore.player_id:
            my_hero = hero_id
            my_ready = is_ready

    _ready_button.text = "取消准备" if my_ready else "准备"
    _sync_hero_picker(my_hero)
    _status.text = "已加入 %d/8 人；全部选好英雄并准备后由服务端自动开局。" % GameStore.room_players.size()


func _sync_hero_picker(hero_id: String) -> void:
    if hero_id == "":
        return
    for index in range(_hero_picker.item_count):
        if str(_hero_picker.get_item_metadata(index)) == hero_id:
            _hero_picker.select(index)
            return


func _on_pick_hero() -> void:
    var index := _hero_picker.selected
    if index < 0:
        return
    var card_id := str(_hero_picker.get_item_metadata(index))
    Net.send(Protocol.MSG_SELECT_HERO_CARD, {"cardId": card_id})
    _status.text = "已提交英雄：%s" % CardDB.display_name(card_id)


func _on_ready_pressed() -> void:
    var my_ready := false
    for player in GameStore.room_players:
        if str(player.get("playerId", "")) == GameStore.player_id:
            my_ready = bool(player.get("ready", false))
    Net.send(Protocol.MSG_READY, {"ready": not my_ready})


func _on_leave_pressed() -> void:
    Net.send(Protocol.MSG_LEAVE_ROOM, {})
    GameStore.reset_room()
    UI.navigate(self, "res://scenes/lobby.tscn")


func _on_room_changed() -> void:
    if GameStore.phase != Protocol.PHASE_LOBBY and not GameStore.view.is_empty():
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


func _error_route(code: String) -> String:
    return UI.error_route(code)


func _optional_text(value: Variant) -> String:
    if value == null:
        return ""
    return str(value)


func _join_cn(values: Array) -> String:
    var parts := PackedStringArray()
    for value in values:
        parts.append(str(value))
    return "、".join(parts)
