extends "res://tests/smoke_case.gd"
## 座位行冒烟：头像 + 名次角标 + 血条、点击展开战报、展开态跨刷新记忆、血量夹紧。

func _ready() -> void:
    await _check_seat_rows()
    await _check_report_toggle()
    await _check_seat_detail()
    await _check_expanded_memory()
    await _check_hp_clamp()
    _finish()


## 战报夹具：p0 第 2 回合战胜 p2；p2 第 2 回合负于 p0，第 3 回合幽灵战负于 p5。
func _view_with_log() -> Dictionary:
    var view: Dictionary = FixtureScript.game_view()
    view["combatLog"] = [
        {
            "turn": 2,
            "participantIds": ["p0", "p2"],
            "winnerId": "p0",
            "loserId": "p2",
            "damage": 7,
            "mirror": false,
        },
        {
            "turn": 3,
            "participantIds": ["p2", "p5"],
            "winnerId": "p5",
            "loserId": null,
            "damage": 0,
            "mirror": true,
        },
    ]
    return view


func _setup(view: Dictionary) -> void:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_RECRUIT
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = view
    _clear_combat()


func _open(view: Dictionary) -> Node:
    _setup(view)
    return await _check_scene("res://scenes/game.tscn", false)


func _rows(rail: Node) -> Array:
    var rows: Array = []
    for child in rail.get_children():
        # 同一父节点下的同名节点会被 Godot 自动改名，用座位元数据认行。
        if child.has_meta("player_id"):
            rows.append(child)
    return rows


func _row_of(rail: Node, player_id: String) -> Node:
    for row in _rows(rail):
        if str(row.get_meta("player_id", "")) == player_id:
            return row
    return null


func _reports(instance: Node) -> Array:
    return instance.find_children("SeatReport", "", true, false)


func _check_seat_rows() -> void:
    var instance := await _open(_view_with_log())
    if instance == null:
        return
    var rail = instance.find_child("PlayersRail", true, false)
    if rail == null:
        _expect(false, "对局场景缺少积分榜容器 PlayersRail")
        instance.queue_free()
        return
    var rows := _rows(rail)
    _expect(rows.size() == 8, "积分榜应有 8 个座位行，实际 %d" % rows.size())
    for row in rows:
        _expect(row.find_child("Avatar", true, false) != null, "座位行缺少头像")
        _expect(row.find_child("SeatRankBadge", true, false) != null, "座位行缺少名次角标")
        _expect(row.find_child("SeatHpBar", true, false) != null, "座位行缺少血条")
        _expect(not _node_text(row).contains("HP"), "座位行内不应出现 HP 文本：%s" % _node_text(row))
    # 已淘汰仍然显示名次数字，整行压暗。
    var dead := _row_of(rail, "p7")
    var dead_badge = dead.find_child("SeatRankBadge", true, false) if dead != null else null
    _expect(dead_badge != null and _node_text(dead_badge).contains("8"), "淘汰座位仍应显示名次数字")
    _expect(dead != null and dead.modulate.a < 1.0, "淘汰座位整行应降透明度")
    instance.queue_free()
    await get_tree().process_frame


func _check_report_toggle() -> void:
    var instance := await _open(_view_with_log())
    if instance == null:
        return
    var rail = instance.find_child("PlayersRail", true, false)
    var row := _row_of(rail, "p0") if rail != null else null
    if row == null:
        _expect(false, "缺少 p0 座位行")
        instance.queue_free()
        return
    await _push_click(instance, row.get_global_rect().get_center())
    await get_tree().process_frame
    var reports := _reports(instance)
    _expect(reports.size() == 1, "点击座位行应展开一块 SeatReport，实际 %d" % reports.size())
    if reports.size() == 1:
        _expect(str(reports[0].get_meta("player_id", "")) == "p0", "展开的战报应属于被点击的座位")
        var text := _node_text(reports[0])
        _expect(text.contains("第 2 回合"), "战报应显示回合：%s" % text)
        _expect(text.contains("机器人C"), "战报应显示对手名：%s" % text)
        _expect(text.contains("胜"), "战报应显示胜负：%s" % text)
        _expect(text.contains("伤害 7"), "战报应显示伤害：%s" % text)
    # 再点同一行收起。
    row = _row_of(rail, "p0")
    await _push_click(instance, row.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(_reports(instance).is_empty(), "再次点击应收起战报")
    instance.queue_free()
    await get_tree().process_frame


func _check_seat_detail() -> void:
    var instance := await _open(_view_with_log())
    if instance == null:
        return
    var rail = instance.find_child("PlayersRail", true, false)
    var row := _row_of(rail, "p0") if rail != null else null
    if row == null:
        _expect(false, "缺少 p0 座位行")
        instance.queue_free()
        return
    await _hover(instance, row, row.size / 2)
    var panel = instance.find_child("MinionDetailPanel", true, false)
    _expect(panel != null and panel.is_visible_in_tree(), "悬停座位行应显示详情面板")
    var text := _node_text(panel) if panel != null else ""
    _expect(text.contains("玩家A"), "座位详情应显示名字：%s" % text)
    _expect(text.contains("35/40"), "座位详情应显示生命当前/上限：%s" % text)
    _expect(text.contains("史前"), "座位详情应显示时代名：%s" % text)
    _expect(text.contains("存活"), "座位详情应显示存活状态：%s" % text)
    instance.queue_free()
    await get_tree().process_frame


func _check_expanded_memory() -> void:
    var view := _view_with_log()
    var instance := await _open(view)
    if instance == null:
        return
    var rail = instance.find_child("PlayersRail", true, false)
    var row := _row_of(rail, "p2") if rail != null else null
    if row == null:
        _expect(false, "缺少 p2 座位行")
        instance.queue_free()
        return
    await _push_click(instance, row.get_global_rect().get_center())
    await get_tree().process_frame
    var reports := _reports(instance)
    _expect(reports.size() == 1, "点击 p2 应展开战报")
    if reports.size() == 1:
        var text := _node_text(reports[0])
        _expect(text.contains("幽灵（机器人F）"), "幽灵战对手名应写幽灵归属者：%s" % text)
        _expect(not text.contains("伤害 0"), "伤害为 0 时应省略伤害段：%s" % text)
    # 连续两次视图重建后仍按 playerId 恢复展开。
    for i in range(2):
        view["players"][2]["hp"] = 30 - i
        GameStore.view = view
        GameStore.view_changed.emit()
        await get_tree().process_frame
        await get_tree().process_frame
        var kept := _reports(instance)
        _expect(kept.size() == 1, "第 %d 次视图重建后战报应仍展开" % (i + 1))
        if kept.size() == 1:
            _expect(str(kept[0].get_meta("player_id", "")) == "p2", "重建后应恢复同一座位的战报")
    # 展开另一行时旧的先收起，同屏只有一块。
    row = _row_of(rail, "p4")
    await _push_click(instance, row.get_global_rect().get_center())
    await get_tree().process_frame
    var switched := _reports(instance)
    _expect(switched.size() == 1, "展开另一座位时同屏应只有一块战报")
    if switched.size() == 1:
        _expect(str(switched[0].get_meta("player_id", "")) == "p4", "新展开的战报应属于新座位")
        _expect(_node_text(switched[0]).contains("无战报"), "没有参与战报的座位应显示无战报")
    instance.queue_free()
    await get_tree().process_frame


func _check_hp_clamp() -> void:
    var view: Dictionary = FixtureScript.game_view()
    view["hpCap"] = 40
    view["players"][0]["hp"] = 999
    view["players"][2]["hp"] = 40
    view["players"][4]["hp"] = -5
    var instance := await _open(view)
    if instance == null:
        return
    var rail = instance.find_child("PlayersRail", true, false)
    var width := UITheme.metric("avatar-rail")
    for case in [["p0", width], ["p2", width], ["p4", 0]]:
        var row := _row_of(rail, case[0])
        var bar = row.find_child("SeatHpBar", true, false) if row != null else null
        var fill = bar.find_child("HpFill", true, false) if bar != null else null
        if fill == null:
            _expect(false, "%s 座位血条缺少 HpFill" % case[0])
            continue
        var fill_width: float = fill.size.x
        _expect(
            fill_width >= 0.0 and fill_width <= float(width),
            "%s 血条填充应在 [0, %d]，实际 %s" % [case[0], width, fill_width],
        )
        _expect(
            is_equal_approx(fill_width, float(case[1])),
            "%s 血条填充应为 %d，实际 %s" % [case[0], case[1], fill_width],
        )
    instance.queue_free()
    await get_tree().process_frame
