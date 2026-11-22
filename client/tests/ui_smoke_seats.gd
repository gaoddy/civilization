extends "res://tests/smoke_case.gd"
## 座位行冒烟：头像 + 名次角标 + 血条、悬停详情内的身份与对战记录、血量夹紧与名次重排。

func _ready() -> void:
    await _check_seat_rows()
    await _check_seat_detail()
    await _check_detail_variants()
    await _check_detail_position_composed()
    await _check_hp_clamp()
    await _check_live_rank_order()
    _finish()


## 战报夹具：p0 第 2 回合战胜 p2；p2 第 3 回合幽灵战负于 p5 的幽灵；
## p3 第 4 回合的幽灵战缺少归属者（ghostId 为 null）。
func _view_with_log() -> Dictionary:
    var view: Dictionary = FixtureScript.game_view()
    view["combatLog"] = [
        {
            "turn": 2,
            "participantIds": ["p0", "p2"],
            "ghostId": null,
            "winnerId": "p0",
            "loserId": "p2",
            "damage": 7,
            "mirror": false,
        },
        {
            "turn": 3,
            "participantIds": ["p2"],
            "ghostId": "p5",
            "winnerId": null,
            "loserId": "p2",
            "damage": 5,
            "mirror": true,
        },
        {
            "turn": 4,
            "participantIds": ["p3"],
            "ghostId": null,
            "winnerId": "p3",
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
    return await _check_scene("res://pages/game/game.tscn", false)


## 缩放内容框下的对局实例：宿主 1280×480 让内容框原点非零、缩放非 1。
## 返回对局实例，宿主为其父节点，调用方负责释放宿主。
func _open_composed(view: Dictionary) -> Node:
    _setup(view)
    var host := Control.new()
    host.name = "ComposedGameHost"
    host.size = Vector2(1280, 480)
    add_child(host)
    var packed = load("res://pages/game/game.tscn")
    if packed == null:
        _expect(false, "无法加载对局场景")
        host.queue_free()
        return null
    var instance = packed.instantiate()
    host.add_child(instance)
    await get_tree().process_frame
    await get_tree().process_frame
    return instance


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


## 行内文本：座位行自身不显示名字、血量与时代，这些只在悬停详情里出现。
## 头像与血条都可以是纯图形；未定名次的行没有角标文本，空串是合法结果。
func _fixed_text(row: Node) -> String:
    var text := ""
    for child in row.find_children("*", "Label", true, false):
        text += str(child.text) + "\n"
    return text


func _check_seat_rows() -> void:
    var view := _view_with_log()
    view["players"][1]["rank"] = null
    var instance := await _open(view)
    if instance == null:
        return
    var panel = instance.find_child("PlayersPanel", true, false)
    var rail = instance.find_child("PlayersRail", true, false)
    if rail == null:
        _expect(false, "对局场景缺少积分榜容器 PlayersRail")
        instance.queue_free()
        return
    _expect(
        panel != null and not _node_text(panel).contains("积分榜"),
        "左栏不再显示「积分榜」小标题",
    )
    # 首帧行高按兜底头像边长排布，左栏尺寸解算后异步重排；等重排落位再取行与断言。
    await get_tree().process_frame
    await get_tree().process_frame
    var rows := _rows(rail)
    _expect(rows.size() == 8, "积分榜应有 8 个座位行，实际 %d" % rows.size())
    var names := ["玩家A", "机器人B", "机器人C", "机器人D", "机器人E", "机器人F", "机器人G", "机器人H"]
    var fallback_side := UITheme.stage_metric("avatar-rail")
    var hp_inset := UITheme.space("space-2xs")
    if not rows.is_empty():
        var rail_rect: Rect2 = (rail as Control).get_global_rect()
        _expect(
            not (rail.get_parent() is ScrollContainer),
            "左栏座位列不应使用滚动容器",
        )
        var first_rect: Rect2 = (rows[0] as Control).get_global_rect()
        var last_rect: Rect2 = (rows[rows.size() - 1] as Control).get_global_rect()
        _expect(
            rail_rect.grow(1.0).encloses(first_rect),
            "首行应落在左栏内：行 %s，左栏 %s" % [str(first_rect), str(rail_rect)],
        )
        _expect(
            rail_rect.grow(1.0).encloses(last_rect),
            "末行应落在左栏内：行 %s，左栏 %s" % [str(last_rect), str(rail_rect)],
        )
        var min_side := first_rect.size.x
        var max_side := first_rect.size.x
        for row in rows:
            var rect: Rect2 = (row as Control).get_global_rect()
            _expect(
                is_equal_approx(rect.size.x, rect.size.y),
                "座位行应为正方形：%s" % str(rect.size),
            )
            min_side = minf(min_side, rect.size.x)
            max_side = maxf(max_side, rect.size.x)
        _expect(
            max_side - min_side <= 1.0,
            "8 个座位行应等边长：最大 %.1f，最小 %.1f" % [max_side, min_side],
        )
    for row in rows:
        _expect(row.find_child("Avatar", true, false) != null, "座位行缺少头像")
        var seat_clip := row.find_child("AvatarFrameClip", true, false) as CanvasItem
        var seat_border := row.find_child("AvatarFrameBorder", true, false) as Control
        _expect(
            seat_clip != null and seat_clip.clip_children == CanvasItem.CLIP_CHILDREN_AND_DRAW,
            "座位头像应同时绘制封面并裁切子节点",
        )
        _expect(
            seat_border != null and seat_clip != null
                and seat_border.get_index() > seat_clip.get_index(),
            "座位头像圆角描边应叠在头像之上",
        )
        var avatar_box: Control = row.find_child("SeatAvatar", true, false)
        var bar: Control = row.find_child("SeatHpBar", true, false)
        _expect(bar != null, "座位行缺少血条")
        if avatar_box != null and bar != null:
            _expect(
                bar.get_parent() == avatar_box,
                "血条应叠在头像块内，实际父节点 %s" % str(bar.get_parent()),
            )
            var avatar_rect := avatar_box.get_global_rect()
            var bar_rect := bar.get_global_rect()
            var row_rect := (row as Control).get_global_rect()
            _expect(
                is_equal_approx(row_rect.size.x - avatar_rect.size.x, 4.0),
                "行边长应为头像边长加两侧描边：行 %.1f，头像 %.1f"
                % [row_rect.size.x, avatar_rect.size.x],
            )
            _expect(
                is_equal_approx(avatar_rect.size.x, avatar_rect.size.y),
                "头像应为正方形：%s" % str(avatar_rect.size),
            )
            _expect(
                avatar_rect.size.x > float(fallback_side),
                "头像应随行高放大，实际边长 %.1f 不大于兜底 %d"
                % [avatar_rect.size.x, fallback_side],
            )
            _expect(
                avatar_rect.encloses(bar_rect),
                "血条应完全位于头像范围内：血条 %s，头像 %s" % [str(bar_rect), str(avatar_rect)],
            )
            _expect(
                is_equal_approx(bar.size.x, avatar_rect.size.x - float(hp_inset * 2)),
                "血条宽度应为头像宽减 %d，实际 %s" % [hp_inset * 2, bar.size.x],
            )
            _expect(
                is_equal_approx(bar_rect.end.y, avatar_rect.end.y - float(hp_inset)),
                "血条底边应距头像底边 %d：血条底 %.1f，头像底 %.1f"
                % [hp_inset, bar_rect.end.y, avatar_rect.end.y],
            )
            var avatar_node: Control = avatar_box.find_child("Avatar", true, false)
            _expect(
                avatar_node != null and bar.get_index() > avatar_node.get_index(),
                "血条层级应位于头像之上",
            )
            var avatar_clip: Panel = null
            if avatar_node != null:
                avatar_clip = avatar_node.find_child("AvatarFrameClip", true, false) as Panel
            var avatar_cover: StyleBoxFlat = null
            if avatar_clip != null:
                avatar_cover = avatar_clip.get_theme_stylebox("panel") as StyleBoxFlat
            _expect(
                avatar_cover != null
                and avatar_cover.corner_radius_top_left == UITheme.radius("radius-md"),
                "头像块圆角应取 radius-md，实际 %s"
                % (avatar_cover.corner_radius_top_left if avatar_cover != null else "缺失"),
            )
        var fixed := _fixed_text(row)
        _expect(not fixed.contains("HP"), "座位行内不应出现 HP 文本：%s" % fixed)
        _expect(
            not fixed.contains(
                TranslationServer.translate("ui.game.era_format").split("%")[0].strip_edges()
            ),
            "座位行内不应出现时代：%s" % fixed,
        )
        _expect(
            not fixed.contains(TranslationServer.translate("enum.card_type.minion")),
            "座位行内不应出现随从数：%s" % fixed,
        )
        _expect(not fixed.contains("minion"), "座位行内不应回退为类型 id：%s" % fixed)
        for player_name in names:
            _expect(not fixed.contains(player_name), "座位行内不应出现玩家名：%s" % fixed)
    # rank 为 null 时不显示名次角标；其余行仍有角标。
    var unranked := _row_of(rail, "p1")
    _expect(
        unranked != null and unranked.find_child("SeatRankBadge", true, false) == null,
        "rank 为 null 的座位不应显示名次角标",
    )
    for player_id in ["p0", "p2", "p7"]:
        var ranked := _row_of(rail, player_id)
        _expect(
            ranked != null and ranked.find_child("SeatRankBadge", true, false) != null,
            "%s 有 rank 时应显示名次角标" % player_id,
        )
        if ranked == null:
            continue
        var badge_avatar := ranked.find_child("SeatAvatar", true, false) as Control
        var badge := ranked.find_child("SeatRankBadge", true, false) as Control
        _expect(
            badge_avatar != null and badge != null
                and badge_avatar.get_global_rect().encloses(badge.get_global_rect()),
            "%s 名次角标应收在头像可选区域内：头像 %s，角标 %s"
            % [
                player_id,
                str(badge_avatar.get_global_rect()) if badge_avatar != null else "缺失",
                str(badge.get_global_rect()) if badge != null else "缺失",
            ],
        )
        if badge != null:
            var badge_box := badge.get_theme_stylebox("panel") as StyleBoxFlat
            _expect(
                badge_box != null
                    and badge_box.corner_radius_top_left * 2 == int(badge.size.x),
                "名次角标应为圆形，当前圆角 %s，边长 %s"
                % [
                    str(badge_box.corner_radius_top_left) if badge_box != null else "缺失",
                    str(badge.size.x),
                ],
            )
            var rank_text := badge.find_child("RankText", true, false) as Label
            _expect(
                rank_text != null
                    and rank_text.get_theme_color("font_color") == UITheme.color("color-white"),
                "名次角标数字应为白色",
            )
    # 已淘汰仍然显示名次数字，整行压暗。
    var dead := _row_of(rail, "p7")
    var dead_badge = dead.find_child("SeatRankBadge", true, false) if dead != null else null
    _expect(dead_badge != null and _node_text(dead_badge).contains("8"), "淘汰座位仍应显示名次数字")
    _expect(dead != null and dead.modulate.a < 1.0, "淘汰座位整行应降透明度")
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
    _expect(
        text.contains(TranslationServer.translate("ui.game.side_panel.rank") % 1),
        "座位详情顶部应显示名次：%s" % text,
    )
    _expect(
        text.contains(CardDB.display_name("darwin")),
        "座位详情顶部应显示英雄：%s" % text,
    )
    _expect(
        text.contains(CardDB.era_name(1)),
        "座位详情顶部应显示时代：%s" % text,
    )
    _expect(
        text.contains("%s %d" % [TranslationServer.translate("ui.game.detail.minions"), 4]),
        "座位详情顶部应显示随从数：%s" % text,
    )
    var hp_bar = panel.find_child("DetailHpBar", true, false) if panel != null else null
    var hp_label: Label = null
    if hp_bar != null:
        hp_label = hp_bar.find_child("HpLabel", true, false) as Label
    _expect(
        hp_label != null and hp_label.text == "35/40",
        "生命当前/上限应直接叠在生命条上：%s" % text,
    )
    _expect(
        hp_bar != null
            and is_equal_approx(
                hp_bar.size.y,
                float(UITheme.stage_metric("seat-detail-hp-bar-height")),
            ),
        "详情生命条高度应取 seat-detail-hp-bar-height，实际 %s"
        % (str(hp_bar.size.y) if hp_bar != null else "缺失"),
    )
    _expect(
        hp_label != null
            and hp_bar.get_global_rect().encloses(hp_label.get_global_rect()),
        "生命当前/上限数字应落在生命条范围内",
    )
    _expect(not text.contains("生命"), "座位详情不应再显示「生命」字样：%s" % text)
    for absent in [
        "ui.game.detail.alive",
        "ui.game.detail.eliminated",
        "ui.game.detail.status",
        "ui.game.detail.recruited",
        "ui.game.detail.not_recruited",
    ]:
        var absent_text := TranslationServer.translate(absent)
        _expect(
            not text.contains(absent_text),
            "座位详情不应再显示 %s：%s" % [absent, text],
        )
    _expect(not text.contains("对战记录"), "座位详情不应再显示「对战记录」标题：%s" % text)
    _expect(
        text.contains(TranslationServer.translate("ui.game.rail.turn") % 2),
        "对战记录应显示回合：%s" % text,
    )
    _expect(text.contains("机器人C"), "对战记录应显示对手名：%s" % text)
    _expect(
        text.contains(TranslationServer.translate("ui.game.rail.win")),
        "对战记录应显示胜负：%s" % text,
    )
    _expect(
        text.contains(TranslationServer.translate("ui.game.rail.damage") % 7),
        "对战记录应显示伤害：%s" % text,
    )
    if panel != null:
        _expect(
            is_equal_approx(
                panel.get_combined_minimum_size().x,
                float(UITheme.stage_metric("seat-detail-width")),
            ),
            "座位详情宽度应取 seat-detail-width，实际 %.1f"
            % panel.get_combined_minimum_size().x,
        )
    # 点击座位不再展开独立战报：对战信息只随悬停出现。
    await _push_click(instance, row.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(
        instance.find_child("SeatReport", true, false) == null,
        "点击座位不应再展开独立战报",
    )
    _expect(
        instance.find_child("ReportLayer", true, false) == null,
        "页面不应再有战报浮层",
    )
    instance.queue_free()
    await get_tree().process_frame


## 对战记录的边界：幽灵带名字、幽灵无归属者、没有参与战报的座位；
## 指针移开后详情收起。
func _check_detail_variants() -> void:
    var instance := await _open(_view_with_log())
    if instance == null:
        return
    var rail = instance.find_child("PlayersRail", true, false)
    var cases := [
        ["p2", [
            TranslationServer.translate("ui.game.rail.turn") % 3,
            TranslationServer.translate("ui.game.rail.ghost_named") % "机器人F",
            TranslationServer.translate("ui.game.rail.lose"),
            TranslationServer.translate("ui.game.rail.damage") % 5,
        ]],
        ["p3", [
            TranslationServer.translate("ui.game.rail.ghost"),
            TranslationServer.translate("ui.game.rail.win"),
        ]],
        ["p4", [TranslationServer.translate("ui.game.rail.no_report")]],
    ]
    for case in cases:
        var row := _row_of(rail, str(case[0])) if rail != null else null
        if row == null:
            _expect(false, "缺少 %s 座位行" % case[0])
            continue
        await _hover(instance, row, row.size / 2)
        var panel = instance.find_child("MinionDetailPanel", true, false)
        var text := _node_text(panel) if panel != null else ""
        for expected in case[1]:
            _expect(
                text.contains(str(expected)),
                "%s 的详情应包含 %s：%s" % [case[0], expected, text],
            )
        if str(case[0]) == "p2" and panel != null:
            var log_box = panel.find_child("SeatCombatLog", true, false)
            var entry_names: Array = []
            if log_box != null:
                for line in log_box.get_children():
                    entry_names.append(str(line.name))
            _expect(
                entry_names == ["SeatCombatEntry_3", "SeatCombatEntry_2"],
                "对战记录应按回合倒序（最新在上），实际 %s" % str(entry_names),
            )
        if str(case[0]) == "p3":
            _expect(
                not text.contains(TranslationServer.translate("ui.game.rail.ghost_named") % ""),
                "ghostId 为 null 时不应出现空括号：%s" % text,
            )
    await _push_motion(instance, Vector2(-40, -40))
    var panel = instance.find_child("MinionDetailPanel", true, false)
    _expect(panel != null and not panel.is_visible_in_tree(), "指针移开后座位详情应收起")
    instance.queue_free()
    await get_tree().process_frame


## 内容框原点非零且整体缩放时，座位详情仍贴在行右侧并留在详情层内。
## 直接把全局坐标当局部坐标（忽略原点与缩放）的做法会在这里失败。
func _check_detail_position_composed() -> void:
    var instance := await _open_composed(_view_with_log())
    if instance == null:
        return
    var host := instance.get_parent() as Control
    var rail = instance.find_child("PlayersRail", true, false)
    var layer := instance.find_child("MinionDetailLayer", true, false) as Control
    var row := _row_of(rail, "p0") if rail != null else null
    if row == null or layer == null:
        _expect(false, "组合用例缺少 p0 座位行或详情层")
        host.queue_free()
        return
    var layer_scale := layer.get_global_transform().get_scale().x
    _expect(not is_equal_approx(layer_scale, 1.0),
        "组合用例详情层应有非 1 缩放，实际 %.4f" % layer_scale)
    _expect(not layer.get_global_position().is_zero_approx(),
        "组合用例详情层应有非零原点，实际 %s" % str(layer.get_global_position()))
    await _hover(instance, row, row.size / 2)
    var panel := instance.find_child("MinionDetailPanel", true, false) as Control
    if panel == null or not panel.visible:
        _expect(false, "组合用例未显示座位详情")
        host.queue_free()
        await get_tree().process_frame
        return
    var row_global: Rect2 = row.get_global_rect()
    var gap := float(UITheme.space("space-sm")) * layer_scale
    var expected := Vector2(row_global.end.x + gap, row_global.position.y)
    var actual := panel.get_global_rect().position
    _expect(actual.distance_to(expected) < 0.5,
        "座位详情应贴在座位行右侧：实际 %s 期望 %s" % [str(actual), str(expected)])
    _expect(layer.get_global_rect().grow(1.0).has_point(actual),
        "座位详情应落在详情层内：%s" % str(actual))
    host.queue_free()
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
    for case in [["p0", true], ["p2", true], ["p4", false]]:
        var row := _row_of(rail, case[0])
        var bar = row.find_child("SeatHpBar", true, false) if row != null else null
        var fill = bar.find_child("HpFill", true, false) if bar != null else null
        if fill == null:
            _expect(false, "%s 座位血条缺少 HpFill" % case[0])
            continue
        var width: float = bar.size.x
        var expected: float = width if case[1] else 0.0
        var fill_width: float = fill.size.x
        _expect(
            fill_width >= 0.0 and fill_width <= width,
            "%s 血条填充应在 [0, %.1f]，实际 %s" % [case[0], width, fill_width],
        )
        _expect(
            is_equal_approx(fill_width, expected),
            "%s 血条填充应为 %.1f，实际 %s" % [case[0], expected, fill_width],
        )
    instance.queue_free()
    await get_tree().process_frame


## 名次与座位故意不同：行顺序必须按服务端 `rank` 升序，并随视图实时重排。
func _check_live_rank_order() -> void:
    var first: Dictionary = FixtureScript.game_view()
    _set_ranks(first, [5, 1, 3, 2, 7, 4, 8, 6])
    var instance := await _open(first)
    if instance == null:
        return
    var rail = instance.find_child("PlayersRail", true, false)
    if rail == null:
        _expect(false, "对局场景缺少积分榜容器 PlayersRail")
        instance.queue_free()
        return
    _expect_rank_order(rail, ["p1", "p3", "p2", "p5", "p0", "p7", "p4", "p6"])

    # 模拟一次 StateUpdate：名次重排、血量变化，行顺序、角标与血条都要实时更新。
    var second: Dictionary = FixtureScript.game_view()
    _set_ranks(second, [5, 2, 3, 4, 7, 6, 1, 8])
    second["players"][6]["hp"] = 40
    second["players"][0]["hp"] = 4
    GameStore.view = second
    GameStore.view_changed.emit()
    await get_tree().process_frame
    await get_tree().process_frame
    _expect_rank_order(rail, ["p6", "p1", "p2", "p3", "p0", "p5", "p4", "p7"])
    var top := _row_of(rail, "p6")
    _expect(
        top != null and _rank_text(top).contains("1"),
        "名次上升的座位应排到第一行并显示角标 1",
    )
    _expect_hp_fill(rail, "p6", 40, 40)
    _expect_hp_fill(rail, "p0", 4, 40)

    # `rank` 缺失的座位垫底，且不显示角标。
    var third: Dictionary = FixtureScript.game_view()
    _set_ranks(third, [5, 2, 3, 4, null, 6, 1, 8])
    GameStore.view = third
    GameStore.view_changed.emit()
    await get_tree().process_frame
    await get_tree().process_frame
    var order: Array = []
    for row in _rows(rail):
        order.append(str(row.get_meta("player_id", "")))
    _expect(
        order == ["p6", "p1", "p2", "p3", "p0", "p5", "p7", "p4"],
        "rank 缺失的座位应排在最后，实际 %s" % [order],
    )
    var unranked := _row_of(rail, "p4")
    _expect(
        unranked != null and unranked.find_child("SeatRankBadge", true, false) == null,
        "rank 缺失的座位不应显示角标",
    )
    instance.queue_free()
    await get_tree().process_frame


func _set_ranks(view: Dictionary, ranks: Array) -> void:
    for seat in range(view["players"].size()):
        view["players"][seat]["rank"] = ranks[seat]


func _rank_text(row: Node) -> String:
    var badge = row.find_child("SeatRankBadge", true, false)
    return _node_text(badge) if badge != null else ""


func _expect_rank_order(rail: Node, expected: Array) -> void:
    var rows := _rows(rail)
    var actual: Array = []
    for row in rows:
        actual.append(str(row.get_meta("player_id", "")))
    _expect(
        actual == expected,
        "积分榜应按名次升序排列，期望 %s，实际 %s" % [expected, actual],
    )
    for index in range(rows.size()):
        _expect(
            _rank_text(rows[index]).contains(str(index + 1)),
            "第 %d 行应显示名次 %d，实际 %s" % [index + 1, index + 1, _rank_text(rows[index])],
        )


func _expect_hp_fill(rail: Node, player_id: String, hp: int, cap: int) -> void:
    var row := _row_of(rail, player_id)
    var bar = row.find_child("SeatHpBar", true, false) if row != null else null
    var fill = bar.find_child("HpFill", true, false) if bar != null else null
    if fill == null:
        _expect(false, "%s 座位血条缺少 HpFill" % player_id)
        return
    var width := float(bar.size.x)
    var expected := roundf(width * float(hp) / float(cap))
    _expect(
        is_equal_approx(fill.size.x, expected),
        "%s 血条应随视图更新为 %s，实际 %s" % [player_id, expected, fill.size.x],
    )
