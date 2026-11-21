extends "res://tests/smoke_case.gd"
## 房间页管理操作冒烟：房主可见移除/解散，非房主不可见，RoomClosed 提示并回大厅。


func _ready() -> void:
    await _check_host_controls()
    await _check_non_host_controls()
    await _check_room_closed_navigation()
    await _check_offline_clicks()
    _finish()


## 房主 + 两名成员：每个成员行带移除按钮，底部显示解散按钮。
func _check_host_controls() -> void:
    _seed_room(true)
    GameStore.room_players[0]["heroCandidates"] = ["zhenghe"]
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance == null:
        return

    var hero_card = instance.find_child("HeroCard_zhenghe", true, false)
    _expect(hero_card != null, "本人候选应渲染英雄卡")
    if hero_card != null:
        _expect(
            hero_card.find_child("CoverArt", true, false) != null,
            "候选卡有卡面素材时应显示卡面图",
        )
        _expect(
            hero_card.find_child("Watermark", true, false) == null,
            "有卡面图时不应显示首字水印",
        )
        await _hover(instance, hero_card, hero_card.size * 0.5)
        var detail_panel = instance.find_child("RoomHeroDetail", true, false)
        _expect(detail_panel != null and detail_panel.visible, "悬停应显示详情卡")
        _expect(
            detail_panel != null
            and detail_panel.find_children("*", "TextureRect", true, false).is_empty(),
            "悬停详情不应显示图片",
        )
        await _push_motion(instance, Vector2(4, 4))
        _expect(detail_panel != null and not detail_panel.visible, "移出候选卡应关闭详情")

        await _push_click(instance, hero_card.get_global_rect().get_center())
        var selected_frame = hero_card.find_child("SelectedFrame", true, false)
        _expect(selected_frame != null and selected_frame.is_visible_in_tree(), "点击候选卡应显示选中框")
        if selected_frame != null:
            var frame_style := selected_frame.get_theme_stylebox("panel") as StyleBoxFlat
            _expect(
                frame_style != null
                and frame_style.corner_radius_top_left == 0
                and frame_style.corner_radius_top_right == 0
                and frame_style.corner_radius_bottom_left == 0
                and frame_style.corner_radius_bottom_right == 0,
                "候选卡选中框应是直角",
            )
        var card_style := hero_card.get_theme_stylebox("panel") as StyleBoxFlat
        _expect(
            card_style != null and card_style.corner_radius_top_left == 0,
            "候选卡底板应是直角",
        )

    var dissolve = instance.find_child("DissolveButton", true, false)
    _expect(dissolve != null and dissolve.is_visible_in_tree(), "房主应看到解散房间按钮")

    for player_id in ["p1", "p2"]:
        var kick = instance.find_child("Kick_%s" % player_id, true, false)
        _expect(kick != null and kick.is_visible_in_tree(), "房主应看到成员 %s 的移除按钮" % player_id)
        if kick != null:
            _expect(
                str(kick.text) == TranslationServer.translate("ui.room.kick"),
                "移除按钮文案应为「移除」，实际：%s" % str(kick.text),
            )

    # 成员行仍保留名字与确认状态 pill。
    var others := instance.find_child("OtherPlayers", true, false)
    var others_text := "" if others == null else _node_text(others)
    _expect(others_text.contains("玩家B"), "成员行应显示名字，实际：%s" % others_text)
    _expect(
        others_text.contains(TranslationServer.translate("ui.room.confirmed")),
        "成员行应显示确认状态，实际：%s" % others_text,
    )

    instance.queue_free()
    await get_tree().process_frame
    _reset_room_state()


## 非房主：移除与解散按钮都不可见。
func _check_non_host_controls() -> void:
    _seed_room(false)
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance == null:
        return

    var dissolve = instance.find_child("DissolveButton", true, false)
    _expect(dissolve != null and not dissolve.is_visible_in_tree(), "非房主不应看到解散房间按钮")
    for player_id in ["p1", "p2"]:
        _expect(
            instance.find_child("Kick_%s" % player_id, true, false) == null,
            "非房主不应看到成员 %s 的移除按钮" % player_id,
        )

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
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "ready": false, "isHost": is_host,
         "heroCardId": null, "heroCandidates": []},
        {"playerId": "p1", "displayName": "玩家B", "ready": true, "isHost": not is_host,
         "heroCardId": null, "heroCandidates": null},
        {"playerId": "p2", "displayName": "玩家C", "ready": false, "isHost": false,
         "heroCardId": null, "heroCandidates": null},
    ]


func _reset_room_state() -> void:
    _fake_disconnect()
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.phase = ""
    GameStore.room_players = []
    GameStore.view = {}
    GameStore.room_closed_reason = ""
