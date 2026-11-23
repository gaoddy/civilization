extends "res://tests/smoke_case.gd"
## 天梯冒烟：排行榜渲染、个人主页可见性、结算积分与段位变化提示。

const LEADERBOARD_SCENE := "res://pages/leaderboard/leaderboard.tscn"
const PROFILE_SCENE := "res://pages/profile/profile.tscn"
const RESULT_SCENE := "res://pages/result/result.tscn"


func _ready() -> void:
    await _check_leaderboard()
    await _check_profile_visible()
    await _check_profile_hidden()
    await _check_result_promoted()
    await _check_result_demoted()
    await _check_result_room_match()
    _restore()
    _finish()


func _check_leaderboard() -> void:
    GameStore.leaderboard = [
        {"rank": 1, "playerId": "p1", "displayName": "甲", "tier": "gold", "rating": 1300},
        {"rank": 2, "playerId": "p2", "displayName": "乙", "tier": "silver", "rating": 1200},
    ]
    var instance := await _check_scene(LEADERBOARD_SCENE, false)
    if instance == null:
        return
    var list = instance.find_child("LeaderboardList", true, false)
    _expect(list != null and list.get_child_count() == 2, "排行榜应渲染两行")
    if list != null and list.get_child_count() == 2:
        var expected := TranslationServer.translate("ui.leaderboard.row") % [
            1, "甲", TranslationServer.translate("ui.tier.gold"), 1300,
        ]
        _expect(str(list.get_child(0).text) == expected, "排行榜首行不符：%s" % list.get_child(0).text)
    var status = instance.find_child("LeaderboardStatus", true, false)
    _expect(status != null and not status.visible, "有数据时不显示空态")
    instance.queue_free()
    await get_tree().process_frame


func _seed_profile(recent_visible: bool, recent: Array) -> void:
    GameStore.profile_target = "p1"
    GameStore.profile_return_page = UINavigation.PAGE_LEADERBOARD
    GameStore.handle(
        Protocol.MSG_PLAYER_PROFILE,
        {
            "playerId": "p1",
            "displayName": "甲",
            "rating": 1300,
            "tier": "gold",
            "highestTier": "platinum",
            "recentVisible": recent_visible,
            "top4Ratio": 0.5,
            "recent": recent,
        },
    )


func _check_profile_visible() -> void:
    _seed_profile(true, [
        {"placement": 1, "ratingDelta": 66, "ratingAfter": 1366, "finishedAt": "2026-11-01T00:00:00Z"},
        {"placement": 6, "ratingDelta": -43, "ratingAfter": 1323, "finishedAt": "2026-11-01T01:00:00Z"},
    ])
    var instance := await _check_scene(PROFILE_SCENE, false)
    if instance == null:
        return
    var rating = instance.find_child("ProfileRating", true, false)
    _expect(rating != null and str(rating.text).contains("1300"), "个人主页应显示当前积分")
    var highest = instance.find_child("ProfileHighestTier", true, false)
    _expect(
        highest != null and str(highest.text).contains(TranslationServer.translate("ui.tier.platinum")),
        "个人主页应显示历史最高段位",
    )
    var top4 = instance.find_child("ProfileTop4", true, false)
    _expect(top4 != null and str(top4.text).contains("50%"), "前 4 名比例按服务端值展示")
    var recent = instance.find_child("ProfileRecent", true, false)
    _expect(recent != null and recent.get_child_count() == 2, "最近战绩行数不符")
    if recent != null and recent.get_child_count() == 2:
        var expected := TranslationServer.translate("ui.profile.recent_row") % [1, "+66", 1366]
        _expect(str(recent.get_child(0).text) == expected, "最近战绩首行不符：%s" % recent.get_child(0).text)
    instance.queue_free()
    await get_tree().process_frame


func _check_profile_hidden() -> void:
    _seed_profile(false, [])
    var instance := await _check_scene(PROFILE_SCENE, false)
    if instance == null:
        return
    var text := _node_text(instance)
    _expect(
        text.contains(TranslationServer.translate("ui.profile.recent_hidden")),
        "非好友应提示最近战绩不可见",
    )
    _expect(
        text.contains(TranslationServer.translate("ui.profile.hidden")),
        "不可见时前 4 名比例应显示占位",
    )
    _expect(instance.find_child("ProfileRecent", true, false) == null, "不可见时不渲染战绩列表")
    instance.queue_free()
    await get_tree().process_frame


func _seed_result(delta, after, tier_before: String, tier_after: String) -> void:
    GameStore.player_id = "p0"
    GameStore.ranking = [{"playerId": "p0", "place": 1}]
    GameStore.game_seed = "seed"
    GameStore.self_stats = {}
    GameStore.rating_delta = delta
    GameStore.rating_after = after
    GameStore.tier_before = tier_before
    GameStore.tier_after = tier_after


func _check_result_promoted() -> void:
    _seed_result(66, 1066, "bronze", "silver")
    var instance := await _check_scene(RESULT_SCENE, false)
    if instance == null:
        return
    var block = instance.find_child("RatingBlock", true, false)
    _expect(block != null, "天梯局结算应显示积分块")
    if block != null:
        var delta = block.find_child("RatingDelta", true, false)
        var expected := TranslationServer.translate("ui.result.rating_delta") % ["+66", 1066]
        _expect(delta != null and str(delta.text) == expected, "积分变化文案不符：%s" % delta.text)
        var trend = block.find_child("TierTrend", true, false)
        _expect(
            trend != null and str(trend.text) == TranslationServer.translate("ui.result.promoted"),
            "跨段位上升应提示晋级",
        )
    instance.queue_free()
    await get_tree().process_frame


func _check_result_demoted() -> void:
    _seed_result(-66, 934, "silver", "bronze")
    var instance := await _check_scene(RESULT_SCENE, false)
    if instance == null:
        return
    var trend = instance.find_child("TierTrend", true, false)
    _expect(
        trend != null and str(trend.text) == TranslationServer.translate("ui.result.demoted"),
        "跨段位下降应提示降级",
    )
    instance.queue_free()
    await get_tree().process_frame


func _check_result_room_match() -> void:
    _seed_result(null, null, "", "")
    var instance := await _check_scene(RESULT_SCENE, false)
    if instance == null:
        return
    _expect(
        instance.find_child("RatingBlock", true, false) == null,
        "房间局不计分，结算页不显示积分块",
    )
    instance.queue_free()
    await get_tree().process_frame


func _restore() -> void:
    GameStore.reset_room()
    GameStore.leaderboard = []
    GameStore.profile = {}
    GameStore.profile_target = ""
    GameStore.profile_return_page = ""
    GameStore.player_id = ""
    GameStore.ranking = []
    GameStore.game_seed = ""
    GameStore.self_stats = {}
