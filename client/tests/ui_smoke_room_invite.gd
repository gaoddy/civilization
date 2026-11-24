extends "res://tests/smoke_case.gd"
## Steam 邀请冒烟：入口可见性、token 换大厅、邀请面板、大厅失败置灰与接收邀请。

const ROOM_SCENE := "res://pages/room/room.tscn"
const FakeBackend = preload("res://tests/fake_platform_backend.gd")


func _ready() -> void:
    await _check_dev_has_no_entry()
    await _check_invite_flow()
    await _check_lobby_failure_disables()
    await _check_reject_while_in_room()
    await _check_pending_until_authenticated()
    Platform.clear_backend_for_test()
    GameStore.reset_session()
    _fake_disconnect()
    _finish()


func _use_fake() -> Object:
    var fake = FakeBackend.new()
    Platform.set_backend_for_test(fake, Platform.CHANNEL_STEAM)
    return fake


func _seed_room() -> void:
    GameStore.reset_session()
    GameStore.player_id = "p_me"
    GameStore.player_code = "100001"
    GameStore.token = "session_tok"
    GameStore.display_name = "我"
    GameStore.room_id = "r1"
    GameStore.room_code = "ABC234"
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.room_players = [
        {
            "displayName": "我",
            "playerId": "p_me",
            "seat": 0,
            "ready": false,
            "isHost": true,
        }
    ]


func _status(instance: Node) -> String:
    var label = instance.find_child("StatusLabel", true, false)
    return str(label.text) if label != null else ""


func _check_dev_has_no_entry() -> void:
    Platform.clear_backend_for_test()
    _seed_room()
    _fake_connect()
    var instance = await _check_scene(ROOM_SCENE, false)
    if instance == null:
        return
    _expect(instance.find_child("SteamInviteButton", true, false) == null, "非 Steam 构建没有 Steam 邀请入口")
    instance.queue_free()
    await get_tree().process_frame


func _check_invite_flow() -> void:
    var fake = _use_fake()
    _seed_room()
    var instance = await _check_scene(ROOM_SCENE, false)
    if instance == null:
        return
    _fake_connect()
    await get_tree().process_frame
    var button = instance.find_child("SteamInviteButton", true, false)
    _expect(button != null and not button.disabled, "Steam 渠道已连接时入口可用")

    button.pressed.emit()
    await get_tree().process_frame
    _expect(button.disabled, "请求 token 期间入口置灰")
    _expect(_status(instance) == tr("ui.room.invite_steam.creating"), "创建期间给出状态")

    GameStore.handle(
        Protocol.MSG_ROOM_INVITE_CODE,
        {"token": "inv_1", "roomCode": "ABC234", "expiresAt": "2030-01-01T00:01:00Z"},
    )
    await get_tree().process_frame
    await get_tree().process_frame
    _expect(fake.invited, "token 到手后打开好友邀请面板")
    _expect(Platform.is_in_lobby(), "大厅创建成功后标记已入大厅")
    _expect(_status(instance) == tr("ui.room.invite_steam.sent"), "邀请成功给出状态")
    _expect(not button.disabled, "邀请结束后入口恢复")
    instance.queue_free()
    await get_tree().process_frame


func _check_lobby_failure_disables() -> void:
    var fake = _use_fake()
    fake.lobby_ok = false
    _seed_room()
    var instance = await _check_scene(ROOM_SCENE, false)
    if instance == null:
        return
    _fake_connect()
    await get_tree().process_frame
    var button = instance.find_child("SteamInviteButton", true, false)
    button.pressed.emit()
    await get_tree().process_frame
    GameStore.handle(
        Protocol.MSG_ROOM_INVITE_CODE,
        {"token": "inv_2", "roomCode": "ABC234", "expiresAt": "2030-01-01T00:01:00Z"},
    )
    await get_tree().process_frame
    await get_tree().process_frame
    _expect(button.disabled, "大厅创建失败后入口置灰")
    _expect(
        str(button.tooltip_text) == tr("ui.room.invite_steam.unavailable"),
        "置灰时提示可用房间码",
    )
    _expect(_status(instance) == tr("ui.room.invite_steam.unavailable"), "大厅失败给出状态")
    instance.queue_free()
    await get_tree().process_frame


func _check_reject_while_in_room() -> void:
    var fake = _use_fake()
    _seed_room()
    var instance = await _check_scene(ROOM_SCENE, false)
    if instance == null:
        return
    _fake_connect()
    await get_tree().process_frame
    fake.join_request = {"room_code": "OTHER", "invite_token": "inv_other"}
    await get_tree().process_frame
    _expect(_status(instance) == tr("ui.room.invite_steam.already_in_room"), "已在房间时提示先离开")
    _expect(GameStore.room_id == "r1", "拒绝邀请不改变当前房间")
    _expect(str(GameStore.last_error.get("code", "")) == "ALREADY_IN_ROOM", "拒绝走 ALREADY_IN_ROOM")
    instance.queue_free()
    await get_tree().process_frame


func _check_pending_until_authenticated() -> void:
    _use_fake()
    _seed_room()
    _fake_connect()
    GameStore.reset_room()
    GameStore.player_id = ""
    GameStore.token = ""
    GameStore.accept_steam_invite("inv_wait")
    _expect(GameStore.pending_steam_invite == "inv_wait", "未认证时缓存邀请 token")
    GameStore.handle(
        Protocol.MSG_AUTH_RESULT,
        {"playerId": "p_me", "playerCode": "100001", "token": "session_tok"},
    )
    _expect(GameStore.pending_steam_invite == "", "认证成功后补发并清空缓存")
    _expect(GameStore.room_id == "", "补发邀请不自行写入房间状态")
