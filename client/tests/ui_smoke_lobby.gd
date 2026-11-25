extends "res://tests/smoke_case.gd"
## 大厅冒烟：两栏比例、卡片、匹配切换、创建模式邀请行、断线与错误提示。

const FakeBackend = preload("res://tests/fake_platform_backend.gd")


## 记录发送的假连接：大厅持有可用的连接状态，发送的消息不影响断言。
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
    Net.socket = RecordingSocket.new()
    var lobby := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if lobby != null:
        await _check_layout(lobby)
        await _check_frame_fill(lobby)
        await _check_hall_texture(lobby)
        await _check_match_toggle(lobby)
        await _check_queue_timer(lobby)
        await _check_creating_blocks_match(lobby)
        await _check_disconnect(lobby)
        await _check_error(lobby)
        await _check_match_found(lobby)
        await _check_auth_error(lobby)
        lobby.queue_free()
        await get_tree().process_frame

    await _check_queue_prefill()
    await _check_room_mode()
    await _check_room_phone_height()
    await _check_steam_share()
    ScreenProfile.set_device_for_test("")
    Net.socket = null
    _finish()


func _seed() -> void:
    GameStore.player_id = "p_me"
    GameStore.player_code = "100001"
    GameStore.display_name = "测试玩家"
    GameStore.account = {}
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.room_code = "ABC123"
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


func _check_frame_fill(lobby: Node) -> void:
    # 页面级材质在首帧布局后统一接入，等两帧再断言。
    for _frame in 3:
        await get_tree().process_frame
    for node_name in ["LobbyPlayerCard", "LobbyRankCard"]:
        _expect_frame_fill(lobby.find_child(node_name, true, false), node_name)

    var friends := lobby.find_child("FriendsPanel", true, false)
    _expect(friends != null, "大厅应有好友抽屉")
    if friends != null:
        friends.open()
        await get_tree().process_frame
        _expect_frame_fill(
            friends.find_child("FriendsPanelCard", true, false), "FriendsPanelCard"
        )
        friends.close()


## 框体材质贴着面板外沿：PanelContainer 的内容内边距不能把材质层挤进内容区。
func _expect_frame_fill(panel: Node, node_name: String) -> void:
    _expect(panel != null, "大厅缺少 %s" % node_name)
    if panel == null:
        return
    var overlay := panel.find_child(FrameMaterial.OVERLAY_NAME, true, false) as Control
    _expect(overlay != null, "%s 应带框体材质" % node_name)
    if overlay == null:
        return
    _expect(overlay.position == Vector2.ZERO, "%s 框体材质应从面板左上角开始" % node_name)
    _expect(
        overlay.size.is_equal_approx((panel as Control).size),
        "%s 框体材质应铺满面板，实际 %s / %s" % [
            node_name, str(overlay.size), str((panel as Control).size),
        ],
    )


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


func _check_hall_texture(lobby: Node) -> void:
    var hall := lobby.find_child("LobbyHallImage", true, false) as Control
    var left := lobby.find_child("LobbyLeft", true, false) as Control
    if hall == null:
        _expect(false, "大厅应有大厅图容器")
        return
    var rect := hall.find_child("HallTexture", true, false) as TextureRect
    if rect == null:
        _expect(false, "大厅图应有纹理层")
    else:
        _expect(rect.texture != null, "大厅图纹理应能加载")
        _expect(rect.expand_mode == TextureRect.EXPAND_IGNORE_SIZE, "大厅图应忽略纹理原始尺寸")
        _expect(
            rect.stretch_mode == TextureRect.STRETCH_KEEP_ASPECT_COVERED,
            "大厅图应以 cover 方式铺满"
        )
    if left != null:
        _expect(absf(hall.size.x - left.size.x) <= 1.0, "大厅图应铺满左栏宽度")


func _check_match_toggle(lobby: Node) -> void:
    var match_button := lobby.find_child("MatchButton", true, false) as Button
    var create_button := lobby.find_child("CreateRoomButton", true, false) as Button
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
    _expect(
        queue_card != null and not _node_text(queue_card).contains("匹配范围"),
        "排队卡不应显示匹配范围"
    )
    _expect(lobby.find_child("QueueWindow", true, false) == null, "排队卡不应有积分窗口节点")

    match_button.pressed.emit()
    await get_tree().process_frame
    _expect(not GameStore.queueing, "再点应取消排队")
    _expect(match_button.text == tr("ui.lobby.start_match"), "取消后主按钮应回到开始匹配")
    _expect(queue_card != null and not queue_card.visible, "取消后应隐藏排队卡")


## 等待时长由客户端本地时钟累计：开始匹配即起算，服务端下发 `waitingMs` 不重置。
func _check_queue_timer(lobby: Node) -> void:
    var match_button := lobby.find_child("MatchButton", true, false) as Button
    var timer := lobby.find_child("QueueTimer", true, false) as Label
    if match_button == null or timer == null:
        _expect(false, "排队卡应提供计时与匹配按钮")
        return

    match_button.pressed.emit()
    await get_tree().process_frame
    _expect(GameStore.queueing, "开始匹配后应进入排队")
    _expect(timer.text == "00:00", "刚入队应从 00:00 起算，实际 %s" % timer.text)
    # 排队卡在页面装载时隐藏，进入排队后才显示；材质不能只在页面装载时接一次。
    await get_tree().process_frame
    _expect_frame_fill(lobby.find_child("LobbyQueueCard", true, false), "LobbyQueueCard")

    # 计时器每 0.5 秒刷新一次，采样留出一个刷新周期的余量。
    await get_tree().create_timer(1.8).timeout
    var started := _clock_seconds(timer.text)
    _expect(started >= 1, "排队后计时应开始走，实际 %s" % timer.text)

    # 服务端只在人数变化时下发快照，且 waitingMs 可能为 0；不得把本地计时清零。
    GameStore.handle(Protocol.MSG_QUEUE_STATE, {"waitingMs": 0, "waitingPlayers": 2})
    await get_tree().process_frame
    var after_snapshot := _clock_seconds(timer.text)
    _expect(after_snapshot >= started, "服务端 waitingMs=0 不应把计时归零，实际 %s" % timer.text)

    await get_tree().create_timer(1.4).timeout
    _expect(_clock_seconds(timer.text) > after_snapshot, "收到快照后计时应继续走，实际 %s" % timer.text)

    match_button.pressed.emit()
    await get_tree().process_frame
    _expect(not GameStore.queueing, "测试结束应取消排队")


## 计时文本换算成秒；格式不符返回 -1。
static func _clock_seconds(text: String) -> int:
    var parts := text.split(":")
    if parts.size() != 2:
        return -1
    return int(parts[0]) * 60 + int(parts[1])


func _check_creating_blocks_match(lobby: Node) -> void:
    var match_button := lobby.find_child("MatchButton", true, false) as Button
    lobby.set("_creating", true)
    lobby.call("_apply_mode")
    await get_tree().process_frame
    _expect(match_button != null and match_button.disabled, "创建房间在途不应能开始匹配")
    lobby.set("_creating", false)
    lobby.call("_apply_mode")
    await get_tree().process_frame


func _check_match_found(lobby: Node) -> void:
    var card := lobby.find_child("LobbyQueueCard", true, false) as Control
    GameStore.queueing = true
    GameStore.queue_changed.emit()
    await get_tree().process_frame
    _expect(card != null and card.visible, "排队中应显示排队卡")
    GameStore.queueing = false
    GameStore.queue_changed.emit()
    GameStore.match_found.emit(0)
    await get_tree().process_frame
    var timer := lobby.find_child("QueueTimer", true, false) as Label
    _expect(card != null and card.visible, "匹配成功后排队卡应保持可见")
    _expect(
        timer != null and timer.text == tr("ui.lobby.match_found"),
        "匹配成功后应显示成功文案"
    )


## 认证类错误优先于其余提示：清空会话并回登录；离树后导航被丢弃，便于断言。
func _check_auth_error(lobby: Node) -> void:
    GameStore.player_id = "p_me"
    GameStore.last_error = {"code": "AUTH_FAILED"}
    var parent := lobby.get_parent()
    if parent != null:
        parent.remove_child(lobby)
    GameStore.error_received.emit()
    await get_tree().process_frame
    _expect(GameStore.player_id == "", "认证错误应清空会话回登录")


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


## 装载时已处于排队：计时器子节点在父节点 `_enter_tree` 之后才进树，
## 首次刷新必须等它进树再启动，否则计时不会走。
func _check_queue_prefill() -> void:
    _seed()
    GameStore.queueing = true
    var lobby := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if lobby != null:
        await get_tree().process_frame
        var timer := lobby.find_child("QueueRefresh", true, false) as Timer
        _expect(timer != null and not timer.is_stopped(), "装载时已排队时排队计时器应已启动")
        lobby.queue_free()
    GameStore.queueing = false
    await get_tree().process_frame


func _check_room_mode() -> void:
    _seed()
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
    var kick := lobby.find_child("RoomKick_p1", true, false)
    _expect(kick is Button and (kick as Button).text == tr("ui.room.kick"), "移除入口应是按钮")

    var text := _node_text(lobby)
    for word in ["玩家列表", "已准备", "未准备", "房间码"]:
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
    var hint := lobby.find_child("RoomStartHint", true, false) as Label
    _expect(hint != null and hint.visible, "开始禁用时应显示可见原因行")
    _expect(
        hint != null and hint.text == tr("ui.room.start_waiting_ready"),
        "原因行文案应与 tooltip 一致"
    )

    _expect(lobby.find_child("RoomCodeValue", true, false) == null, "创建模式不应显示房间码")
    _expect(lobby.find_child("CopyCodeButton", true, false) == null, "创建模式不应有复制房间码入口")
    _expect(lobby.find_child("SteamInviteButton", true, false) == null, "非 Steam 构建没有 Steam 邀请入口")
    var invite_row := lobby.find_child("RoomInviteRow", true, false) as Control
    var invite_status := lobby.find_child("RoomInviteStatus", true, false) as Label
    _expect(invite_row != null, "有空位时应显示邀请行")
    _expect(invite_status != null and not invite_status.visible, "无状态文案时不应显示状态行")
    var fill := lobby.find_child("RoomFillHint", true, false) as Label
    _expect(
        fill != null and fill.text == tr("ui.room.bot_fill_off_hint") % 5,
        "补位关闭时应提示还差人数"
    )

    # 邀请入口打开好友抽屉的邀请模式。
    var invite := lobby.find_child("RoomInviteButton", true, false) as Button
    if invite != null:
        invite.pressed.emit()
        await get_tree().process_frame
        var panel := lobby.find_child("FriendsPanel", true, false) as Control
        _expect(panel != null and panel.visible, "邀请入口应打开好友面板")
        _expect(lobby.find_child("Invite_p_online", true, false) != null, "邀请模式应给在线好友邀请按钮")
        GameStore.last_error = {"code": "ROOM_NOT_FOUND"}
        GameStore.error_received.emit()
        await get_tree().process_frame
        var friends_status := lobby.find_child("FriendsStatus", true, false) as Label
        _expect(
            friends_status != null and friends_status.text == UIWording.error_text("ROOM_NOT_FOUND"),
            "好友抽屉打开时错误应回填到抽屉"
        )
        if panel != null:
            panel.close()

    _seed_room(8, true)
    GameStore.room_changed.emit()
    await get_tree().process_frame
    _expect(lobby.find_child("RoomInviteButton", true, false) == null, "满员时不应有邀请入口")
    _expect(lobby.find_child("SteamInviteButton", true, false) == null, "满员时不应有 Steam 邀请入口")
    _expect(lobby.find_child("RoomInviteStatus", true, false) == null, "满员时不应有邀请状态行")
    _expect(not (hint != null and hint.visible), "开始可用时不应显示原因行")

    _seed_room(3, true)
    GameStore.room_bot_fill = true
    GameStore.room_changed.emit()
    await get_tree().process_frame
    _expect(
        fill != null and fill.text == tr("ui.room.bot_fill_hint") % 5,
        "补位开启时应提示补入机器人数"
    )
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
    _seed()
    _seed_room(3, false)
    var lobby := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if lobby == null:
        return
    var area := lobby.find_child("SafeAreaMargin", true, false) as Control
    _expect(area != null, "大厅应有安全区包裹")
    if area != null:
        area.call("apply_for_test", Vector4i(0, 0, 0, 29), 0)
        await get_tree().process_frame
        var actions := lobby.find_child("LobbyRoomActions", true, false) as Control
        _expect(actions != null, "手机创建模式应有房间操作")
        for node_name in ["RoomInviteButton"]:
            var entry := lobby.find_child(node_name, true, false) as Button
            # 手机档位次要操作命中尺寸为 44 画布单位；无头环境的 hit_small 不换算，直接断言令牌值。
            _expect(
                entry != null and entry.custom_minimum_size.y >= 44,
                "手机邀请入口应达到次要命中尺寸：%s" % node_name
            )
        # 操作按钮按令牌固定视觉高度，触控端不再二次抬升到命中高度。
        var action_buttons := {
            "MatchButton": "lobby-primary-action-height",
            "CreateRoomButton": "lobby-secondary-action-height",
            "ReconnectButton": "action-button-height",
            "NoticeConfirmButton": "action-button-height",
        }
        for node_name in action_buttons:
            var entry := lobby.find_child(node_name, true, false) as Button
            var expected := UITheme.stage_metric(action_buttons[node_name])
            _expect(
                entry != null and int(entry.custom_minimum_size.y) == expected,
                "手机操作按钮应按令牌固定高度：%s 期望 %d，实际 %s" % [
                    node_name, expected, -1 if entry == null else int(entry.custom_minimum_size.y)
                ]
            )
        if actions != null:
            var page := lobby as Control
            var scale: float = page.size.x / float(ScreenProfile.base_canvas().x)
            var bottom: float = actions.get_global_rect().end.y / scale
            _expect(
                bottom <= ScreenProfile.base_canvas().y - 29.0 + 1.0,
                "手机安全区内房间操作不应被裁切，底部 %s" % bottom
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


func _check_steam_share() -> void:
    var fake = FakeBackend.new()
    Platform.set_backend_for_test(fake, Platform.CHANNEL_STEAM)
    _seed()
    _seed_room(3, false)
    var lobby := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if lobby == null:
        Platform.clear_backend_for_test()
        return
    var steam := lobby.find_child("SteamInviteButton", true, false) as Button
    _expect(steam != null and not steam.disabled, "Steam 构建应有可用的 Steam 邀请入口")
    if steam != null:
        steam.pressed.emit()
        await get_tree().process_frame
        _expect(steam.disabled, "请求邀请 token 期间入口置灰")
        GameStore.handle(
            Protocol.MSG_ROOM_INVITE_CODE,
            {"token": "inv_lobby", "roomCode": "ABC123", "expiresAt": "2030-01-01T00:01:00Z"},
        )
        await get_tree().process_frame
        await get_tree().process_frame
        _expect(fake.invited, "token 到手后打开 Steam 好友邀请")
        var status := lobby.find_child("RoomInviteStatus", true, false) as Label
        _expect(
            status != null and status.text == tr("ui.room.invite_steam.sent"),
            "Steam 邀请成功后应在邀请行下方给出状态"
        )
    lobby.queue_free()
    await get_tree().process_frame
    Platform.clear_backend_for_test()
