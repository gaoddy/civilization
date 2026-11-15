extends Node
## 无头冒烟：实例化各场景，确认脚本与控件构建无错误。

const FixtureScript = preload("res://tests/fixture.gd")

const SCENES := [
    "res://scenes/login.tscn",
    "res://scenes/lobby.tscn",
    "res://scenes/room.tscn",
]

var _failures := 0


func _ready() -> void:
    for path in SCENES:
        await _check_scene(path, true)
    await _check_game()
    await _check_result()
    if _failures == 0:
        print("UI_SMOKE_OK")
        get_tree().quit(0)
    else:
        get_tree().quit(1)


func _check_scene(path: String, free_after: bool) -> Node:
    var packed = load(path)
    if packed == null:
        push_error("无法加载场景 %s" % path)
        _failures += 1
        return null
    var instance = packed.instantiate()
    if instance == null:
        push_error("无法实例化场景 %s" % path)
        _failures += 1
        return null
    add_child(instance)
    await get_tree().process_frame
    await get_tree().process_frame
    if free_after:
        instance.queue_free()
    return instance


func _check_game() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.view = FixtureScript.view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    for index in range(6):
        var name := "ShopSlot%d" % index
        if instance.find_child(name, true, false) == null:
            push_error("对局场景缺少 %s" % name)
            _failures += 1
    instance.queue_free()
    GameStore.view = {}


func _check_result() -> void:
    GameStore.ranking = [
        {"playerId": "p0", "place": 1},
        {"playerId": "p1", "place": 2},
    ]
    GameStore.game_seed = "abc123"
    var instance := await _check_scene("res://scenes/result.tscn", false)
    if instance == null:
        return
    var list = instance.find_child("RankingList", true, false)
    if list == null:
        push_error("结算场景缺少 RankingList")
        _failures += 1
    elif list.get_child_count() != 2:
        push_error("结算场景排名行数为 %d，期望 2" % list.get_child_count())
        _failures += 1
    instance.queue_free()
    GameStore.ranking = []
    GameStore.game_seed = ""

