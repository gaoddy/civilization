extends "res://tests/smoke_case.gd"
## 账号页冒烟：四种页面状态、验证码倒计时、绑定失败保留输入与大厅入口可见性。

const ACCOUNT_SCENE := "res://pages/account/account.tscn"
const LOBBY_SCENE := "res://pages/lobby/lobby.tscn"
const FakeBackend = preload("res://tests/fake_platform_backend.gd")


func _ready() -> void:
    await _check_unbound_form()
    await _check_bound_state()
    await _check_load_failure()
    await _check_send_code_countdown()
    await _check_bind_failure_keeps_input()
    await _check_lobby_entry_visibility()
    Platform.clear_backend_for_test()
    GameStore.reset_session()
    Net.disconnect_from_server()
    _finish()


func _seed_account(phone_bound: bool, masked_phone := "") -> void:
    GameStore.reset_session()
    GameStore.handle(
        Protocol.MSG_ACCOUNT_VIEW,
        {
            "playerId": "p1",
            "playerCode": "100001",
            "displayName": "甲",
            "steamBound": true,
            "phoneBound": phone_bound,
            "maskedPhone": masked_phone,
        },
    )


func _check_unbound_form() -> void:
    _seed_account(false)
    var instance = await _check_scene(ACCOUNT_SCENE, false)
    if instance == null:
        return
    await get_tree().process_frame
    var form = instance.find_child("BindForm", true, false)
    _expect(form != null and form.visible, "未绑定时显示绑定表单")
    var status = instance.find_child("PhoneStatus", true, false)
    _expect(status != null and str(status.text) == tr("ui.account.phone_unbound"), "未绑定文案")
    _expect(instance.find_child("PhoneField", true, false) != null, "提供手机号输入")
    _expect(instance.find_child("CodeField", true, false) != null, "提供验证码输入")
    _expect(instance.find_child("SendCodeButton", true, false) != null, "提供获取验证码")
    _expect(instance.find_child("BindButton", true, false) != null, "提供绑定按钮")
    _expect(instance.find_child("RetryButton", true, false).visible == false, "加载成功后不收起重试")
    instance.queue_free()
    await get_tree().process_frame


func _check_bound_state() -> void:
    _seed_account(true, "138****0000")
    var instance = await _check_scene(ACCOUNT_SCENE, false)
    if instance == null:
        return
    await get_tree().process_frame
    var form = instance.find_child("BindForm", true, false)
    _expect(form != null and not form.visible, "已绑定时隐藏绑定表单")
    var status = instance.find_child("PhoneStatus", true, false)
    _expect(
        status != null and str(status.text) == tr("ui.account.phone_bound") % "138****0000",
        "已绑定显示服务端掩码",
    )
    instance.queue_free()
    await get_tree().process_frame


func _check_load_failure() -> void:
    GameStore.reset_session()
    var instance = await _check_scene(ACCOUNT_SCENE, false)
    if instance == null:
        return
    await get_tree().process_frame
    _expect(
        str(instance.find_child("AccountStatus", true, false).text) == tr("ui.account.loading"),
        "进入页面先显示加载中",
    )
    GameStore.handle(Protocol.MSG_ERROR, {"code": "INTERNAL_ERROR"})
    await get_tree().process_frame
    _expect(instance.find_child("RetryButton", true, false).visible, "加载失败显示重试")
    _expect(not instance.find_child("BindForm", true, false).visible, "加载失败不显示空表单")
    instance.queue_free()
    await get_tree().process_frame


func _check_send_code_countdown() -> void:
    _seed_account(false)
    var instance = await _check_scene(ACCOUNT_SCENE, false)
    if instance == null:
        return
    instance.set_process(false)
    await get_tree().process_frame
    var send = instance.find_child("SendCodeButton", true, false)
    GameStore.handle(
        Protocol.MSG_PHONE_CODE_SENT,
        {"expiresAt": "2030-01-01T00:10:00Z"},
    )
    _expect(send.disabled, "发送成功后倒计时期间禁用")
    _expect(str(send.text).begins_with("60"), "倒计时从 60 秒开始")
    instance._process(61.0)
    _expect(not send.disabled, "倒计时结束恢复可用")
    _expect(str(send.text) == tr("ui.account.send_code"), "倒计时结束恢复文案")
    instance.queue_free()
    await get_tree().process_frame


func _check_bind_failure_keeps_input() -> void:
    _seed_account(false)
    var instance = await _check_scene(ACCOUNT_SCENE, false)
    if instance == null:
        return
    await get_tree().process_frame
    var phone = instance.find_child("PhoneField", true, false)
    var code = instance.find_child("CodeField", true, false)
    var bind = instance.find_child("BindButton", true, false)
    phone.text = "13900000001"
    code.text = "123456"
    bind.emit_signal("pressed")
    _expect(bind.disabled, "绑定请求中禁用按钮")
    GameStore.handle(Protocol.MSG_ERROR, {"code": "CODE_INVALID"})
    await get_tree().process_frame
    _expect(not bind.disabled, "绑定失败恢复按钮")
    _expect(phone.text == "13900000001" and code.text == "123456", "绑定失败保留输入")
    _expect(
        str(instance.find_child("AccountStatus", true, false).text) == UIWording.error_text("CODE_INVALID"),
        "绑定失败显示错误文案",
    )
    instance.queue_free()
    await get_tree().process_frame


func _check_lobby_entry_visibility() -> void:
    Platform.clear_backend_for_test()
    GameStore.reset_session()
    var dev_lobby = await _check_scene(LOBBY_SCENE, false)
    if dev_lobby == null:
        return
    _expect(dev_lobby.find_child("AccountButton", true, false) == null, "非 Steam 构建没有账号入口")
    dev_lobby.queue_free()
    await get_tree().process_frame

    Platform.set_backend_for_test(FakeBackend.new(), Platform.CHANNEL_STEAM)
    var steam_lobby = await _check_scene(LOBBY_SCENE, false)
    if steam_lobby == null:
        return
    _expect(steam_lobby.find_child("AccountButton", true, false) != null, "Steam 构建显示账号入口")
    steam_lobby.queue_free()
    await get_tree().process_frame
