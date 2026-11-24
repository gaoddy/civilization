extends "res://tests/smoke_case.gd"
## 房间页管理操作冒烟：房主可见移除/解散/开始/补位，非房主不可见，RoomClosed 提示并回大厅。


func _ready() -> void:
    await _check_host_controls()
    await _check_non_host_controls()
    await _check_room_closed_navigation()
    await _check_offline_clicks()
    _finish()


## 房主 + 两名成员：座位格带徽标，成员座位有移除按钮，底部显示解散/开始/补位。
func _check_host_controls() -> void:
    _seed_room(true)
    _fake_connect()
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance == null:
        return

    var grid = instance.find_child("RoomSeatGrid", true, false)
    _expect(grid != null and grid.get_child_count() == 8, "房主视图应渲染 8 个座位格")
    if grid != null:
        var host_cell = grid.get_child(0)
        var ready_cell = grid.get_child(1)
        var unready_cell = grid.get_child(2)
        _expect(
            _node_text(host_cell).contains(TranslationServer.translate("ui.room.host")),
            "房主座位应有房主徽标：%s" % _node_text(host_cell),
        )
        _expect(
            _node_text(ready_cell).contains(TranslationServer.translate("ui.room.seat_ready")),
            "已准备成员应有已准备徽标：%s" % _node_text(ready_cell),
        )
        _expect(
            _node_text(unready_cell).contains(TranslationServer.translate("ui.room.seat_unready")),
            "未准备成员应有未准备徽标：%s" % _node_text(unready_cell),
        )

    for player_id in ["p1", "p2"]:
        var kick = instance.find_child("Kick_%s" % player_id, true, false)
        _expect(kick != null and kick.is_visible_in_tree(), "房主应看到成员 %s 的移除按钮" % player_id)
        if kick != null:
            _expect(
                str(kick.text) == TranslationServer.translate("ui.room.kick"),
                "移除按钮文案应为「移除」，实际：%s" % str(kick.text),
            )
    _expect(instance.find_child("Kick_p0", true, false) == null, "房主不应看到移除本人的按钮")

    var dissolve = instance.find_child("DissolveButton", true, false)
    _expect(dissolve != null and dissolve.is_visible_in_tree(), "房主应看到解散房间按钮")
    var start = instance.find_child("StartButton", true, false)
    _expect(start != null and start.is_visible_in_tree(), "房主应看到开始按钮")
    var bot_fill = instance.find_child("BotFillButton", true, false)
    _expect(bot_fill != null and bot_fill.is_visible_in_tree(), "房主应看到补位开关")
    _expect(bot_fill != null and not bot_fill.disabled, "已连接时房主补位开关应可点")
    var ready = instance.find_child("ReadyButton", true, false)
    _expect(
        ready != null and str(ready.text) == TranslationServer.translate("ui.room.ready"),
        "本人未准备时应显示「准备」",
    )

    instance.queue_free()
    await get_tree().process_frame
    _reset_room_state()


## 非房主：移除、解散与开始都不可见，补位开关禁用。
func _check_non_host_controls() -> void:
    _seed_room(false)
    _fake_connect()
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance == null:
        return

    var dissolve = instance.find_child("DissolveButton", true, false)
    _expect(
        dissolve == null or not dissolve.is_visible_in_tree(),
        "非房主不应看到解散房间按钮",
    )
    var start = instance.find_child("StartButton", true, false)
    _expect(
        start == null or not start.is_visible_in_tree(),
        "非房主不应看到开始按钮",
    )
    for player_id in ["p1", "p2"]:
        _expect(
            instance.find_child("Kick_%s" % player_id, true, false) == null,
            "非房主不应看到成员 %s 的移除按钮" % player_id,
        )
    var bot_fill = instance.find_child("BotFillButton", true, false)
    _expect(bot_fill != null and bot_fill.disabled, "非房主补位开关应禁用")

    instance.queue_free()
    await get_tree().process_frame
    _reset_room_state()


## RoomClosed(kicked)：先写状态文案，再切到大堂。
func _check_room_closed_navigation() -> void:
    _seed_room(true)
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance == null:
        return
    var status = instance.find_child("StatusLabel", true, false)

    # current_scene 必须是 root 的直接子节点：把房间页挂到 root 后再设为当前场景，
    # change_scene 只替换它，冒烟用例节点仍留在树中继续断言。
    remove_child(instance)
    get_tree().root.add_child(instance)
    get_tree().current_scene = instance
    GameStore.room_closed_reason = "kicked"
    GameStore.room_closed.emit()
    _expect(
        status != null and str(status.text) == TranslationServer.translate("ui.room.kicked_notice"),
        "RoomClosed(kicked) 应显示被移出提示，实际：%s" % ("" if status == null else str(status.text)),
    )

    await get_tree().process_frame
    await get_tree().process_frame
    var current := get_tree().current_scene
    _expect(
        current != null and str(current.scene_file_path) == "res://pages/lobby/lobby.tscn",
        "RoomClosed 后应切换到大堂，实际：%s"
        % ("<null>" if current == null else str(current.scene_file_path)),
    )

    if current != null and current != self:
        current.queue_free()
    get_tree().current_scene = self
    await get_tree().process_frame
    _reset_room_state()


## 无连接时点击移除/解散：Net.send 返回空串且页面不崩溃。
func _check_offline_clicks() -> void:
    _seed_room(true)
    _fake_disconnect()
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance == null:
        return

    var kick = instance.find_child("Kick_p1", true, false)
    var dissolve = instance.find_child("DissolveButton", true, false)
    _expect(kick != null and dissolve != null, "房主应看到移除与解散按钮")
    _expect(
        Net.send(Protocol.MSG_KICK_MEMBER, {"targetPlayerId": "p1"}) == "",
        "未连接时 KickMember 应返回空串",
    )
    _expect(
        Net.send(Protocol.MSG_DISSOLVE_ROOM, {}) == "",
        "未连接时 DissolveRoom 应返回空串",
    )
    if kick != null:
        kick.pressed.emit()
    if dissolve != null:
        dissolve.pressed.emit()
    await get_tree().process_frame
    _expect(is_instance_valid(instance) and instance.is_inside_tree(), "无连接点击管理按钮不应崩溃")

    instance.queue_free()
    await get_tree().process_frame
    _reset_room_state()


func _seed_room(is_host: bool) -> void:
    _fake_disconnect()
    GameStore.player_id = "p0"
    GameStore.room_id = "r_test"
    GameStore.room_code = "ABC123"
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.view = {}
    GameStore.room_closed_reason = ""
    GameStore.room_bot_fill = false
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "seat": 0, "ready": false, "isHost": is_host},
        {"playerId": "p1", "displayName": "玩家B", "seat": 1, "ready": true, "isHost": not is_host},
        {"playerId": "p2", "displayName": "玩家C", "seat": 2, "ready": false, "isHost": false},
    ]


func _reset_room_state() -> void:
    _fake_disconnect()
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.phase = ""
    GameStore.room_players = []
    GameStore.view = {}
    GameStore.room_closed_reason = ""
    GameStore.room_bot_fill = false
