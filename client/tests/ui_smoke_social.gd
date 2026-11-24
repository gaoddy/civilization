extends "res://tests/smoke_case.gd"
## 好友 UI 冒烟：大厅好友区、邀请弹窗与房间等待页的邀请入口。


func _ready() -> void:
    await _check_friends_button()
    await _check_lobby_social()
    await _check_room_invites()
    _finish()


## 好友入口只显示在线好友数，点击转发打开请求。
func _check_friends_button() -> void:
    var button := FriendsButton.new()
    add_child(button)
    button.set_online_count(1)
    await get_tree().process_frame
    _expect(button.text == "1", "好友入口只显示在线好友数")
    _expect(
        button.custom_minimum_size.x >= UITheme.stage_metric("lobby-friends-button-width"),
        "好友入口宽度取大厅令牌"
    )
    var opened := [false]
    button.open_requested.connect(func(): opened[0] = true)
    button.pressed.emit()
    _expect(opened[0], "点击好友入口应请求打开好友面板")
    button.queue_free()


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

    # 好友管理收在抽屉里：主页只留入口，入口显示在线好友数。
    var entry := instance.find_child("FriendsButton", true, false)
    _expect(entry != null, "大厅应有好友入口")
    _expect(entry != null and str(entry.text) == "1", "好友入口应展示在线好友数")
    var panel = instance.find_child("FriendsPanel", true, false)
    _expect(panel != null and not panel.visible, "好友浮层初始应隐藏")
    if entry == null or panel == null:
        instance.queue_free()
        await get_tree().process_frame
        return

    entry.pressed.emit()
    await get_tree().process_frame
    _expect(panel.visible, "点击好友入口应打开浮层")
    _expect(panel.placement == PopupLayer.Placement.EDGE_LEFT, "好友面板应贴左边")
    _expect(int(panel.content.size.y) == int(panel.size.y), "好友面板应占满整高")
    _expect(
        panel.content.size.x < float(UITheme.metric("lobby-friends-drawer-width")),
        "好友抽屉宽度应贴合内容，未达令牌上限时不占满宽度"
    )

    # 邀请模式：在线好友可邀请，对局中的好友禁用。
    panel.open_invite()
    await get_tree().process_frame
    var invite_online = instance.find_child("Invite_p_online", true, false)
    var invite_game = instance.find_child("Invite_p_game", true, false)
    _expect(invite_online != null and not invite_online.disabled, "在线好友应可邀请")
    _expect(invite_game != null and invite_game.disabled, "对局中的好友应禁用邀请")
    panel.open()
    await get_tree().process_frame
    _expect(
        instance.find_child("Invite_p_online", true, false) == null,
        "普通模式不应残留邀请按钮"
    )

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
            "seat": 0,
            "ready": false,
            "isHost": true,
        }
    ]
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance == null:
        return
    await get_tree().process_frame

    var online = instance.find_child("Invite_p_online", true, false)
    var busy = instance.find_child("Invite_p_game", true, false)
    _expect(online != null, "房间等待页应列出在线好友")
    _expect(online != null and not online.disabled, "在线好友应可邀请")
    _expect(busy != null, "房间等待页应列出对局中的好友")
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
