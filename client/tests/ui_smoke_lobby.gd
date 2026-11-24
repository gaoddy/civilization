extends "res://tests/smoke_case.gd"
## 大厅冒烟：两栏比例、卡片、匹配切换、加入校验、断线与错误提示。


## 记录发送的假连接：加入弹窗的非法输入不能产生任何消息。
class RecordingSocket:
    extends RefCounted

    var sent: Array = []

    func get_ready_state() -> int:
        return WebSocketPeer.STATE_OPEN

    func poll() -> void:
        pass

    func get_available_packet_count() -> int:
        return 0

    func send_text(text: String) -> int:
        sent.append(text)
        return OK


func _ready() -> void:
    _seed()
    var socket := RecordingSocket.new()
    Net.socket = socket
    var lobby := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if lobby != null:
        await _check_layout(lobby)
        await _check_match_toggle(lobby)
        await _check_join_dialog(lobby, socket)
        await _check_disconnect(lobby)
        await _check_error(lobby)
        lobby.queue_free()
        await get_tree().process_frame

    await _check_room_mode()
    await _check_room_phone_height()
    ScreenProfile.set_device_for_test("")
    Net.socket = null
    _finish()


func _seed() -> void:
    GameStore.player_id = "p_me"
    GameStore.player_code = "100001"
    GameStore.display_name = "测试玩家"
    GameStore.account = {}
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.room_players = []
    GameStore.room_closed_reason = ""
    GameStore.queueing = false
    GameStore.queue_state = {}
    GameStore.friends = [
        {
            "displayName": "在线好友",
            "playerCode": "100002",
            "playerId": "p_online",
            "status": "online",
        },
    ]


func _check_layout(lobby: Node) -> void:
    var columns := lobby.find_child("LobbyColumns", true, false) as Control
    var left := lobby.find_child("LobbyLeft", true, false) as Control
    var right := lobby.find_child("LobbyRight", true, false) as Control
    _expect(columns != null, "大厅应有两栏容器")
    _expect(left != null and right != null, "大厅应有左右两栏")
    if left != null and right != null:
        var total := left.size.x + right.size.x
        var share := left.size.x / total if total > 0.0 else 0.0
        _expect(absf(share - 0.7) <= 0.02, "大厅左右两栏应约 7:3，实际 %.3f" % share)

    for node_name in [
        "LobbyHallImage",
        "LobbyPlayerCard",
        "LobbyRankCard",
        "MatchButton",
        "CreateRoomButton",
        "JoinRoomButton",
        "FriendsButton",
        "GalleryButton",
        "LeaderboardButton",
    ]:
        _expect(lobby.find_child(node_name, true, false) != null, "大厅缺少 %s" % node_name)

    var tier := lobby.find_child("RankTier", true, false) as Label
    var score := lobby.find_child("RankScore", true, false) as Label
    _expect(tier != null and tier.text == tr("ui.lobby.rank_none"), "无段位时应显示暂无段位")
    _expect(score != null and score.text.contains("0"), "无段位时积分应显示 0")

    var text := _node_text(lobby)
    for child in lobby.find_children("*", "Button", true, false):
        text += str(child.text) + "\n"
    _expect(not text.contains("文明战旗"), "大厅不应出现产品名")
    _expect(not text.contains("大厅"), "大厅不应出现页面标题")

    if columns != null:
        var view: Vector2 = lobby.get_viewport().get_visible_rect().size
        _expect(columns.get_global_rect().end.y <= view.y + 1.0, "桌面右栏底边不应超出视口")


func _check_match_toggle(lobby: Node) -> void:
    var match_button := lobby.find_child("MatchButton", true, false) as Button
    var create_button := lobby.find_child("CreateRoomButton", true, false) as Button
    var join_button := lobby.find_child("JoinRoomButton", true, false) as Button
    var queue_card := lobby.find_child("LobbyQueueCard", true, false) as Control
    var rank_card := lobby.find_child("LobbyRankCard", true, false) as Control
    if match_button == null:
        return

    match_button.pressed.emit()
    await get_tree().process_frame
    _expect(GameStore.queueing, "点开始匹配应进入排队")
    _expect(match_button.text == tr("ui.lobby.cancel_match"), "排队中主按钮应变为取消匹配")
    _expect(queue_card != null and queue_card.visible, "排队中应显示排队卡")
    _expect(rank_card != null and not rank_card.visible, "排队中段位卡应让位")
    _expect(create_button != null and create_button.disabled, "排队中不能创建房间")
    _expect(join_button != null and join_button.disabled, "排队中不能加入房间")

    match_button.pressed.emit()
    await get_tree().process_frame
    _expect(not GameStore.queueing, "再点应取消排队")
    _expect(match_button.text == tr("ui.lobby.start_match"), "取消后主按钮应回到开始匹配")
    _expect(queue_card != null and not queue_card.visible, "取消后应隐藏排队卡")


func _check_join_dialog(lobby: Node, socket: RecordingSocket) -> void:
    var join_button := lobby.find_child("JoinRoomButton", true, false) as Button
    var dialog := lobby.find_child("LobbyJoinDialog", true, false) as Control
    var input := lobby.find_child("JoinCodeInput", true, false) as LineEdit
    var confirm := lobby.find_child("JoinConfirmButton", true, false) as Button
    var error := lobby.find_child("JoinErrorText", true, false) as Label
    var cancel := lobby.find_child("JoinCancelButton", true, false) as Button
    if join_button == null or dialog == null or input == null or confirm == null:
        return

    join_button.pressed.emit()
    await get_tree().process_frame
    _expect(dialog.visible, "点加入房间应打开弹窗")

    input.text = "ABC"
    confirm.pressed.emit()
    await get_tree().process_frame
    _expect(error != null and error.text == tr("ui.lobby.room_code_invalid"), "非法房间码应就地报错")
    _expect(_sent_count(socket, Protocol.MSG_JOIN_ROOM) == 0, "非法房间码不应发送 JoinRoom")

    input.text = "abc123"
    confirm.pressed.emit()
    await get_tree().process_frame
    _expect(_sent_count(socket, Protocol.MSG_JOIN_ROOM) == 1, "合法房间码应发送 JoinRoom")

    if cancel != null:
        cancel.pressed.emit()
        await get_tree().process_frame
    _expect(not dialog.visible, "取消应关闭加入弹窗")


func _check_disconnect(lobby: Node) -> void:
    var dialog := lobby.find_child("ConnectionDialog", true, false)
    var reconnect := lobby.find_child("ReconnectButton", true, false) as Button
    var match_button := lobby.find_child("MatchButton", true, false) as Button
    Net.disconnected.emit()
    await get_tree().process_frame
    _expect(dialog != null and dialog.visible, "断线应显示连接弹窗")
    _expect(reconnect != null and not reconnect.disabled, "断线弹窗重连按钮应可用")
    _expect(match_button != null and match_button.disabled, "断线应禁用快捷操作")

    Net.connected.emit()
    await get_tree().process_frame
    _expect(dialog != null and not dialog.visible, "重连后应收起连接弹窗")
    _expect(match_button != null and not match_button.disabled, "重连后快捷操作应恢复")


func _check_error(lobby: Node) -> void:
    var notice := lobby.find_child("LobbyNoticeDialog", true, false)
    GameStore.last_error = {"code": "ROOM_NOT_FOUND"}
    GameStore.error_received.emit()
    await get_tree().process_frame
    _expect(notice != null and notice.visible, "服务端错误应弹出提示")


func _check_room_mode() -> void:
    _seed_room(3, false)
    var lobby := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if lobby == null:
        return

    var room_list := lobby.find_child("LobbyRoomList", true, false) as Control
    var rank := lobby.find_child("LobbyRankCard", true, false) as Control
    var quick := lobby.find_child("LobbyQuickActions", true, false) as Control
    _expect(room_list != null and room_list.visible, "房主应进入创建模式座位列表")
    _expect(rank != null and not rank.visible, "创建模式应隐藏段位卡")
    _expect(quick != null and not quick.visible, "创建模式应隐藏快捷操作")

    for seat in 8:
        _expect(lobby.find_child("RoomRow%d" % seat, true, false) != null, "座位列表应有第 %d 行" % seat)
    _expect(_room_name_count(lobby) == 3, "3 名玩家应占 3 行座位")
    _expect(lobby.find_child("RoomReady_p0", true, false) != null, "房主行应有准备圆点")
    _expect(lobby.find_child("RoomInviteButton", true, false) != null, "有空位时应显示邀请入口")

    var text := _node_text(lobby)
    for word in ["玩家列表", "已准备", "未准备"]:
        _expect(not text.contains(word), "座位列表不应出现「%s」文字" % word)

    _expect(lobby.find_child("ReadyButton", true, false) == null, "房主不应有准备按钮")
    _expect(lobby.find_child("StartButton", true, false) != null, "房主应有开始按钮")
    _expect(lobby.find_child("DissolveButton", true, false) != null, "房主应有解散按钮")
    var start := lobby.find_child("StartButton", true, false) as Button
    _expect(start != null and start.disabled, "有成员未准备时开始应禁用")
    _expect(
        start != null and start.tooltip_text == tr("ui.room.start_waiting_ready"),
        "开始禁用原因应提示等待准备"
    )

    # 邀请入口打开好友抽屉的邀请模式。
    var invite := lobby.find_child("RoomInviteButton", true, false) as Button
    if invite != null:
        invite.pressed.emit()
        await get_tree().process_frame
        var panel := lobby.find_child("FriendsPanel", true, false) as Control
        _expect(panel != null and panel.visible, "邀请入口应打开好友面板")
        _expect(lobby.find_child("Invite_p_online", true, false) != null, "邀请模式应给在线好友邀请按钮")
        if panel != null:
            panel.close()

    _seed_room(8, true)
    GameStore.room_changed.emit()
    await get_tree().process_frame
    _expect(lobby.find_child("RoomInviteButton", true, false) == null, "满员时不应有邀请入口")

    _seed_room(3, true)
    GameStore.room_changed.emit()
    await get_tree().process_frame
    _expect(lobby.find_child("RoomInviteButton", true, false) != null, "空位恢复后邀请入口应回来")
    start = lobby.find_child("StartButton", true, false) as Button
    _expect(
        start != null and start.tooltip_text == tr("ui.room.start_need_players") % 5,
        "补位关闭且不足 8 人时应提示还差人数"
    )

    _expect(lobby._host_room_mode(), "房主且房间非空应判定为创建模式")
    GameStore.player_id = "p1"
    _expect(not lobby._host_room_mode(), "宾客不应判定为创建模式")
    GameStore.player_id = "p0"
    GameStore.room_players = []
    _expect(not lobby._host_room_mode(), "空房间不应判定为创建模式")

    lobby.queue_free()
    await get_tree().process_frame


func _check_room_phone_height() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    _seed_room(3, false)
    var lobby := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if lobby == null:
        return
    var right := lobby.find_child("LobbyRight", true, false) as Control
    _expect(
        right != null and right.size.y <= 508.0 + 1.0,
        "手机创建模式右栏不应超出 508，实际 %s" % (str(right.size.y) if right != null else "-")
    )
    lobby.queue_free()
    await get_tree().process_frame


func _seed_room(count: int, all_ready: bool) -> void:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.room_bot_fill = false
    var players := []
    for seat in count:
        players.append(
            {
                "playerId": "p%d" % seat,
                "displayName": "玩家%d" % seat,
                "seat": seat,
                "ready": all_ready or seat == 0,
                "isHost": seat == 0,
            }
        )
    GameStore.room_players = players


func _room_name_count(lobby: Node) -> int:
    var count := 0
    for child in lobby.find_children("RoomName_*", "Label", true, false):
        count += 1
    return count


func _sent_count(socket: RecordingSocket, message_type: String) -> int:
    var count := 0
    for raw in socket.sent:
        var envelope = JSON.parse_string(str(raw))
        if envelope is Dictionary and str(envelope.get("type", "")) == message_type:
            count += 1
    return count
