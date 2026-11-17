extends "res://tests/smoke_case.gd"
## Task 9 右栏冒烟：顶栏与事件流已删除，右栏改为身份 / 回合倒计时 / 英雄三段。


func _ready() -> void:
    await _check_recruit_identity()
    await _check_combat_identity()
    await _check_eliminated_identity()
    await _check_ghost_identity()
    _finish()


func _base_view(phase: String) -> Dictionary:
    GameStore.player_id = "p0"
    GameStore.phase = phase
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.game_view()
    view["phase"] = phase
    return view


func _mount(view: Dictionary, combat: Dictionary = {}) -> Node:
    GameStore.view = view
    if combat.is_empty():
        _clear_combat()
    else:
        _set_combat(combat)
    var instance := await _check_scene("res://scenes/game.tscn", false)
    await get_tree().process_frame
    return instance


func _unmount(instance: Node) -> void:
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""


func _check_recruit_identity() -> void:
    var view := _base_view(Protocol.PHASE_RECRUIT)
    var instance := await _mount(view)
    if instance == null:
        return

    _expect(instance.find_child("Header", true, false) == null, "顶栏节点不应存在")
    _expect(instance.find_child("EraPill", true, false) == null, "顶栏时代胶囊不应存在")
    _expect(instance.find_child("LogText", true, false) == null, "事件流文本节点不应存在")

    var status = instance.find_child("SideStatus", true, false)
    _expect(status != null, "右栏中部应存在回合与倒计时块")
    if status != null:
        _expect(_node_text(status).contains("第 3 回合"), "右栏中部应显示回合数")

    var identity = instance.find_child("SideIdentity", true, false)
    _expect(identity != null, "右栏顶部应存在身份块")
    if identity != null:
        _expect(_node_text(identity).contains("商店营业中"), "招募阶段右栏顶部显示售货员")

    await _unmount(instance)


func _check_combat_identity() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["opponentPlayerId"] = "p2"
    var instance := await _mount(view, FixtureScript.combat_result())
    if instance == null:
        return

    var identity = instance.find_child("SideIdentity", true, false)
    _expect(identity != null, "战斗阶段右栏顶部应存在身份块")
    if identity != null:
        var text := _node_text(identity)
        _expect(text.contains("机器人C"), "战斗阶段应显示对手英雄名")
        _expect(text.contains("HP"), "战斗阶段应显示对手 HP 行")
    var status = instance.find_child("SideStatus", true, false)
    _expect(status != null, "战斗阶段右栏中部应存在回合与倒计时块")
    if status != null:
        _expect(_node_text(status).contains("剩余"), "战斗阶段应显示剩余时间")

    await _unmount(instance)


func _check_eliminated_identity() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["players"][0]["alive"] = false
    var instance := await _mount(view)
    if instance == null:
        return

    var identity = instance.find_child("SideIdentity", true, false)
    _expect(identity != null, "已淘汰观战者应有身份块")
    if identity != null:
        _expect(
            _node_text(identity).contains("已淘汰 · 观战中"),
            "已淘汰观战者应显示「已淘汰 · 观战中」",
        )

    await _unmount(instance)


func _check_ghost_identity() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["opponentPlayerId"] = "p2"
    view["players"][2]["alive"] = false
    var combat := FixtureScript.combat_result()
    combat["mirror"] = true
    var instance := await _mount(view, combat)
    if instance == null:
        return

    var identity = instance.find_child("SideIdentity", true, false)
    _expect(identity != null, "幽灵战应有身份块")
    if identity != null:
        _expect(_node_text(identity).contains("幽灵"), "幽灵战身份块应标注「幽灵」")
        _expect(
            identity.find_child("SideHpBar", true, false) == null,
            "幽灵战身份块不应显示血量条",
        )

    await _unmount(instance)
