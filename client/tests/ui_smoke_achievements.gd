extends "res://tests/smoke_case.gd"
## 成就下发冒烟：非 Steam 忽略、Steam 幂等同步、单条解锁与重置清理。

const FakeBackend = preload("res://tests/fake_platform_backend.gd")

const ITEM := {"achievementId": "genetics", "steamApiName": "GENETICS"}


func _ready() -> void:
    await _check_dev_ignores()
    await _check_steam_syncs_once()
    await _check_single_unlock()
    await _check_reset_clears()
    Platform.clear_backend_for_test()
    GameStore.reset_session()
    _finish()


func _use(fake) -> void:
    Platform.set_backend_for_test(fake, Platform.CHANNEL_STEAM)


func _check_dev_ignores() -> void:
    Platform.clear_backend_for_test()
    GameStore.reset_session()
    GameStore.handle(Protocol.MSG_ACHIEVEMENT_STATE, {"unlocked": [ITEM]})
    _expect(GameStore.unlocked_achievements.has("genetics"), "非 Steam 也记录解锁状态")
    GameStore.reset_session()


func _check_steam_syncs_once() -> void:
    var fake = FakeBackend.new()
    _use(fake)
    GameStore.reset_session()
    GameStore.handle(Protocol.MSG_ACHIEVEMENT_STATE, {"unlocked": [ITEM]})
    await get_tree().process_frame
    _expect(str(fake.unlocked) == str(["GENETICS"]), "Steam 渠道同步成就 API 名")
    GameStore.handle(Protocol.MSG_ACHIEVEMENT_STATE, {"unlocked": [ITEM]})
    await get_tree().process_frame
    _expect(fake.unlocked.size() == 1, "重复下发同一成就只同步一次")
    GameStore.handle(
        Protocol.MSG_ACHIEVEMENT_STATE,
        {"unlocked": [{"achievementId": "turing-moment", "steamApiName": "TURING_MOMENT"}]},
    )
    await get_tree().process_frame
    _expect(fake.unlocked.size() == 2, "不同成就分别同步")
    GameStore.reset_session()


func _check_single_unlock() -> void:
    var fake = FakeBackend.new()
    _use(fake)
    GameStore.reset_session()
    GameStore.handle(Protocol.MSG_ACHIEVEMENT_UNLOCKED, ITEM)
    await get_tree().process_frame
    _expect(str(fake.unlocked) == str(["GENETICS"]), "单条解锁走同一同步路径")
    GameStore.reset_session()


func _check_reset_clears() -> void:
    var fake = FakeBackend.new()
    _use(fake)
    GameStore.reset_session()
    GameStore.handle(Protocol.MSG_ACHIEVEMENT_STATE, {"unlocked": [ITEM]})
    _expect(GameStore.unlocked_achievements.size() == 1, "解锁状态写入内存")
    GameStore.reset_session()
    _expect(GameStore.unlocked_achievements.is_empty(), "重置会话清空解锁状态")
