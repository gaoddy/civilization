extends "res://tests/smoke_case.gd"
## Steam 登录冒烟：假后端断言自动登录、失败重试、无手机号表单与账号 / 成就 / 邀请状态搬运。

const FakeBackend = preload("res://tests/fake_platform_backend.gd")
const LOGIN_SCENE := "res://pages/login/login.tscn"

var _join_events: Array = []


func _ready() -> void:
    Platform.lobby_join_requested.connect(_on_lobby_join_requested)
    _check_display_name()
    _check_platform_url()
    await _check_account_and_achievements()
    await _check_login_steam_failure()
    await _check_login_steam_success()
    await _check_lobby_transport()
    await _check_dev_login_unchanged()
    Platform.clear_backend_for_test()
    GameStore.reset_session()
    Net.disconnect_from_server()
    _finish()


func _on_lobby_join_requested(room_code: String, invite_token: String) -> void:
    _join_events.append([room_code, invite_token])


func _use_fake(reason := "") -> Object:
    var fake = FakeBackend.new()
    fake.reason = reason
    Platform.set_backend_for_test(fake, Platform.CHANNEL_STEAM)
    return fake


func _check_display_name() -> void:
    _expect(GameStore.steam_display_name("  Steam 玩家  ") == "Steam 玩家", "展示名去空白")
    var long_name := "一二三四五六七八九十一二三四五六七八九十尾"
    _expect(
        GameStore.steam_display_name(long_name) == "一二三四五六七八九十一二三四五六七八九十",
        "展示名按字符截断到 20",
    )
    _expect(GameStore.steam_display_name("   ") == "", "空展示名不传")


func _check_platform_url() -> void:
    _expect(Net.default_url_for(false) == "ws://127.0.0.1:8080/ws", "非 Steam 桌面回落本机地址")
    _expect(Net.default_url_for(false, true) == "ws://127.0.0.1:8080/ws", "Steam 无配置回落本机地址")


func _check_account_and_achievements() -> void:
    GameStore.reset_session()
    var fake = _use_fake()
    GameStore.handle(
        Protocol.MSG_ACCOUNT_VIEW,
        {
            "playerId": "p1",
            "playerCode": "100001",
            "displayName": "甲",
            "steamBound": true,
            "phoneBound": false,
        },
    )
    _expect(str(GameStore.account.get("playerCode", "")) == "100001", "账号视图保存玩家编号")
    _expect(GameStore.account.get("steamBound") == true, "账号视图保存 Steam 绑定状态")

    var state := {
        "unlocked": [
            {"achievementId": "a1", "steamApiName": "ACH_ONE"},
            {"achievementId": "a2", "steamApiName": "ACH_TWO"},
        ]
    }
    GameStore.handle(Protocol.MSG_ACHIEVEMENT_STATE, state)
    _expect(str(fake.unlocked) == str(["ACH_ONE", "ACH_TWO"]), "认证下发的成就逐个同步")
    GameStore.handle(Protocol.MSG_ACHIEVEMENT_STATE, state)
    _expect(fake.unlocked.size() == 2, "重复成就状态不重复同步")
    GameStore.handle(
        Protocol.MSG_ACHIEVEMENT_UNLOCKED,
        {"achievementId": "a3", "steamApiName": "ACH_THREE"},
    )
    _expect(str(fake.unlocked) == str(["ACH_ONE", "ACH_TWO", "ACH_THREE"]), "局内解锁按 API 名透传")
    await get_tree().process_frame


func _check_login_steam_failure() -> void:
    GameStore.reset_session()
    var fake = _use_fake("not_running")
    var instance = await _check_scene(LOGIN_SCENE, false)
    if instance == null:
        return
    await get_tree().process_frame
    _expect(
        instance.find_children("*", "LineEdit", true, false).is_empty(),
        "Steam 登录不显示手机号表单",
    )
    var status = instance.find_child("StatusLabel", true, false)
    _expect(status != null and str(status.text) == tr("ui.login.steam.not_running"), "未运行 Steam 时给唯一文案")
    var retry = instance.find_child("RetryButton", true, false)
    _expect(retry != null and retry.visible, "失败后显示重试")
    _expect(instance.find_child("ExitButton", true, false) != null, "失败后仍可退出")

    fake.reason = ""
    retry.emit_signal("pressed")
    await get_tree().process_frame
    await get_tree().process_frame
    _expect(str(status.text) == tr("ui.login.steam.connecting"), "重试重新取票并进入连接")
    _expect(not retry.visible, "重试成功后收起重试按钮")
    Net.disconnect_from_server()
    instance.queue_free()
    await get_tree().process_frame


func _check_login_steam_success() -> void:
    GameStore.reset_session()
    _use_fake()
    var instance = await _check_scene(LOGIN_SCENE, false)
    if instance == null:
        return
    await get_tree().process_frame
    var status = instance.find_child("StatusLabel", true, false)
    _expect(status != null and str(status.text) == tr("ui.login.steam.connecting"), "自动登录进入连接状态")
    var retry = instance.find_child("RetryButton", true, false)
    _expect(retry != null and not retry.visible, "自动登录不显示重试")
    Net.disconnect_from_server()
    instance.queue_free()
    await get_tree().process_frame


func _check_lobby_transport() -> void:
    var fake = _use_fake()
    var created: bool = await Platform.create_lobby("ABC234", "inv_1", "2030-01-01T00:00:00Z")
    _expect(created and Platform.is_in_lobby(), "大厅创建成功后标记已入大厅")
    Platform.invite_friends()
    _expect(fake.invited, "邀请面板在已入大厅时打开")

    fake.join_request = {"room_code": "ABC234", "invite_token": "inv_1"}
    await get_tree().process_frame
    _expect(str(_join_events) == str([["ABC234", "inv_1"]]), "好友入房请求转成房间码与邀请 token")

    Platform.leave_lobby()
    _expect(fake.left and not Platform.is_in_lobby(), "离开大厅后清空大厅状态")

    fake.lobby_ok = false
    _expect(not (await Platform.create_lobby("ABC234", "inv_2", "2030-01-01T00:00:00Z")), "大厅创建失败返回 false")


func _check_dev_login_unchanged() -> void:
    Platform.clear_backend_for_test()
    GameStore.reset_session()
    var instance = await _check_scene(LOGIN_SCENE, false)
    if instance == null:
        return
    await get_tree().process_frame
    _expect(instance.find_children("*", "LineEdit", true, false).size() == 4, "开发登录保留四个输入框")
    _expect(instance.find_child("LoginButton", true, false) != null, "开发登录保留登录按钮")
    _expect(instance.find_child("RetryButton", true, false) == null, "开发登录没有 Steam 重试按钮")
    instance.queue_free()
    await get_tree().process_frame
