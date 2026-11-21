extends "res://tests/smoke_case.gd"
## 好友 UI 冒烟：大厅好友区、邀请弹窗与选英雄页的邀请入口。


func _ready() -> void:
    await _check_lobby_social()
    await _check_room_invites()
    _finish()


func _seed_social() -> void:
    GameStore.player_id = "p_me"
    GameStore.player_code = "100001"
    GameStore.display_name = "我"
    GameStore.friends = [
        {
            "displayName": "在线好友",
            "playerCode": "100002",
            "playerId": "p_online",
            "status": "online",
        },
        {
            "displayName": "对局好友",
            "playerCode": "100003",
            "playerId": "p_game",
            "status": "inGame",
        },
    ]
    GameStore.incoming_requests = [
        {
            "displayName": "申请人",
            "playerCode": "100004",
            "playerId": "p_request",
            "status": "pending",
        }
    ]
    GameStore.outgoing_requests = []
    GameStore.recent_players = [
        {"displayName": "前队友", "playerCode": "100005", "playerId": "p_recent"},
    ]
    GameStore.pending_invites = [
        {
            "inviteId": "inv_1",
            "fromDisplayName": "房主",
            "fromPlayerCode": "100006",
            "fromPlayerId": "p_host",
            "roomCode": "ABC234",
            "expiresAt": "2030-01-01T00:00:00Z",
        }
    ]


func _check_lobby_social() -> void:
    _seed_social()
    var instance := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if instance == null:
        return
    await get_tree().process_frame

    var labels := _node_text(instance)
    _expect(labels.contains("100001"), "大厅应展示本人玩家编号")
    _expect(labels.contains("ABC234"), "邀请弹窗应展示房间码")

    # 好友管理收在浮层里：主页只留入口，入口显示待处理申请数。
    var entry := instance.find_child("FriendsButton", true, false)
    _expect(entry != null, "大厅应有好友入口")
    _expect(entry != null and str(entry.text).contains("1"), "好友入口应展示待处理申请数")
    var panel = instance.find_child("FriendsPanel", true, false)
    _expect(panel != null and not panel.visible, "好友浮层初始应隐藏")
    if entry == null or panel == null:
        instance.queue_free()
        await get_tree().process_frame
        return

    entry.pressed.emit()
    await get_tree().process_frame
    _expect(panel.visible, "点击好友入口应打开浮层")

    labels = _node_text(instance)
    _expect(labels.contains("在线好友"), "好友浮层应列出好友")
    _expect(labels.contains("对局中"), "好友行应展示服务端状态")
    _expect(labels.contains("申请人"), "好友浮层应列出收到的申请")
    _expect(
        instance.find_child("FriendCodeInput", true, false) != null,
        "好友浮层应提供添加好友输入框"
    )
    _expect(
        instance.find_child("AddFriendButton", true, false) != null,
        "好友浮层应提供添加好友按钮"
    )
    _expect(
        instance.find_child("Accept_p_request", true, false) != null,
        "收到的申请应有同意按钮"
    )
    _expect(
        instance.find_child("Decline_p_request", true, false) != null,
        "收到的申请应有拒绝按钮"
    )
    _expect(
        instance.find_child("Remove_p_online", true, false) != null,
        "好友行应有删除按钮"
    )
    _expect(
        instance.find_child("AddRecent_p_recent", true, false) != null,
        "最近队友应可回加"
    )

    var popup = instance.find_child("InvitePopup", true, false)
    _expect(popup != null and popup.visible, "有待处理邀请时应显示邀请弹窗")
    _expect(
        instance.find_child("AcceptInvite_inv_1", true, false) != null,
        "邀请弹窗应提供接受按钮"
    )
    _expect(
        instance.find_child("DeclineInvite_inv_1", true, false) != null,
        "邀请弹窗应提供拒绝按钮"
    )

    # 关闭按钮与 Esc 都应收起浮层，且不留下遮罩挡住大厅。
    var close := instance.find_child("FriendsClose", true, false)
    _expect(close != null, "好友浮层应有关闭按钮")
    if close != null:
        close.pressed.emit()
        await get_tree().process_frame
        _expect(not panel.visible, "点关闭后好友浮层应隐藏")
    entry.pressed.emit()
    await get_tree().process_frame
    _expect(panel.visible, "再次点击好友入口应重新打开浮层")
    var cancel := InputEventAction.new()
    cancel.action = "ui_cancel"
    cancel.pressed = true
    instance.get_viewport().push_input(cancel)
    await get_tree().process_frame
    _expect(not panel.visible, "Esc 应收起好友浮层")

    # 清空邀请后弹窗应隐藏，避免留下遮罩挡住大厅。
    GameStore.pending_invites = []
    GameStore.social_changed.emit()
    await get_tree().process_frame
    _expect(not popup.visible, "没有邀请时弹窗应隐藏")
    instance.queue_free()
    await get_tree().process_frame


func _check_room_invites() -> void:
    _seed_social()
    GameStore.pending_invites = []
    GameStore.room_players = [
        {
            "displayName": "我",
            "playerId": "p_me",
            "ready": false,
            "heroCandidates": ["adam-smith"],
        }
    ]
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance == null:
        return
    await get_tree().process_frame

    var online = instance.find_child("Invite_p_online", true, false)
    var busy = instance.find_child("Invite_p_game", true, false)
    _expect(online != null, "选英雄页应列出在线好友")
    _expect(online != null and not online.disabled, "在线好友应可邀请")
    _expect(busy != null, "选英雄页应列出对局中的好友")
    _expect(busy != null and busy.disabled, "对局中的好友应禁用邀请")

    GameStore.friends = []
    GameStore.social_changed.emit()
    await get_tree().process_frame
    _expect(
        instance.find_child("Invite_p_online", true, false) == null,
        "没有好友时应清空邀请入口"
    )
    instance.queue_free()
    await get_tree().process_frame
