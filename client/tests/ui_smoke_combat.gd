extends "res://tests/smoke_case.gd"
## 战斗视图冒烟：中区只剩对手随从、结果横幅、我方随从，不再有双方英雄条。
##
## 断言在演示播放中与播放结束两种状态下都要成立。

func _ready() -> void:
    await _check_combat_center_only()
    await _check_banner_variants()
    _finish()


func _check_combat_center_only() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    _expect_combat_center(instance, false)
    # 3 条动作 × 320ms + 2000ms 横幅停留之外再多推一拍，确保进入结束态。
    for _step in 10:
        instance._process(0.4)
    _expect_combat_center(instance, true)
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 横幅文案：平局夹具显示「平局」与「伤害 0」；幽灵战夹具追加「（幽灵）」。
func _check_banner_variants() -> void:
    var draw := FixtureScript.combat_result()
    draw["winnerId"] = null
    draw["damage"] = 0
    await _expect_banner(draw, ["平局", "伤害 0"], "平局")
    var ghost := FixtureScript.combat_result()
    ghost["mirror"] = true
    await _expect_banner(ghost, ["（幽灵）"], "幽灵战")


## 摆一份战报并推进到演示结束，断言横幅文案包含给定片段。
func _expect_banner(combat: Dictionary, expected: Array, stage: String) -> void:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_COMBAT
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view := FixtureScript.game_view()
    view["phase"] = Protocol.PHASE_COMBAT
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    _set_combat(combat)
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        _clear_combat()
        return
    for _step in 10:
        instance._process(0.4)
    var banner = instance.find_child("CombatBanner", true, false)
    _expect(banner != null and banner.visible, "%s横幅应可见" % stage)
    if banner != null:
        for piece in expected:
            _expect(
                banner.text.contains(str(piece)),
                "%s横幅应含「%s」，实际「%s」" % [stage, piece, banner.text]
            )
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 中区节点：
## - 两个英雄条必须不存在；
## - 结果横幅、对手随从行、我方随从行必须保留，且三者相对顺序固定；
## - 可见性与行内数量随播放状态变化。
func _expect_combat_center(instance: Node, finished: bool) -> void:
    var stage := "播放结束后" if finished else "播放中"
    _expect(instance.find_child("CombatOpponentStrip", true, false) == null, "战斗视图不应有对手英雄条（%s）" % stage)
    _expect(instance.find_child("CombatSelfStrip", true, false) == null, "战斗视图不应有我方英雄条（%s）" % stage)

    var banner = instance.find_child("CombatBanner", true, false)
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    var own_row = instance.find_child("CombatOwnRow", true, false)
    _expect(banner != null, "结果横幅保留（%s）" % stage)
    _expect(opponent_row != null, "对手随从行仍在（%s）" % stage)
    _expect(own_row != null, "我方随从行仍在（%s）" % stage)
    _expect_combat_row_order(instance)

    if banner == null:
        return
    if finished:
        _expect(banner.visible, "动作播完并停留后应显示结果横幅")
    else:
        _expect(not banner.visible, "动作播完前不应显示结果横幅")
        if opponent_row != null:
            _expect(opponent_row.get_child_count() == 2, "对手随从行应为 2 张")
        if own_row != null:
            _expect(own_row.get_child_count() == 3, "我方随从行应为 3 张")


## 中区三段在 `CombatBody` 下的相对顺序：对手行 → 横幅 → 我方行（标题 Label 可在其间）。
func _expect_combat_row_order(instance: Node) -> void:
    var body = instance.find_child("CombatBody", true, false)
    if body == null:
        _expect(false, "战斗视图应有 CombatBody")
        return
    var opponent_index := -1
    var banner_index := -1
    var own_index := -1
    for position in body.get_child_count():
        match str(body.get_child(position).name):
            "CombatOpponentRow":
                opponent_index = position
            "CombatBanner":
                banner_index = position
            "CombatOwnRow":
                own_index = position
    _expect(
        opponent_index >= 0 and banner_index >= 0 and own_index >= 0
            and opponent_index < banner_index and banner_index < own_index,
        "中区顺序应为对手随从行 → 结果横幅 → 我方随从行，实际 %d/%d/%d" % [
            opponent_index, banner_index, own_index,
        ]
    )
