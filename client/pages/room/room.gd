extends Control
## 选英雄页：顶部只显示其他玩家的名字与确认状态，底部是本人候选英雄卡。
##
## 本页不显示房间码，也没有离开房间入口；点卡选中、点确认一次完成「选英雄 + 准备」。
## 候选由服务端分配且只发给本人，因此这里从不展示别人的英雄。
## 鼠标悬停候选卡时在悬停卡外侧显示详情，移开关闭；点击只切换选中框。

var _others: HBoxContainer
var _candidates: RoomCandidateList
var _actions: RoomActionBar
var _invite: RoomInviteRow
var _detail_layer: RoomHeroDetailLayer

var _selected_hero := ""
## 最近一次鼠标事件的画布坐标；候选重建后据此判断指针是否仍停在原卡上。
var _pointer := Vector2.ZERO

## 触控长按状态：与详情层共用同一套阈值与计时。
var _touch := LongPressTracker.new()


func _ready() -> void:
    UILayout.safe_area(self)
    _build()
    GameStore.room_changed.connect(_on_room_changed)
    GameStore.game_started.connect(_on_game_started)
    GameStore.view_changed.connect(_on_view_changed)
    GameStore.error_received.connect(_on_error)
    GameStore.social_changed.connect(_render_invites)
    GameStore.invite_result_received.connect(_on_invite_result)
    GameStore.room_closed.connect(_on_room_closed)
    # 断线不产生房间或视图消息：直接订阅连接状态，否则「未连接」永远不刷新。
    Net.connected.connect(_sync_state)
    Net.disconnected.connect(_sync_state)
    _refresh()


func _build() -> void:
    add_child(UIControls.background())

    # 面板铺满整屏、尺寸不随内容变化；内容块在面板内水平与垂直居中。
    var panel := UIControls.card_panel("surface")
    panel.name = "DraftPanel"
    panel.set_anchors_preset(Control.PRESET_FULL_RECT)
    # 面板只做视觉容器：空白处的点击由页面 `_input` 处理，候选卡仍优先命中。
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(panel)

    var center := CenterContainer.new()
    center.name = "ContentCenter"
    center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    panel.add_child(center)

    var root := VBoxContainer.new()
    root.name = "DraftColumn"
    root.add_theme_constant_override("separation", UITheme.space("space-md"))
    # 页面容器一律放行鼠标：空白处的点击落到页面 `_input`，候选卡仍优先接收悬停与点击。
    root.mouse_filter = Control.MOUSE_FILTER_IGNORE
    center.add_child(root)

    _others = HBoxContainer.new()
    _others.name = "OtherPlayers"
    _others.alignment = BoxContainer.ALIGNMENT_CENTER
    _others.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _others.add_theme_constant_override("separation", UITheme.space("space-md"))
    root.add_child(_others)

    var title := UIControls.section_title(tr("ui.room.select_hero"))
    title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    title.mouse_filter = Control.MOUSE_FILTER_IGNORE
    root.add_child(title)

    _candidates = RoomCandidateList.new()
    root.add_child(_candidates)

    _invite = RoomInviteRow.new()
    _invite.invite_pressed.connect(_on_invite_pressed)
    root.add_child(_invite)

    _actions = RoomActionBar.new()
    _actions.confirm_pressed.connect(_on_confirm_pressed)
    _actions.dissolve_pressed.connect(_on_dissolve_pressed)
    root.add_child(_actions)

    # 悬停详情层位于内容之上但整体忽略鼠标：详情不得挡住候选卡与确认按钮。
    _detail_layer = RoomHeroDetailLayer.new()
    add_child(_detail_layer)
    _detail_layer.row = _candidates


func _refresh() -> void:
    # 重连或晚到的 RoomUpdate：以服务端的已选英雄为准，避免多端不一致。
    if _is_confirmed():
        _selected_hero = GameStore.my_hero_id()
    _render_others()
    _render_candidates()
    _render_invites()
    _sync_state()


## 顶部：其他玩家的名字与确认状态；不展示任何人的英雄选择。
## 房主在等待区时，每名成员行追加移除按钮。
func _render_others() -> void:
    UIControls.clear(_others)
    var shown := 0
    var can_manage := _is_host() and GameStore.phase == Protocol.PHASE_LOBBY
    for player in GameStore.room_players:
        if str(player.get("playerId", "")) == GameStore.player_id:
            continue
        var row := RoomPlayerRow.new(player, can_manage)
        row.name = "OtherPlayer%d" % shown
        row.kicked.connect(_on_kick_pressed)
        _others.add_child(row)
        shown += 1
    if shown == 0:
        _others.add_child(UIControls.empty_hint(tr("ui.room.waiting_for_players")))


## 底部：本人候选卡。选中态用外扩边框，已确认后全部锁定但悬停仍可看详情。
func _render_candidates() -> void:
    var ids: Array[String] = []
    for hero_id in GameStore.my_hero_candidates():
        ids.append(str(hero_id))
    var locked := _is_confirmed()

    if ids.is_empty():
        if _candidates.show_empty():
            _detail_layer.reset()
            _clear_touch()
        return

    if not _candidates.changed(ids):
        _candidates.apply_state(_selected_hero, locked)
        return

    var hovered := _detail_layer.visible_hero_id()
    _detail_layer.reset()
    _clear_touch()
    _candidates.rebuild(ids, _selected_hero, locked)
    for card in _candidates.cards():
        _detail_layer.track(card, {"detail_kind": "hero", "heroId": card.hero_id})
    # 容器要到下一帧才排好新卡位置；恢复悬停必须等布局完成，否则命中用的是旧矩形。
    if hovered != "" and ids.has(hovered):
        _restore_hover.call_deferred(hovered)


## 候选重建后，若指针仍停在原英雄卡上则恢复悬停详情。
func _restore_hover(hero_id: String) -> void:
    if ScreenProfile.is_touch():
        return
    _candidates.sync_hover(_pointer)
    var card := _candidates.find_card(hero_id)
    if card != null and card.get_global_rect().has_point(_pointer):
        _detail_layer.show_for(card, hero_id)


## 指针所在卡片显示悬停描边：重建后新卡收不到 `mouse_entered`，由页面按指针位置统一同步。
func _sync_hover_styles() -> void:
    _candidates.sync_hover(_pointer)


## 切换选中：只更新选中框，不重建候选卡；再次点击同一张保持选中。
func _select(hero_id: String) -> void:
    if _is_confirmed():
        return
    _selected_hero = hero_id
    _candidates.apply_state(hero_id, false)
    _sync_state()


## 桌面左键点击候选卡选中；触控端短按选中，长按交给详情层。
func _input(event: InputEvent) -> void:
    if ScreenProfile.is_touch():
        if event is InputEventScreenTouch:
            if event.pressed:
                _begin_touch(event)
            else:
                _end_touch(event)
        elif event is InputEventScreenDrag:
            _drag_touch(event)
        return
    if event is InputEventMouseButton:
        var mouse := event as InputEventMouseButton
        _pointer = _canvas_point(mouse.position)
        if mouse.pressed and mouse.button_index == MOUSE_BUTTON_LEFT:
            var card := _candidates.card_at(_pointer)
            if card != null:
                _select(card.hero_id)
    elif event is InputEventMouseMotion:
        _pointer = _canvas_point((event as InputEventMouseMotion).position)
        _sync_hover_styles()


## 触控长按计时与详情层同源：共用 LongPressTracker，按帧 delta 累加。
func _process(delta: float) -> void:
    if not _touch.active() or _touch.cancelled:
        return
    _touch.advance(delta)


func _begin_touch(event: InputEventScreenTouch) -> void:
    if _touch.active() or _is_confirmed():
        return
    var point := _canvas_point(event.position)
    var card := _candidates.card_at(point)
    if card == null:
        return
    _touch.begin(event.index, point, card)


func _drag_touch(event: InputEventScreenDrag) -> void:
    if not _touch.matches(event.index):
        return
    _touch.dragged(event.index, _canvas_point(event.position))


func _end_touch(event: InputEventScreenTouch) -> void:
    if not _touch.matches(event.index):
        return
    var card := _touch.target
    var cancelled := _touch.cancelled
    var elapsed := _touch.elapsed_ms
    _clear_touch()
    if card == null or not is_instance_valid(card) or cancelled or elapsed >= _touch.long_press_ms:
        return
    if _is_confirmed() or card.locked:
        return
    if not card.get_global_rect().has_point(_canvas_point(event.position)):
        return
    _select(card.hero_id)


func _clear_touch() -> void:
    _touch.cancel()


## 视口坐标转为画布坐标，与详情层的 `_input` 命中同一坐标系。
func _canvas_point(viewport_point: Vector2) -> Vector2:
    return get_viewport().get_canvas_transform().affine_inverse() * viewport_point


## 好友邀请入口的渲染由 RoomInviteRow 负责。
func _render_invites() -> void:
    if _invite == null:
        return
    _invite.render(GameStore.friends)


func _on_invite_pressed(player_id: String, friend_name: String) -> void:
    Net.send(Protocol.MSG_INVITE_TO_ROOM, {"friendId": player_id})
    _actions.status.text = tr("ui.room.invite_sent") % friend_name


func _on_invite_result() -> void:
    if bool(GameStore.last_invite_result.get("accepted", false)):
        _actions.status.text = tr("ui.room.invite_accepted")
    else:
        _actions.status.text = tr("ui.room.invite_declined")


func _is_confirmed() -> bool:
    return GameStore.my_ready()


## 本人是否为房主：以服务端下发的座位字段为准。
func _is_host() -> bool:
    for player in GameStore.room_players:
        if str(player.get("playerId", "")) == GameStore.player_id:
            return bool(player.get("isHost", false))
    return false


func _sync_state() -> void:
    var confirmed := _is_confirmed()
    var candidates := GameStore.my_hero_candidates()
    var connected := Net.is_open()
    _actions.set_state(_selected_hero, confirmed, connected, _is_host())

    if confirmed:
        _actions.status.text = tr("ui.room.confirmed_hero") % CardDB.display_name(_selected_hero)
    elif not connected:
        _actions.status.text = tr("ui.room.not_connected")
    elif candidates.is_empty():
        _actions.status.text = tr("ui.room.assigning_hero")
    else:
        _actions.status.text = tr("ui.room.candidate_hint") % candidates.size()


func _on_confirm_pressed() -> void:
    if _selected_hero == "":
        return
    Net.send(Protocol.MSG_SELECT_HERO_CARD, {"cardId": _selected_hero})
    Net.send(Protocol.MSG_READY, {"ready": true})
    _actions.status.text = tr("ui.room.submitted_hero") % CardDB.display_name(_selected_hero)
    _actions.confirm.disabled = true


func _on_kick_pressed(player_id: String) -> void:
    Net.send(Protocol.MSG_KICK_MEMBER, {"targetPlayerId": player_id})


func _on_dissolve_pressed() -> void:
    Net.send(Protocol.MSG_DISSOLVE_ROOM, {})


## 被移出或房间解散：先写原因文案，再回大厅；房间状态已由 GameStore 清空。
func _on_room_closed() -> void:
    if GameStore.room_closed_reason == "kicked":
        _actions.status.text = tr("ui.room.kicked_notice")
    else:
        _actions.status.text = tr("ui.room.dissolved_notice")
    UINavigation.navigate(self, UINavigation.PAGE_LOBBY)


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
    UINavigation.navigate(self, UINavigation.PAGE_GAME)


func _error_route(code: String) -> String:
    return UINavigation.error_route(code)


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    match _error_route(code):
        UINavigation.ROUTE_LOBBY:
            GameStore.reset_room()
            UINavigation.navigate(self, UINavigation.PAGE_LOBBY)
            return
        UINavigation.ROUTE_LOGIN:
            GameStore.reset_session()
            UINavigation.navigate(self, UINavigation.PAGE_LOGIN)
            return
    _actions.status.text = UIWording.error_text(code)
    _actions.status.tooltip_text = UIWording.error_tooltip(code)
    # 请求被拒时没有新的 RoomUpdate：按当前状态重算按钮，否则会一直停在禁用。
    _sync_state()
