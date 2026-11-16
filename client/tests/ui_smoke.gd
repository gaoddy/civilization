extends Node
## 无头冒烟：实例化各场景，确认脚本、控件与关键展示无错误。

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
    await _check_room_null_hero()
    await _check_game()
    await _check_game_layout()
    await _check_combat_layout()
    await _check_hand_drag_emits_position()
    await _check_drag_rejected_for_target_effect_on_blank()
    await _check_drag_disabled_when_stale_or_expired()
    await _check_drag_cancelled_after_view_rebuild()
    await _check_combat_playback()
    await _check_combat_banner_after_tail()
    await _check_combat_resume_from_deadline()
    await _check_target_pending_overlay()
    await _check_choose_target_sends_intent()
    await _check_recruited_gate()
    await _check_expired_gate()
    await _check_stale_view_gate()
    await _check_disconnected_send()
    await _check_navigation_guard()
    await _check_error_route()
    await _check_reset_session()
    await _check_error_text()
    await _check_result()
    if _failures == 0:
        print("UI_SMOKE_OK")
        get_tree().quit(0)
    else:
        get_tree().quit(1)


func _expect(condition: bool, message: String) -> void:
    if not condition:
        push_error(message)
        _failures += 1


## 在卡片局部坐标处用真实鼠标移动事件模拟悬停。
func _hover(instance: Node, card: Control, local: Vector2) -> void:
    await _push_motion(instance, card.get_global_rect().position + local)


func _push_motion(instance: Node, canvas_point: Vector2) -> void:
    # 传入画布坐标并令 in_local_coords = true：Godot 在 _input 之前按画布变换
    # 换算位置（1280x720 视口被缩放到无头窗口），页面再按同一变换反算，
    # 与 get_global_rect() 同一坐标系。
    var event := InputEventMouseMotion.new()
    event.position = canvas_point
    event.global_position = canvas_point
    instance.get_viewport().push_input(event, true)
    await get_tree().process_frame


## 在画布坐标处用真实鼠标按下/松开事件点击。
func _push_click(instance: Node, canvas_point: Vector2) -> void:
    await _push_motion(instance, canvas_point)
    for pressed in [true, false]:
        var event := InputEventMouseButton.new()
        event.button_index = MOUSE_BUTTON_LEFT
        event.pressed = pressed
        event.position = canvas_point
        event.global_position = canvas_point
        instance.get_viewport().push_input(event, true)
        await get_tree().process_frame


## 战斗只读卡（CombatOwnRow / CombatOpponentRow）同样触发详情。
func _check_combat_detail(instance: Node) -> void:
    var detail = instance.find_child("MinionDetailPanel", true, false)
    var own_row = instance.find_child("CombatOwnRow", true, false)
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    if detail == null or own_row == null or opponent_row == null:
        _expect(false, "战斗视图缺少详情面板或随从行")
        return
    for row in [own_row, opponent_row]:
        if row.get_child_count() == 0:
            _expect(false, "战斗随从行为空")
            continue
        var card: Control = row.get_child(0)
        await _hover(instance, card, card.size / 2)
        _expect(detail.is_visible_in_tree(), "战斗卡 %s 悬停应显示详情面板" % card.name)
        _expect(detail.mouse_filter == Control.MOUSE_FILTER_IGNORE, "详情面板不应拦截鼠标")
    await _push_motion(instance, Vector2(-40, -40))
    _expect(not detail.is_visible_in_tree(), "移出战斗卡矩形应隐藏详情面板")


func _check_detail_without_stats() -> void:
    # stats 缺失（旧服务端 / 部分回放）时面板只显示当前攻防。
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"][0].erase("stats")
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var detail = instance.find_child("MinionDetailPanel", true, false)
    var card = instance.find_child("BoardMinion0", true, false)
    if detail == null or card == null:
        _expect(false, "对局场景缺少详情面板或棋盘随从")
    else:
        await get_tree().process_frame
        await _hover(instance, card, Vector2(card.size.x / 2, card.size.y * 0.3))
        _expect(detail.is_visible_in_tree(), "缺少 stats 时详情面板仍应显示")
        var text := _node_text(detail)
        _expect(text.contains("当前") and text.contains("3 / 4"), "缺少 stats 时应显示当前攻防：%s" % text)
        for absent in ["初始", "永久", "回合", "光环", "受击"]:
            _expect(not text.contains(absent), "缺少 stats 时不应出现「%s」：%s" % [absent, text])
    instance.queue_free()
    GameStore.view = {}
    GameStore.deadline = ""
    GameStore.turn = 1
    GameStore.phase = ""


func _node_text(node: Node) -> String:
    var text := ""
    for child in node.find_children("*", "Label", true, false):
        text += str(child.text) + "\n"
    return text


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


func _check_room_null_hero() -> void:
    GameStore.player_id = "p0"
    GameStore.room_id = "r_test"
    GameStore.room_code = "ABC123"
    GameStore.phase = "Lobby"
    GameStore.room_players = [{
        "playerId": "p0",
        "displayName": "玩家A",
        "ready": false,
        "isHost": true,
        "heroCardId": null,
    }]
    var instance := await _check_scene("res://scenes/room.tscn", false)
    if instance == null:
        return
    var list = instance.find_child("PlayerList", true, false)
    if list == null:
        _expect(false, "房间场景缺少 PlayerList")
    else:
        var text := ""
        for child in list.get_children():
            if child is Label:
                text += child.text
        _expect(text.contains("未选英雄"), "未选英雄的座位应显示「未选英雄」，实际：%s" % text)
        _expect(not text.contains("<null>"), "房间成员出现 <null>：%s" % text)
    instance.queue_free()
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.room_players = []


func _check_game() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.view = FixtureScript.view()
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    for index in range(6):
        var name := "ShopSlot%d" % index
        _expect(instance.find_child(name, true, false) != null, "对局场景缺少 %s" % name)
    var empty_slot = instance.find_child("ShopSlot2", true, false)
    if empty_slot != null:
        _expect(str(empty_slot.text).begins_with("空槽"), "空商店槽应显示「空槽」，实际：%s" % empty_slot.text)
    var detail = instance.find_child("MinionDetailPanel", true, false)
    var detail_layer = instance.find_child("MinionDetailLayer", true, false)
    _expect(detail != null, "对局页应有随从详情面板")
    _expect(detail != null and not detail.is_visible_in_tree(), "详情面板默认隐藏")
    _expect(detail_layer != null and not detail_layer.is_visible_in_tree(), "详情层默认隐藏")
    var card = instance.find_child("BoardMinion0", true, false)
    if detail != null and card != null:
        await _hover(instance, card, Vector2(card.size.x / 2, card.size.y * 0.3))
        _expect(detail.is_visible_in_tree(), "悬停棋盘随从应显示详情面板")
        var text := _node_text(detail)
        for expected in ["初始", "3 / 4", "永久", "+1 / +2", "回合", "+0 / -2", "光环", "+0 / +1", "受击", "0 / -3", "当前", "4 / 2"]:
            _expect(text.contains(expected), "详情面板应包含「%s」：%s" % [expected, text])
        _expect(not text.contains("-0"), "非负构成项应带 +：%s" % text)
        # 指针落在卡片内按钮区域（STOP 子控件）继续移动，面板保持可见。
        var actions = card.find_child("Actions", true, false)
        if actions != null and actions.get_child_count() > 0:
            var button: Control = actions.get_child(0)
            await _push_motion(instance, button.get_global_rect().get_center())
            _expect(detail.is_visible_in_tree(), "指针落在卡片按钮上时详情面板应保持可见")
        _expect(detail.mouse_filter == Control.MOUSE_FILTER_IGNORE, "详情面板不应拦截鼠标")
        # 详情面板覆盖按钮时只读：真实点击应穿透到下层按钮。
        if actions != null and actions.get_child_count() > 0:
            var sell: Button = actions.get_child(0)
            detail.position = sell.get_global_rect().position
            var hits := {"count": 0}
            sell.pressed.connect(func() -> void: hits["count"] += 1)
            await _push_click(instance, sell.get_global_rect().get_center())
            _expect(int(hits["count"]) == 1, "详情面板覆盖时点击应触发下层按钮")
        await _hover(instance, card, Vector2(card.size.x / 2, card.size.y + 24))
        _expect(not detail.is_visible_in_tree(), "移出棋盘随从矩形应隐藏详情面板")
    # 重新显示详情，再打开选择浮层，确认浮层会隐藏详情层。
    if detail != null and card != null:
        await _hover(instance, card, Vector2(card.size.x / 2, card.size.y * 0.3))
        _expect(detail.is_visible_in_tree(), "打开浮层前详情面板应可见")
    var power = instance.find_child("HeroPowerButton", true, false)
    if power == null:
        _expect(false, "对局场景缺少 HeroPowerButton")
    else:
        power.pressed.emit()
        await get_tree().process_frame
        var overlay = instance.find_child("ChooserOverlay", true, false)
        _expect(overlay != null and overlay.visible, "英雄技能应打开目标选择弹层")
        _expect(detail != null and not detail.is_visible_in_tree(), "打开选择浮层时详情面板应隐藏")
        var has_skip := false
        if overlay != null:
            for child in overlay.find_children("*", "Button", true, false):
                if child is Button and str(child.text) == "不指定目标":
                    has_skip = true
        _expect(has_skip, "英雄技能弹层应提供「不指定目标」")
    instance.queue_free()
    await get_tree().process_frame
    await _check_detail_without_stats()
    GameStore.view = {}
    GameStore.turn = 1


func _check_game_layout() -> void:
    # 招募阶段：积分榜 8 座、中区自上而下为商店/我方棋盘/手牌、棋盘 7 槽、右栏英雄卡；战斗视图隐藏。
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = FixtureScript.game_view()
    GameStore.last_combat = {}
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var rail = instance.find_child("PlayersRail", true, false)
    _expect(rail != null and rail.get_child_count() == 8, "积分榜应有 8 个座位")
    var recruit = instance.find_child("RecruitPanel", true, false)
    var combat = instance.find_child("CombatPanel", true, false)
    _expect(recruit != null and recruit.visible, "招募阶段应显示招募视图")
    _expect(combat != null and not combat.visible, "招募阶段不应显示战斗视图")
    _expect(
        instance.find_child("OpponentsPanel", true, false) == null,
        "招募中区不含对手面板，对手信息只在积分榜",
    )
    _expect(
        instance.find_child("OpponentsRow", true, false) == null,
        "招募中区不含对手席位行",
    )
    var shop = instance.find_child("ShopPanel", true, false)
    var board_panel = instance.find_child("BoardPanel", true, false)
    var hand = instance.find_child("HandPanel", true, false)
    _expect(shop != null and shop.get_parent() == recruit, "商店应位于招募中区")
    _expect(board_panel != null and board_panel.get_parent() == recruit, "我方棋盘应位于招募中区")
    var center = instance.find_child("CenterPanel", true, false)
    _expect(hand != null and hand.get_parent() == center, "手牌应位于中区（招募与战斗共用）")
    if shop != null and board_panel != null:
        _expect(
            shop.get_index() < board_panel.get_index(),
            "招募中区顺序应为商店、我方棋盘",
        )
    if hand != null and combat != null:
        _expect(hand.get_index() > combat.get_index(), "手牌应位于战斗视图之下")
    var board = instance.find_child("BoardRow", true, false)
    _expect(board != null and board.get_child_count() == 7, "棋盘应有 7 个槽位（含空位）")
    _expect(instance.find_child("HeroTile", true, false) != null, "右栏应显示我的英雄卡")
    instance.queue_free()
    GameStore.view = {}
    GameStore.last_combat = {}
    GameStore.turn = 1


func _check_combat_layout() -> void:
    # 战斗阶段：战斗视图替换招募视图，双方随从行与结果横幅取自 CombatResult。
    GameStore.player_id = "p0"
    GameStore.phase = "Combat"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view := FixtureScript.game_view()
    view["phase"] = "Combat"
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    GameStore.last_combat = FixtureScript.combat_result()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var recruit = instance.find_child("RecruitPanel", true, false)
    var combat = instance.find_child("CombatPanel", true, false)
    _expect(combat != null and combat.visible, "战斗阶段应显示战斗视图")
    _expect(recruit != null and not recruit.visible, "战斗阶段不应显示招募视图")
    var hand_panel = instance.find_child("HandPanel", true, false)
    _expect(hand_panel != null and hand_panel.visible, "演示期间手牌仍应可见")
    var banner = instance.find_child("CombatBanner", true, false)
    _expect(banner != null and not banner.visible, "动作播完前不应显示结果横幅")
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    _expect(opponent_row != null and opponent_row.get_child_count() == 2, "对手随从行应为 2 张")
    var own_row = instance.find_child("CombatOwnRow", true, false)
    _expect(own_row != null and own_row.get_child_count() == 3, "我方随从行应为 3 张")
    var end_button = instance.find_child("EndButton", true, false)
    _expect(end_button != null and not end_button.visible, "战斗阶段不应显示结束招募按钮")
    var power_button = instance.find_child("HeroPowerButton", true, false)
    _expect(power_button != null and not power_button.visible, "演示期间不应显示英雄技能按钮")
    _expect(instance.find_child("ShopPanel", true, false) != null, "商店节点仍在招募面板内")
    var hand_card = instance.find_child("HandCard0", true, false)
    _expect(hand_card != null and not hand_card.drag_allowed, "演示期间手牌不可拖动")
    await _check_combat_no_skip_entry(instance)
    await _check_combat_detail(instance)
    instance.queue_free()
    GameStore.view = {}
    GameStore.last_combat = {}
    GameStore.turn = 1
    GameStore.phase = ""


## 演示期间没有跳过或加速入口。
func _check_combat_no_skip_entry(instance: Node) -> void:
    for child in instance.find_children("*", "Button", true, false):
        var text := str((child as Button).text)
        _expect(not text.contains("跳过"), "演示不应提供「跳过」：%s" % text)
        _expect(not text.contains("加速"), "演示不应提供「加速」：%s" % text)


## 构造战斗阶段的场景，返回实例（调用方负责释放）。
func _combat_instance(deadline: String) -> Node:
    GameStore.player_id = "p0"
    GameStore.phase = "Combat"
    GameStore.turn = 3
    GameStore.deadline = deadline
    var view := FixtureScript.game_view()
    view["phase"] = "Combat"
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    GameStore.last_combat = FixtureScript.combat_result()
    return await _check_scene("res://scenes/game.tscn", false)


func _stats_text(panel: Control) -> String:
    var stats = panel.find_child("Stats", true, false)
    return "" if stats == null else str(stats.text)


## 逐动作播放：每 320ms 应用一条动作，数值只取服务端下发值。
func _check_combat_playback() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    var own_row = instance.find_child("CombatOwnRow", true, false)
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    if own_row == null or opponent_row == null:
        _expect(false, "战斗视图缺少随从行")
        instance.queue_free()
        return
    instance._process(0.32)
    _expect(_stats_text(own_row.get_child(0)) == "7 / 6", "第一条攻击后我方首卡应为 7/6，实际 %s" % _stats_text(own_row.get_child(0)))
    _expect(_stats_text(opponent_row.get_child(0)) == "6 / 2", "第一条攻击后对手首卡应为 6/2，实际 %s" % _stats_text(opponent_row.get_child(0)))
    instance._process(0.32)
    _expect(opponent_row.get_child_count() == 2, "阵亡随从仍占位")
    _expect(opponent_row.get_child(0).self_modulate.a < 1.0, "阵亡随从应变暗")
    instance._process(0.32)
    _expect(own_row.get_child_count() == 4, "召唤后我方行应为 4 张，实际 %d" % own_row.get_child_count())
    if own_row.get_child_count() == 4:
        _expect(str(own_row.get_child(1).get_meta("minion_id", "")) == "mi_9", "召唤随从应插到下标 1")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.last_combat = {}
    GameStore.turn = 1
    GameStore.phase = ""


## 播完动作并经过 TAIL_MS 后才显示胜负横幅。
func _check_combat_banner_after_tail() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    var banner = instance.find_child("CombatBanner", true, false)
    for _step in range(4):
        instance._process(0.32)
    _expect(banner != null and not banner.visible, "动作刚播完且未到横幅停留时不应显示横幅")
    instance._process(2.0)
    _expect(banner != null and banner.visible, "横幅停留结束后应显示横幅")
    _expect(banner != null and str(banner.text).contains("胜利"), "横幅应显示胜负：%s" % ("" if banner == null else str(banner.text)))
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.last_combat = {}
    GameStore.turn = 1
    GameStore.phase = ""


## 演示中重连：按 phaseDeadline 反推进度，已播动作不重播。
func _check_combat_resume_from_deadline() -> void:
    # 总时长 = 3 × 320 + 2000 = 2960ms；`phaseDeadline` 只剩 2 秒时，1 秒
    # 前就该播完 3 条动作并留在横幅停留里——续播点直接落到 960ms，不从头重播。
    var deadline := Time.get_datetime_string_from_unix_time(
        int(Time.get_unix_time_from_system()) + 2, true
    ) + "Z"
    var instance := await _combat_instance(deadline)
    if instance == null:
        return
    var own_row = instance.find_child("CombatOwnRow", true, false)
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    var banner = instance.find_child("CombatBanner", true, false)
    _expect(
        own_row != null and own_row.get_child_count() == 4,
        "续播应按已过时间落地召唤：实际 %d 张" % (0 if own_row == null else own_row.get_child_count())
    )
    _expect(opponent_row != null and opponent_row.get_child_count() == 2, "对手阵亡随从仍占位")
    if opponent_row != null and opponent_row.get_child_count() > 0:
        _expect(opponent_row.get_child(0).self_modulate.a < 1.0, "续播点前的阵亡动作应立即落地")
    _expect(banner != null and not banner.visible, "横幅停留未走完时不应显示横幅")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.last_combat = {}
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 拖拽出牌：随从牌落在两张卡之间，落点下标取指针左半边的那张卡。
func _check_hand_drag_emits_position() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [
        FixtureScript.board_minion("mi_1", "baiyi"),
        FixtureScript.board_minion("mi_2", "airplane"),
    ]
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    _expect(
        instance.drag_gate(Protocol.PHASE_RECRUIT, false, false, true, false, true),
        "招募阶段、视图新鲜、已连接且可打出时手牌应允许拖拽"
    )
    var hand_card = instance.find_child("HandCard0", true, false)
    _expect(hand_card != null, "对局场景缺少 HandCard0")
    var board_row = instance.find_child("BoardRow", true, false)
    _expect(board_row != null, "对局场景缺少 BoardRow")
    if hand_card != null and board_row != null:
        _expect(str(hand_card.payload.get("cardKind", "")) == "minion", "archimedes 是随从牌，cardKind 应为 minion")
        _expect(int(hand_card.payload.get("epoch", -1)) == int(instance._view_epoch), "拖拽数据应带当前视图版本")
        # 直接以「允许拖拽」的姿态验证落点换算：指针在第二张卡左半边。
        var payload: Dictionary = hand_card.payload.duplicate()
        var second: Control = null
        for child in board_row.get_children():
            if child is Control and child.has_meta("minion_id"):
                second = child
        _expect(second != null, "棋盘应有两张随从卡")
        if second != null:
            var local: Vector2 = board_row.get_global_transform().affine_inverse() * (second.get_global_rect().position + Vector2(4, 4))
            _expect(
                board_row._can_drop_data(local, payload),
                "随从牌应能落在随从卡上"
            )
            var captured: Array = []
            instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
            board_row._drop_data(local, payload)
            await get_tree().process_frame
            _expect(captured.size() == 1, "一次落点只应产生一条意图，实际 %d" % captured.size())
            if captured.size() == 1:
                var intent: Dictionary = captured[0]
                _expect(str(intent.get("kind", "")) == Protocol.INTENT_PLAY_CARD, "落点意图应为 PlayCard")
                _expect(int(intent.get("handIndex", -1)) == 0, "落点意图应带手牌下标")
                _expect(int(intent.get("position", -1)) == 0, "插到第二张卡左边应为位置 0")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 需要目标的效果牌只能落在随从卡上；空白落点拒绝。
func _check_drag_rejected_for_target_effect_on_blank() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [FixtureScript.board_minion("mi_1", "baiyi")]
    view["self"]["hand"] = [FixtureScript.hand("erfenfa", true)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var board_row = instance.find_child("BoardRow", true, false)
    var payload := {
        "epoch": 0,
        "handIndex": 0,
        "cardId": "erfenfa",
        "cardKind": "effect",
        "requiresTarget": true,
    }
    if board_row != null:
        var card: Control = null
        for child in board_row.get_children():
            if child is Control and child.has_meta("minion_id"):
                card = child
        _expect(card != null, "棋盘应有一张随从卡")
        if card != null:
            var on_card: Vector2 = board_row.get_global_transform().affine_inverse() * card.get_global_rect().get_center()
            _expect(board_row._can_drop_data(on_card, payload), "需要目标的效果牌应能落在随从卡上")
            var blank := Vector2(board_row.size.x + 200, board_row.size.y + 100)
            _expect(not board_row._can_drop_data(blank, payload), "需要目标的效果牌不应接受空白落点")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 视图过期、倒计时归零、未连接、目标待选或不可打出时，拖拽不开始。
func _check_drag_disabled_when_stale_or_expired() -> void:
    var gate: Callable = Callable(load("res://scenes/game.gd"), "drag_gate")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, true, false, true, false, true), "视图过期时不应允许拖拽")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, false, true, true, false, true), "倒计时归零时不应允许拖拽")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, false, false, false, false, true), "未连接时不应允许拖拽")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, false, false, true, true, true), "目标待选期间不应允许拖拽")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, false, false, true, false, false), "不可打出的手牌不应允许拖拽")
    _expect(not gate.call(Protocol.PHASE_COMBAT, false, false, true, false, true), "非招募阶段不应允许拖拽")

    # 不允许拖拽的手牌，`_get_drag_data` 直接返回 null（拖拽根本不开始）。
    var card := HandCard.new()
    card.payload = {"handIndex": 0, "cardKind": "minion"}
    card.drag_allowed = false
    _expect(card._get_drag_data(Vector2.ZERO) == null, "不允许拖拽时不应交出拖拽数据")
    card.free()


## 拖拽途中视图重建后，落点数据已过期，不应再发出意图。
func _check_drag_cancelled_after_view_rebuild() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var hand_card = instance.find_child("HandCard0", true, false)
    var board_row = instance.find_child("BoardRow", true, false)
    if hand_card != null and board_row != null:
        var payload: Dictionary = hand_card.payload.duplicate()
        GameStore.view_changed.emit()
        await get_tree().process_frame
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        var local: Vector2 = Vector2(4, 4)
        if board_row._can_drop_data(local, payload):
            board_row._drop_data(local, payload)
        await get_tree().process_frame
        _expect(captured.is_empty(), "视图重建后旧拖拽数据不应产生意图，实际 %d 条" % captured.size())
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 目标待选的视图夹具：队首是 target 待选，棋盘两个随从。
func _target_pending_view() -> Dictionary:
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [
        FixtureScript.board_minion("mi_1", "baiyi"),
        FixtureScript.board_minion("mi_2", "airplane"),
    ]
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["pendingChoices"] = [
        {"kind": "target", "sourceMinionId": "mi_1"},
    ]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    return view


## 目标待选：浮层只列候选随从，没有任何退出路径，且所有招募控件被阻塞。
func _check_target_pending_overlay() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = _target_pending_view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var overlay = instance.find_child("ChooserOverlay", true, false)
    _expect(overlay != null and overlay.visible, "目标待选应弹出选择浮层")
    if overlay != null:
        var buttons: Array = []
        for child in overlay.find_children("*", "Button", true, false):
            buttons.append(child)
        _expect(buttons.size() == 2, "浮层内应只有 2 个候选随从按钮，实际 %d" % buttons.size())
        var texts: Array = []
        for button in buttons:
            texts.append(str(button.text))
            _expect(not str(button.text).contains("取消"), "目标待选不应提供取消：%s" % button.text)
            _expect(not str(button.text).contains("不指定目标"), "目标待选不应提供跳过：%s" % button.text)
        _expect(texts.size() == 2 and str(texts[0]).begins_with("★ "), "触发待选的随从应被高亮：%s" % str(texts))
    for name in ["RerollButton", "UpgradeButton", "FreezeButton", "EndButton", "HeroPowerButton"]:
        var button = instance.find_child(name, true, false)
        _expect(button != null and button.disabled, "目标待选期间 %s 应禁用" % name)
    var hand_card = instance.find_child("HandCard0", true, false)
    _expect(hand_card != null and not hand_card.drag_allowed, "目标待选期间手牌不应允许拖拽")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 选中候选后只发一条 ChooseTarget。
func _check_choose_target_sends_intent() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = _target_pending_view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var overlay = instance.find_child("ChooserOverlay", true, false)
    var captured: Array = []
    instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
    if overlay != null:
        # 第二个候选是 mi_2（第一个是触发待选的 mi_1）。
        var candidates: Array = overlay.find_children("*", "Button", true, false)
        if candidates.size() >= 2:
            candidates[1].pressed.emit()
    await get_tree().process_frame
    _expect(captured.size() == 1, "选中候选只应发出 1 条意图，实际 %d" % captured.size())
    if captured.size() == 1:
        var intent: Dictionary = captured[0]
        _expect(str(intent.get("kind", "")) == Protocol.INTENT_CHOOSE_TARGET, "应发出 ChooseTarget")
        _expect(str(intent.get("minionId", "")) == "mi_2", "应带上被选随从 id")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


func _check_recruited_gate() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["recruited"] = true
    view["self"]["pendingChoices"] = [{"kind": "discover", "candidates": ["huo", "lun", "che"]}]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var freeze = instance.find_child("FreezeButton", true, false)
    _expect(freeze != null and freeze.disabled, "已结束招募时冻结按钮应禁用")
    var overlay = instance.find_child("ChooserOverlay", true, false)
    _expect(overlay != null and not overlay.visible, "已结束招募时不应弹出待选弹层")
    instance.queue_free()
    GameStore.view = {}
    GameStore.turn = 1


func _check_expired_gate() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2000-01-01T00:00:00Z"
    GameStore.view = FixtureScript.view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var end_button = instance.find_child("EndButton", true, false)
    _expect(end_button != null and end_button.disabled, "倒计时归零后结束招募按钮应禁用")
    var reroll = instance.find_child("RerollButton", true, false)
    _expect(reroll != null and reroll.disabled, "倒计时归零后刷新按钮应禁用")
    instance.queue_free()
    GameStore.view = {}
    GameStore.deadline = ""


func _check_stale_view_gate() -> void:
    # 新回合的 PhaseChanged 先于新视图到达时，旧视图不能用来发起操作。
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 4
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = FixtureScript.view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var end_button = instance.find_child("EndButton", true, false)
    _expect(end_button != null and end_button.disabled, "视图与当前回合不一致时结束招募按钮应禁用")
    var reroll = instance.find_child("RerollButton", true, false)
    _expect(reroll != null and reroll.disabled, "视图与当前回合不一致时刷新按钮应禁用")
    instance.queue_free()
    GameStore.view = {}
    GameStore.turn = 1
    GameStore.deadline = ""


func _check_disconnected_send() -> void:
    # 未连接时点击操作要明确提示「未发出」，不能静默吞掉。
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.view = FixtureScript.view()
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var reroll = instance.find_child("RerollButton", true, false)
    var status = instance.find_child("StatusLabel", true, false)
    if reroll == null or status == null:
        _expect(false, "对局场景缺少 RerollButton 或 StatusLabel")
    else:
        reroll.pressed.emit()
        await get_tree().process_frame
        _expect(str(status.text).contains("未连接"), "未连接时操作应提示未发出，实际：%s" % status.text)
    instance.queue_free()
    GameStore.view = {}
    GameStore.turn = 1
    GameStore.deadline = ""


func _check_navigation_guard() -> void:
    # 页面被切走后仍连着 GameStore 信号：再次触发导航时不能访问已失效的场景树。
    GameStore.player_id = "p0"
    GameStore.room_id = "r_test"
    GameStore.room_code = "ABC123"
    GameStore.phase = "Recruit"
    GameStore.view = FixtureScript.view()
    var instance := await _check_scene("res://scenes/room.tscn", false)
    if instance == null:
        return
    var before = get_tree().current_scene
    remove_child(instance)
    _expect(not instance.is_inside_tree(), "移出树后房间场景应已离树")
    _expect(not UI.navigate(instance, "res://scenes/game.tscn"), "已离树的页面不应再执行导航")
    GameStore.view_changed.emit()
    _expect(get_tree().current_scene == before, "已离树的房间场景不应改变当前场景")
    instance.queue_free()
    GameStore.view = {}
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.phase = ""


func _check_error_route() -> void:
    var instance := await _check_scene("res://scenes/room.tscn", true)
    if instance == null:
        return
    if not instance.has_method("_error_route"):
        _expect(false, "房间场景缺少 _error_route")
        return
    _expect(instance._error_route("NOT_IN_ROOM") == "lobby", "NOT_IN_ROOM 应回大厅")
    _expect(instance._error_route("ROOM_NOT_FOUND") == "lobby", "ROOM_NOT_FOUND 应回大厅")
    _expect(instance._error_route("NOT_ENOUGH_GOLD") == "", "规则错误不应切场景")
    _expect(instance._error_route("TOKEN_EXPIRED") == "login", "令牌过期应回登录")


func _check_reset_session() -> void:
    GameStore.player_id = "p0"
    GameStore.token = "t0"
    GameStore.room_id = "r0"
    GameStore.room_code = "ABC123"
    GameStore.view = FixtureScript.view()
    GameStore.reset_session()
    _expect(GameStore.player_id == "", "reset_session 应清空 player_id")
    _expect(GameStore.token == "", "reset_session 应清空 token")
    _expect(GameStore.room_id == "", "reset_session 应清空 room_id")
    _expect(GameStore.view.is_empty(), "reset_session 应清空 view")


func _check_error_text() -> void:
    _expect(UI.error_text("NOT_ENOUGH_GOLD") == "金币不足", "NOT_ENOUGH_GOLD 应有中文文案")
    _expect(UI.error_text("INVALID_STAGE") == "当前阶段不能执行该操作", "INVALID_STAGE 应有中文文案")
    _expect(UI.error_text("SOME_UNKNOWN_CODE") == "SOME_UNKNOWN_CODE", "未知错误码应回退为原文")


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
        _expect(false, "结算场景缺少 RankingList")
    else:
        _expect(list.get_child_count() == 2, "结算场景排名行数为 %d，期望 2" % list.get_child_count())
    instance.queue_free()
    GameStore.ranking = []
    GameStore.game_seed = ""
