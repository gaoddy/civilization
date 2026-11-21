extends "res://tests/smoke_case.gd"
## 无头冒烟：实例化各场景，确认脚本、控件与关键展示无错误。

const SCENES := [
    "res://scenes/login.tscn",
    "res://scenes/lobby.tscn",
    "res://scenes/room.tscn",
]


func _ready() -> void:
    for path in SCENES:
        await _check_scene(path, true)
    await _check_room_null_hero()
    await _check_room_hero_picker()
    await _check_game()
    await _check_game_layout()
    await _check_hand_overlap_layout()
    await _check_hand_shows_server_stats()
    await _check_hand_reward_card_badge()
    await _check_hand_hover()
    await _check_hand_hover_overlap_strip()
    await _check_hand_hover_in_combat()
    await _check_board_capacity_from_view()
    await _check_board_stealth_badge()
    await _check_combat_layout()
    await _check_hand_drag_emits_position()
    await _check_hand_drag_cursor_rule()
    await _check_hand_drag_drops_on_board_blank()
    await _check_hand_drag_drops_on_minion_card()
    await _check_shop_drag_buys()
    await _check_drag_preview_keeps_pointer()
    await _check_board_drag_sells_over_shop_slot()
    await _check_board_drag_reorders()
    await _check_board_sell_drop()
    await _check_detail_hidden_while_dragging()
    await _check_drag_rejected_for_target_effect_on_blank()
    await _check_drag_rejected_when_board_full()
    await _check_drag_disabled_when_stale_or_expired()
    await _check_drag_cancelled_after_view_rebuild()
    await _check_drop_gate()
    await _check_combat_playback()
    await _check_combat_banner_after_tail()
    await _check_combat_resume_from_started_at()
    await _check_combat_playback_uses_sides()
    await _check_combat_playback_new_actions()
    await _check_combat_reborn_reappears()
    await _check_summon_inserts_at_alive_rank()
    await _check_stale_combat_not_shown()
    await _check_combat_placeholder_cleared()
    await _check_sfx_events()
    await _check_sfx_initial_shop_enter()
    await _check_sfx_freeze()
    await _check_sfx_combat()
    await _check_target_pending_aim()
    await _check_pending_aim_abandon()
    await _check_skill_aim()
    await _check_effect_drag_aim_visual()
    await _check_overlay_hidden_in_combat()
    await _check_last_combat_cleared_on_next_recruit()
    await _check_recruited_gate()
    await _check_expired_gate()
    await _check_stale_view_gate()
    await _check_disconnected_send()
    await _check_navigation_guard()
    await _check_error_route()
    await _check_reset_session()
    await _check_error_text()
    await _check_result()
    await _check_confirm_navigation()
    _finish()


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
        var expected_skill := CardDB.display_text("bruce-lee" if row == own_row else "aristotle")
        _expect(
            _node_text(detail).contains(expected_skill),
            "战斗卡 %s 的详情应显示介绍" % card.name,
        )
    await _push_motion(instance, Vector2(-40, -40))
    _expect(not detail.is_visible_in_tree(), "移出战斗卡矩形应隐藏详情面板")


func _check_detail_without_stats() -> void:
    # stats 缺失（旧服务端 / 部分回放）时面板只显示当前攻防。
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.game_view()
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
        _expect(
            text.contains(TranslationServer.translate("ui.game.detail.current")) and text.contains("3 / 4"),
            "缺少 stats 时应显示当前攻防：%s" % text,
        )
        _expect(
            text.contains(CardDB.display_text("bruce-lee")),
            "缺少 stats 时仍应显示介绍：%s" % text,
        )
        for absent in [
            TranslationServer.translate("enum.stat_layer.initial"),
            TranslationServer.translate("enum.stat_layer.permanent"),
            TranslationServer.translate("enum.stat_layer.aura"),
            TranslationServer.translate("enum.stat_layer.damage"),
        ]:
            _expect(not text.contains(absent), "缺少 stats 时不应出现「%s」：%s" % [absent, text])
    instance.queue_free()
    GameStore.view = {}
    GameStore.deadline = ""
    GameStore.turn = 1
    GameStore.phase = ""


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
    var others = instance.find_child("OtherPlayers", true, false)
    if others == null:
        _expect(false, "房间场景缺少 OtherPlayers")
    else:
        var text := _node_text(others)
        _expect(not text.contains("<null>"), "成员行出现 <null>：%s" % text)
    instance.queue_free()
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.room_players = []


## 选英雄页：顶部只显示他人名字与确认状态，底部是本人候选卡，悬停详情与选中框就位。
func _check_room_hero_picker() -> void:
    GameStore.player_id = "p0"
    GameStore.room_id = "r_test"
    GameStore.room_code = "ABC123"
    GameStore.phase = "Lobby"
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "ready": false, "isHost": true,
         "heroCardId": null, "heroCandidates": ["gauss", "mozi", "luban", "shenkuo"]},
        {"playerId": "p1", "displayName": "玩家B", "ready": true, "isHost": false,
         "heroCardId": "darwin", "heroCandidates": null},
    ]
    var instance := await _check_scene("res://scenes/room.tscn", false)
    if instance == null:
        return

    var others := instance.find_child("OtherPlayers", true, false)
    _expect(others != null, "缺少 OtherPlayers")
    var others_text := _node_text(others)
    _expect(others_text.contains("玩家B"), "顶部应显示其他玩家名字，实际：%s" % others_text)
    _expect(not others_text.contains("达尔文"), "顶部不得显示他人英雄，实际：%s" % others_text)
    _expect(
        others_text.contains(TranslationServer.translate("ui.room.confirmed")),
        "顶部应显示他人确认状态，实际：%s" % others_text,
    )

    var candidates := instance.find_child("HeroCandidates", true, false)
    _expect(candidates != null, "缺少 HeroCandidates")
    _expect(candidates != null and candidates.get_child_count() == 4,
        "本人候选应渲染 4 张卡，实际：%d" % (candidates.get_child_count() if candidates != null else 0))

    var selectable := 0
    if candidates != null:
        for card in candidates.get_children():
            if card.has_method("set_selected") and card.get("locked") != true:
                selectable += 1
    _expect(selectable == 4, "未确认时 4 张候选卡都应可选，实际：%d" % selectable)

    _expect(instance.find_child("ConfirmButton", true, false) != null, "缺少 ConfirmButton")
    var detail_layer := instance.find_child("HeroDetailLayer", true, false)
    _expect(detail_layer != null, "缺少 HeroDetailLayer")
    _expect(detail_layer != null and not detail_layer.visible, "悬停详情层初始应隐藏")
    var detail_panel := instance.find_child("RoomHeroDetail", true, false)
    _expect(detail_panel != null, "缺少 RoomHeroDetail")
    _expect(detail_panel != null and not detail_panel.visible, "悬停详情卡初始应隐藏")
    var status := instance.find_child("StatusLabel", true, false)
    _expect(status != null, "缺少 StatusLabel")

    # 房间码与离开房间入口不得出现在本页。
    var all_text := _node_text(instance)
    _expect(not all_text.contains("ABC123"), "选英雄页不得显示房间码，实际：%s" % all_text)
    _expect(instance.find_child("LeaveButton", true, false) == null, "选英雄页不得有离开房间入口")

    var confirm := instance.find_child("ConfirmButton", true, false)
    _expect(confirm != null and confirm.disabled, "未选中时确认按钮应禁用")

    # 布局：面板铺满整屏且尺寸固定，内容块在面板内居中；确认键与候选行同轴。
    var panel := instance.find_child("DraftPanel", true, false)
    _expect(panel != null, "缺少 DraftPanel")
    var column := instance.find_child("DraftColumn", true, false)
    _expect(column != null, "缺少 DraftColumn")
    if panel != null:
        var page_rect: Rect2 = instance.get_global_rect()
        var panel_rect: Rect2 = panel.get_global_rect()
        _expect(
            absf(panel_rect.size.x - page_rect.size.x) <= 1.0
            and absf(panel_rect.size.y - page_rect.size.y) <= 1.0,
            "面板应铺满整屏，实际 %s，页面 %s" % [panel_rect.size, page_rect.size],
        )
        if column != null:
            var column_rect: Rect2 = column.get_global_rect()
            _expect(column_rect.size.x < panel_rect.size.x,
                "内容块不应随面板铺满，实际宽度：%s" % column_rect.size.x)
            _expect(
                absf(column_rect.get_center().x - page_rect.get_center().x) <= 1.0
                and absf(column_rect.get_center().y - page_rect.get_center().y) <= 1.0,
                "内容块应在面板内居中，内容中心 %s，页面中心 %s" % [
                    column_rect.get_center(), page_rect.get_center(),
                ],
            )
    if confirm != null and candidates != null:
        _expect(
            absf(confirm.get_global_rect().get_center().x - candidates.get_global_rect().get_center().x) <= 1.0,
            "确认按钮应与候选行同轴居中，按钮中心 %s，候选行中心 %s" % [
                confirm.get_global_rect().get_center(), candidates.get_global_rect().get_center(),
            ],
        )

    # 悬停第一张候选卡：左右外侧显示详情，移出关闭；详情层不得拦截鼠标。
    if candidates != null and candidates.get_child_count() > 0:
        var first: Control = candidates.get_child(0)
        var hover_point := first.get_global_rect().get_center()
        await _hover(instance, first, first.size * 0.5)
        _expect(detail_layer != null and detail_layer.visible, "悬停候选卡应显示详情层")
        _expect(detail_panel != null and detail_panel.visible, "悬停候选卡应显示详情卡")
        _expect(
            detail_panel != null and detail_panel.find_child("HeroPortrait", true, false) == null,
            "悬停详情不应显示图片",
        )
        _expect(detail_layer != null and not _blocks_mouse(detail_layer), "悬停详情层及其子节点不得拦截鼠标点击")
        if detail_panel != null:
            var row_rect: Rect2 = candidates.get_global_rect()
            var left_rect: Rect2 = detail_panel.get_global_rect()
            _expect(not left_rect.intersects(row_rect), "左侧卡片的详情不应遮挡候选行")
            _expect(left_rect.end.x <= row_rect.position.x + 0.5, "左侧卡片的详情应挂在候选行左侧")

            var fourth: Control = candidates.get_child(candidates.get_child_count() - 1)
            await _hover(instance, fourth, fourth.size * 0.5)
            var right_rect: Rect2 = detail_panel.get_global_rect()
            _expect(not right_rect.intersects(row_rect), "右侧卡片的详情不应遮挡候选行")
            _expect(right_rect.position.x >= row_rect.end.x - 0.5, "右侧卡片的详情应挂在候选行右侧")

            # 详情卡尺寸必须跟随当前英雄内容收缩：面板不在容器内，只靠最小尺寸不会自动变矮。
            var heights: Array[float] = []
            for card in candidates.get_children():
                var body: Control = card
                await _hover(instance, body, body.size * 0.5)
                heights.append(detail_panel.get_combined_minimum_size().y)
            var shortest := 0
            for index in heights.size():
                if heights[index] < heights[shortest]:
                    shortest = index
            var short_card: Control = candidates.get_child(shortest)
            await _hover(instance, short_card, short_card.size * 0.5)
            _expect(
                absf(detail_panel.size.y - heights[shortest]) <= 1.0,
                "详情卡高度应等于当前英雄内容高度，实际 %s，期望 %s" % [
                    detail_panel.size.y, heights[shortest],
                ],
            )
        await _push_motion(instance, Vector2(4, 4))
        _expect(detail_panel != null and not detail_panel.visible, "移出候选卡应关闭详情")

        # 点击第一张候选卡：只加选中框，不弹居中浮层。
        await _push_click(instance, hover_point)
        var frame := candidates.get_child(0).find_child("SelectedFrame", true, false)
        _expect(frame != null and frame.is_visible_in_tree(), "点击候选卡应显示选中框")
        _expect(
            frame != null
            and frame.get_global_rect().size.x > candidates.get_child(0).get_global_rect().size.x
            and frame.get_global_rect().size.y > candidates.get_child(0).get_global_rect().size.y,
            "选中框应外扩到卡片之外",
        )
        await _push_click(instance, hover_point)
        frame = candidates.get_child(0).find_child("SelectedFrame", true, false)
        _expect(frame != null and frame.is_visible_in_tree(), "再次点击同一张应保持选中")
        await _push_click(instance, Vector2(4, 4))
        frame = candidates.get_child(0).find_child("SelectedFrame", true, false)
        _expect(frame != null and frame.is_visible_in_tree(), "点空白不应取消选中")
        _expect(detail_panel != null and not detail_panel.visible, "点空白应关闭悬停详情")

        # 单独验证「空白点击」路径：只推送按下/松开，不推送移动事件。
        var second_before := candidates.get_child(1)
        await _hover(instance, second_before, second_before.size * 0.5)
        _expect(detail_panel != null and detail_panel.visible, "空白点击前应先显示详情")
        await _push_button(instance, Vector2(4, 4), true)
        await _push_button(instance, Vector2(4, 4), false)
        _expect(detail_panel != null and not detail_panel.visible, "空白点击（无移动）应关闭悬停详情")

        # 点击另一张卡：选中框移动到新卡，原卡取消。
        var other_card: Control = candidates.get_child(2)
        await _push_click(instance, other_card.get_global_rect().get_center())
        var moved_frame := candidates.get_child(2).find_child("SelectedFrame", true, false)
        var old_frame := candidates.get_child(0).find_child("SelectedFrame", true, false)
        _expect(moved_frame != null and moved_frame.is_visible_in_tree(), "点击另一张卡应移动选中框")
        _expect(old_frame != null and not old_frame.is_visible_in_tree(), "点击另一张卡后原选中框应取消")
        await _push_click(instance, hover_point)

        # 候选重排：正在悬停的英雄仍在行内时，重建后应恢复悬停详情。
        var second: Control = candidates.get_child(1)
        await _hover(instance, second, second.size * 0.5)
        _expect(detail_panel != null and detail_panel.visible, "重排前悬停应显示详情")
        var reordered := GameStore.room_players.duplicate(true)
        reordered[0]["heroCandidates"] = ["darwin", "mozi", "luban", "shenkuo"]
        GameStore.room_players = reordered
        GameStore.room_changed.emit()
        await get_tree().process_frame
        await get_tree().process_frame
        _expect(detail_panel != null and detail_panel.visible, "候选重排后应恢复悬停详情")
        _expect(
            detail_panel != null and detail_panel.find_child("HeroPortrait", true, false) == null,
            "恢复的悬停详情也不应显示图片",
        )
        var restored := candidates.get_child(1) as RoomHeroCard
        var restored_style = restored.get_theme_stylebox("panel")
        _expect(
            restored_style != null
            and (restored_style as StyleBoxFlat).border_color == UITheme.color("color-border-strong"),
            "恢复悬停时卡片也应恢复悬停描边",
        )
        await _push_motion(instance, Vector2(4, 4))
        GameStore.room_players[0]["heroCandidates"] = ["gauss", "mozi", "luban", "shenkuo"]

        # 冒烟环境没有真实连接：确认按钮必须保持禁用，并给出未连接提示。
        _expect(confirm.disabled, "未连接时确认按钮必须禁用")
        _expect(
            str(status.text).contains(TranslationServer.translate("ui.room.not_connected")),
            "未连接时状态行应提示，实际：%s" % status.text,
        )

    instance.queue_free()

    # 断线要立刻反映到页面上：连接信号不经过 RoomUpdate，页面必须自己订阅。
    _fake_connect()
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "ready": false, "isHost": true,
         "heroCardId": null, "heroCandidates": ["gauss", "mozi", "luban", "shenkuo"]},
        {"playerId": "p1", "displayName": "玩家B", "ready": false, "isHost": false,
         "heroCardId": null, "heroCandidates": null},
    ]
    var online := await _check_scene("res://scenes/room.tscn", false)
    if online != null:
        var online_cards := online.find_child("HeroCandidates", true, false)
        var online_confirm := online.find_child("ConfirmButton", true, false)
        var online_status := online.find_child("StatusLabel", true, false)
        if online_cards != null and online_cards.get_child_count() > 0:
            var online_card: Control = online_cards.get_child(0)
            await _push_click(online, online_card.get_global_rect().get_center())
            _expect(online_confirm != null and not online_confirm.disabled,
                "已连接并选中后可确认")
            # 断线时 `Net` 先清掉连接再发信号，页面必须据此立刻重算状态。
            _fake_disconnect()
            Net.disconnected.emit()
            await get_tree().process_frame
            _expect(online_confirm != null and online_confirm.disabled,
                "断线后确认按钮必须立刻禁用")
            _expect(online_status != null and str(online_status.text).contains(
                TranslationServer.translate("ui.room.not_connected")
            ),
                "断线后状态行应立刻提示未连接，实际：%s" % (str(online_status.text) if online_status != null else ""))
        online.queue_free()
    _fake_disconnect()

    # 已确认：候选卡锁定且不再响应点击，确认按钮禁用并标「已确认」。
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "ready": true, "isHost": true,
         "heroCardId": "gauss", "heroCandidates": ["gauss", "mozi", "luban", "shenkuo"]},
        {"playerId": "p1", "displayName": "玩家B", "ready": false, "isHost": false,
         "heroCardId": null, "heroCandidates": null},
    ]
    var locked_instance := await _check_scene("res://scenes/room.tscn", false)
    if locked_instance != null:
        var locked_confirm = locked_instance.find_child("ConfirmButton", true, false)
        _expect(locked_confirm != null and locked_confirm.disabled, "已确认后确认按钮必须禁用")
        _expect(locked_confirm != null and str(locked_confirm.text) == TranslationServer.translate("ui.room.confirmed"),
            "已确认后按钮文字应为「已确认」，实际：%s" % (str(locked_confirm.text) if locked_confirm != null else ""))
        var locked_cards = locked_instance.find_child("HeroCandidates", true, false)
        if locked_cards != null and locked_cards.get_child_count() > 0:
            var all_locked := true
            var selected_frames := 0
            for card in locked_cards.get_children():
                if card.get("locked") != true:
                    all_locked = false
                var frame = card.find_child("SelectedFrame", true, false)
                if frame != null and frame.is_visible_in_tree():
                    selected_frames += 1
            _expect(all_locked, "已确认后全部候选卡应锁定")
            _expect(selected_frames == 1, "已确认后应保留一张选中框，实际：%d" % selected_frames)

            # 已确认后点击其他卡不改变选中，悬停仍可看详情。
            var locked_first: Control = locked_cards.get_child(0)
            var locked_second: Control = locked_cards.get_child(1) if locked_cards.get_child_count() > 1 else locked_first
            await _push_click(locked_instance, locked_second.get_global_rect().get_center())
            var first_frame = locked_first.find_child("SelectedFrame", true, false)
            _expect(first_frame != null and first_frame.is_visible_in_tree(), "已确认后点击其他卡不应改变选中")
            await _hover(locked_instance, locked_second, locked_second.size * 0.5)
            var locked_detail = locked_instance.find_child("RoomHeroDetail", true, false)
            _expect(locked_detail != null and locked_detail.visible, "已确认后悬停仍应显示详情")
        locked_instance.queue_free()

    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.room_players = []


## 递归检查控件树里是否有会拦截鼠标（STOP）的节点。
func _blocks_mouse(node: Node) -> bool:
    if node is Control and (node as Control).mouse_filter == Control.MOUSE_FILTER_STOP:
        return true
    for child in node.get_children():
        if _blocks_mouse(child):
            return true
    return false


func _check_game() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.view = FixtureScript.view()
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    for index in [0, 1, 3, 5]:
        var name := "ShopSlot%d" % index
        _expect(instance.find_child(name, true, false) != null, "对局场景缺少 %s" % name)
    _expect(
        instance.find_child("ShopSlot2", true, false) == null,
        "空商店槽不应渲染占位",
    )
    _expect(
        instance.find_child("ShopSlot4", true, false) == null,
        "空商店槽不应渲染占位",
    )
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
        for expected in [
            TranslationServer.translate("enum.stat_layer.initial"),
            "3 / 4",
            TranslationServer.translate("enum.stat_layer.permanent"),
            "+1 / +2",
            TranslationServer.translate("enum.stat_layer.turn"),
            "+0 / -2",
            TranslationServer.translate("enum.stat_layer.aura"),
            "+0 / +1",
            TranslationServer.translate("enum.stat_layer.damage"),
            "0 / -3",
            TranslationServer.translate("ui.game.detail.current"),
            "4 / 2",
        ]:
            _expect(text.contains(expected), "详情面板应包含「%s」：%s" % [expected, text])
        _expect(not text.contains("-0"), "非负构成项应带 +：%s" % text)
        _expect(card.find_child("Actions", true, false) == null, "棋盘随从卡不应再显示出售/移位按钮")
        _expect(detail.mouse_filter == Control.MOUSE_FILTER_IGNORE, "详情面板不应拦截鼠标")
        _expect(
            not detail.get_global_rect().intersects(card.get_global_rect()),
            "棋盘详情面板应挂在卡片上下方且不覆盖卡片"
        )
        await _hover(instance, card, Vector2(card.size.x / 2, card.size.y + 24))
        _expect(not detail.is_visible_in_tree(), "移出棋盘随从矩形应隐藏详情面板")
    # 商店卡：原始数值与类型写在卡面，悬停详情挂在卡片左右且不覆盖卡片。
    var shop_card = instance.find_child("ShopSlot0", true, false)
    if detail != null and shop_card != null:
        var shop_text := _node_text(shop_card)
        _expect(
            shop_text.contains(TranslationServer.translate("ui.game.detail.cost")) == false,
            "卡面不应挤入详情字段：%s" % shop_text,
        )
        await _hover(instance, shop_card, shop_card.size / 2)
        _expect(detail.is_visible_in_tree(), "悬停商店卡应显示详情面板")
        _expect(
            not detail.get_global_rect().intersects(shop_card.get_global_rect()),
            "商店详情面板应挂在卡片左右且不覆盖卡片"
        )
        var detail_text := _node_text(detail)
        _expect(
            detail_text.contains(TranslationServer.translate("ui.game.detail.cost"))
            and detail_text.contains(TranslationServer.translate("ui.game.detail.initial_stats")),
            "商店详情应显示费用与原始数值：%s" % detail_text
        )
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        await _push_click(instance, shop_card.get_global_rect().get_center())
        _expect(captured.is_empty(), "点击商店卡不应再发购买意图，实际 %s" % [captured])
        await _push_motion(instance, Vector2(-40, -40))
        _expect(not detail.is_visible_in_tree(), "移出商店卡矩形应隐藏详情面板")
    # 生命、时代与金币随英雄行展示：英雄卡承载名字与数值，英雄侧列显示金币。
    var hero_card = instance.find_child("HeroCard", true, false)
    var hero_text := "缺失" if hero_card == null else _node_text(hero_card)
    for text in ["35/40", TranslationServer.translate("enum.era.primitive")]:
        _expect(
            hero_card != null and hero_text.contains(text),
            "英雄行应显示「%s」，实际：%s" % [text, hero_text],
        )
    var gold_tile = instance.find_child("GoldTile", true, false)
    _expect(
        gold_tile != null and str(gold_tile.text).contains("6"),
        "英雄行应显示金币 6，实际：%s" % ("缺失" if gold_tile == null else gold_tile.text),
    )
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
    _clear_combat()
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
    var hand = instance.find_child("HandStrip", true, false)
    _expect(shop != null and recruit.is_ancestor_of(shop), "商店应位于招募中区")
    _expect(board_panel != null and board_panel.get_parent() == recruit, "我方棋盘应位于招募中区")
    if shop != null:
        _expect(not _node_text(shop).contains("商店"), "商店面板不再显示标题")
    if board_panel != null:
        _expect(not _node_text(board_panel).contains("我方棋盘"), "棋盘面板不再显示标题")
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
    _expect(board != null and board.get_child_count() == 4, "棋盘只应渲染实际随从，实际 %s"
        % ("null" if board == null else str(board.get_child_count())))
    _expect(instance.find_child("EmptySlot", true, false) == null, "棋盘不再渲染空位")
    var expected_card := Vector2(112, 146)
    for node_name in ["ShopSlot0", "ShopSlot3", "BoardMinion0", "HandCard0"]:
        var card_node: Control = instance.find_child(node_name, true, false)
        _expect(card_node != null, "布局中缺少 %s" % node_name)
        if card_node != null:
            _expect(
                card_node.custom_minimum_size == expected_card,
                "%s 应为 112×146，实际 %s" % [node_name, card_node.custom_minimum_size]
            )
    var board_scroll = instance.find_child("BoardScroll", true, false)
    _expect(
        board_scroll != null and is_equal_approx(board_scroll.custom_minimum_size.y, 146.0),
        "棋盘滚动高度应为 card-height"
    )
    # 商店与棋盘卡不显示介绍；手牌保留介绍，且仍在名称上方。
    for no_desc_node in ["ShopSlot0", "BoardMinion0"]:
        var plain_card: Control = instance.find_child(str(no_desc_node), true, false)
        _expect(plain_card != null, "布局中缺少 %s" % str(no_desc_node))
        if plain_card == null:
            continue
        _expect(
            plain_card.find_child("CardDesc", true, false) == null,
            "%s 不应显示卡面介绍" % str(no_desc_node),
        )
        var plain_info: Control = plain_card.find_child("CardInfo", true, false)
        _expect(
            plain_info != null and absf(plain_info.size.y - 56.0) <= 1.0,
            "%s 的信息区应为紧凑高度 56，实际 %s"
            % [str(no_desc_node), plain_info.size.y if plain_info != null else -1.0],
        )
    var intro_card: Control = instance.find_child("HandCard0", true, false)
    _expect(intro_card != null, "布局中缺少 HandCard0")
    if intro_card != null:
        var intro_desc: Label = intro_card.find_child("CardDesc", true, false)
        var intro_name: Label = intro_card.find_child("CardName", true, false)
        _expect(
            intro_desc != null and intro_desc.is_visible_in_tree(),
            "HandCard0 应显示卡面介绍",
        )
        _expect(
            intro_desc != null and str(intro_desc.text) == CardDB.display_text("archimedes"),
            "HandCard0 的介绍应为卡表技能",
        )
        if intro_desc != null and intro_name != null:
            _expect(
                intro_desc.get_global_rect().end.y <= intro_name.get_global_rect().position.y + 1.0,
                "HandCard0 的介绍应在名称上方",
            )
    var layout_detail = instance.find_child("MinionDetailPanel", true, false)
    var board_minion = instance.find_child("BoardMinion0", true, false)
    if layout_detail != null and board_minion != null:
        await _hover(instance, board_minion, board_minion.size / 2)
        _expect(
            _node_text(layout_detail).contains(CardDB.display_text("bruce-lee")),
            "棋盘随从详情应显示介绍",
        )
        await _push_motion(instance, Vector2(-40, -40))
    var hand_scroll = instance.find_child("HandScroll", true, false)
    _expect(
        hand != null and is_equal_approx(hand.custom_minimum_size.y, 146.0),
        "手牌行高度应为 card-height"
    )
    var hand_card_zero = instance.find_child("HandCard0", true, false)
    var hand_slot_one = instance.find_child("HandSlot1", true, false)
    _expect(
        hand_card_zero != null and hand_slot_one != null
        and hand_card_zero.get_global_rect().get_center().x < hand_slot_one.get_global_rect().position.x,
        "首张手牌中心应落在可见区内，真实拖拽起点不被下一槽遮挡",
    )
    if board != null:
        _expect(board.get_global_rect().size.y <= 147.0, "棋盘 7 槽应保持单行")
    var shop_zero = instance.find_child("ShopSlot0", true, false)
    var shop_text := _node_text(shop_zero) if shop_zero != null else ""
    _expect(
        shop_text.contains(TranslationServer.translate("enum.playstyle.reptile")),
        "商店卡应显示玩法：%s" % shop_text,
    )
    _expect(shop_text.contains("9"), "商店卡应显示原始生命值：%s" % shop_text)
    _expect(instance.find_child("HeroCard", true, false) != null, "英雄行应显示我的英雄卡")
    instance.queue_free()
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1


## 手牌重叠：1–10 张按步进 80 单行，相邻卡重叠 32，悬停放大不被滚动行裁切。
func _check_hand_overlap_layout() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    var hand: Array = []
    var affordances: Array = []
    for index in range(10):
        hand.append({
            "index": index,
            "cardId": "dia",
            "playable": false,
            "requiresTarget": false,
            "disabledReason": "NO_SPACE",
        })
        affordances.append({"index": index, "playable": false, "disabledReason": "NO_SPACE"})
    view["self"]["hand"] = hand
    view["self"]["affordances"]["handCards"] = affordances
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var hand_row = instance.find_child("HandRow", true, false)
    var hand_scroll = instance.find_child("HandScroll", true, false)
    _expect(hand_row != null, "对局页应存在手牌行")
    _expect(
        hand_row != null and hand_row.get_theme_constant("h_separation") == 0,
        "手牌行不应在槽之间再留间距",
    )
    if hand_row != null:
        _expect(hand_row.get_child_count() == 10, "十张手牌应渲染十个槽，实际 %d" % hand_row.get_child_count())
        var slots: Array = []
        for index in range(10):
            var slot: Control = hand_row.get_child(index) as Control if index < hand_row.get_child_count() else null
            _expect(slot != null and str(slot.name) == "HandSlot%d" % index, "第 %d 个槽应为 HandSlot%d" % [index, index])
            if slot == null:
                continue
            slots.append(slot)
            var expected_slot := (
                Vector2(UITheme.metric("card-width"), 146) if index == 9 else Vector2(80, 146)
            )
            _expect(
                slot.custom_minimum_size == expected_slot,
                "HandSlot%d 应为 %s，实际 %s" % [index, expected_slot, slot.custom_minimum_size],
            )
        for index in slots.size():
            var slot: Control = slots[index]
            var card: Control = slot.get_child(0) as Control if slot.get_child_count() > 0 else null
            _expect(card != null and str(card.name) == "HandCard%d" % index, "槽 %d 应承载 HandCard%d" % [index, index])
            if card == null:
                continue
            _expect(card.size == Vector2(112, 146), "HandCard%d 应为 112×146，实际 %s" % [index, card.size])
            _expect(card.position == Vector2.ZERO, "HandCard%d 应贴槽左上角" % index)
            if index > 0:
                var previous: Control = slots[index - 1]
                _expect(
                    is_equal_approx(slot.get_global_rect().position.x - previous.get_global_rect().position.x, 80.0),
                    "相邻槽应步进 80",
                )
                _expect(
                    is_equal_approx(slot.get_global_rect().position.y, previous.get_global_rect().position.y),
                    "手牌槽应保持同一行",
                )
        if slots.size() >= 2:
            var first_card: Control = slots[0].get_child(0) as Control
            var second_card: Control = slots[1].get_child(0) as Control
            _expect(
                first_card.get_global_rect().end.x > second_card.get_global_rect().position.x,
                "相邻手牌应重叠",
            )
            _expect(
                is_equal_approx(
                    first_card.get_global_rect().end.x - second_card.get_global_rect().position.x,
                    32.0,
                ),
                "相邻手牌应正好交叠 32",
            )
        _expect(hand_row.get_global_rect().size.y < 292.0, "十张手牌应保持单行")
        if slots.size() == 10 and hand_scroll != null:
            var last_card: Control = slots[9].get_child(0) as Control
            _expect(
                last_card.get_global_rect().end.x <= hand_scroll.get_global_rect().end.x + 0.5,
                "末张手牌不应超出滚动行右边界",
            )
    var hand_strip = instance.find_child("HandStrip", true, false)
    _expect(
        hand_strip != null and is_equal_approx(hand_strip.custom_minimum_size.y, 146.0),
        "手牌行高度应保持 card-height",
    )
    _expect(hand_scroll != null and not hand_scroll.clip_contents, "手牌滚动行不应裁切悬停放大的卡片")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 手牌随从牌显示服务端给出的真实身材与金色标识，效果牌显示破折号。
func _check_hand_shows_server_stats() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    var minion: Dictionary = FixtureScript.hand("archimedes", false, true, 7, 12)
    minion["index"] = 0
    var effect: Dictionary = FixtureScript.hand("aolinpike-jidian", false)
    effect["index"] = 1
    view["self"]["hand"] = [minion, effect]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
        {"index": 1, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var golden_card = instance.find_child("HandCard0", true, false)
    _expect(golden_card != null, "应渲染金色手牌")
    if golden_card != null:
        var face = golden_card.find_child("CardFace", true, false)
        var attack_label = face.find_child("CardAttack", true, false) if face != null else null
        var health_label = face.find_child("CardHealth", true, false) if face != null else null
        _expect(attack_label != null and attack_label.text == "7", "手牌金色应显示继承后的攻击力 7")
        _expect(health_label != null and health_label.text == "12", "手牌金色应显示继承后的生命值 12")
        var badge = face.find_child("CardBadge", true, false) if face != null else null
        _expect(badge != null and badge.text == "★", "手牌金色应带金色标识")
        _expect(bool(golden_card.payload.get("golden", false)), "手牌拖拽数据应带金色标记")
        _expect(str(golden_card.payload.get("instanceId", "")) == "2001", "手牌拖拽数据应带实例编号")
    var effect_card = instance.find_child("HandCard1", true, false)
    _expect(effect_card != null, "应渲染效果牌")
    if effect_card != null:
        var effect_face = effect_card.find_child("CardFace", true, false)
        var effect_attack = effect_face.find_child("CardAttack", true, false) if effect_face != null else null
        _expect(effect_attack != null and effect_attack.text == "—", "效果牌不显示身材")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 三连奖励牌：角标显示锁定星级，悬停文案写明发现星级。
func _check_hand_reward_card_badge() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["hand"] = [FixtureScript.hand("triple-reward", false, false, null, null, 4)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var card = instance.find_child("HandCard0", true, false)
    _expect(card != null, "应渲染奖励牌")
    if card != null:
        var face = card.find_child("CardFace", true, false)
        var badge = face.find_child("CardBadge", true, false) if face != null else null
        _expect(badge != null and badge.text == "4", "奖励牌角标应显示锁定星级 4")
        _expect(
            str(card.tooltip_text) == tr("ui.game.reward_card.text") % 4,
            "奖励牌悬停文案应写明发现 4 星随从",
        )
        _expect(not bool(card.payload.get("golden", true)), "奖励牌不是金色牌")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 手牌悬停：置顶并以底边中点为轴放大 1.12，越出滚动行仍可命中；移出复位。
func _check_hand_hover() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = FixtureScript.game_view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var card: Control = instance.find_child("HandCard0", true, false)
    _expect(card is HandCard, "布局中缺少 HandCard0")
    if card is HandCard:
        var slot: Control = card.get_parent() as Control
        var row = instance.find_child("HandRow", true, false)
        var top_left: Vector2 = card.get_global_rect().position
        var payload_before: Dictionary = (card as HandCard).payload.duplicate()
        var order_before: Array = []
        if row != null:
            for child in row.get_children():
                order_before.append(str(child.name))
        await _push_motion(instance, Vector2(-40, -40))
        await _hover(instance, card, card.size / 2)
        _expect(
            card.scale.is_equal_approx(Vector2(1.12, 1.12)),
            "悬停手牌应以 1.12 放大，实际 %s" % card.scale,
        )
        _expect(
            card.pivot_offset.is_equal_approx(Vector2(card.size.x / 2, card.size.y)),
            "悬停手牌应以底边中点为轴，实际 %s" % card.pivot_offset,
        )
        _expect(slot != null and slot.z_index == 1, "悬停手牌应置顶")
        # 指针移到原卡上缘之外、放大卡之内：滚动行关闭裁剪后仍应命中并保持悬停。
        await _push_motion(instance, top_left + Vector2(card.size.x / 2, -10))
        _expect(
            card.scale.is_equal_approx(Vector2(1.12, 1.12)),
            "放大越出滚动行后仍应保持悬停",
        )
        _expect((card as HandCard).payload == payload_before, "悬停不应改变拖拽数据")
        var order_after: Array = []
        if row != null:
            for child in row.get_children():
                order_after.append(str(child.name))
        _expect(order_after == order_before, "悬停不应改变手牌顺序")
        await _push_motion(instance, Vector2(-40, -40))
        _expect(card.scale.is_equal_approx(Vector2.ONE), "移出手牌后应恢复原尺寸")
        _expect(card.pivot_offset.is_equal_approx(Vector2.ZERO), "移出后应清空缩放轴")
        _expect(slot != null and slot.z_index == 0, "移出后应恢复层级")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 十张 dia 手牌的对局实例；`playable` 决定手牌是否允许拖拽。
func _ten_card_instance(playable: bool) -> Node:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    var hand: Array = []
    var affordances: Array = []
    for index in range(10):
        hand.append({
            "index": index,
            "cardId": "dia",
            "playable": playable,
            "requiresTarget": false,
            "disabledReason": null if playable else "NO_SPACE",
        })
        affordances.append({
            "index": index,
            "playable": playable,
            "disabledReason": null if playable else "NO_SPACE",
        })
    view["self"]["hand"] = hand
    view["self"]["affordances"]["handCards"] = affordances
    GameStore.view = view
    return await _check_scene("res://scenes/game.tscn", false)


## 悬停卡右侧交叠带仍归这张卡：Godot 按树序逆序拾取，z_index 不参与命中。
func _check_hand_hover_overlap_strip() -> void:
    var instance := await _ten_card_instance(false)
    if instance != null:
        var card0: Control = instance.find_child("HandCard0", true, false)
        var card1: Control = instance.find_child("HandCard1", true, false)
        _expect(
            card0 is HandCard and not (card0 as HandCard).drag_allowed,
            "禁用手牌在招募阶段应不可拖动",
        )
        if card0 is HandCard and card1 != null:
            var slot: Control = card0.get_parent() as Control
            var top_left: Vector2 = card0.get_global_rect().position
            var strip := top_left + Vector2(card0.size.x - 16.0, card0.size.y / 2.0)
            await _push_motion(instance, Vector2(-40, -40))
            await _hover(instance, card0, card0.size / 2)
            _expect(card0.scale.is_equal_approx(Vector2(1.12, 1.12)), "禁用手牌也应悬停放大")
            await _push_motion(instance, strip)
            _expect(
                card0.scale.is_equal_approx(Vector2(1.12, 1.12)),
                "指针进入本卡右侧交叠带后应保持放大",
            )
            _expect(card1.scale.is_equal_approx(Vector2.ONE), "交叠带不应把悬停让给下一张卡")
            _expect(slot != null and slot.z_index == 1, "交叠带内应保持置顶")
            var hovered := instance.get_viewport().gui_get_hovered_control()
            _expect(
                hovered == card0,
                "交叠带应命中的是本卡，实际 %s" % ("<null>" if hovered == null else str(hovered.name)),
            )
            await _push_motion(instance, Vector2(-40, -40))
        instance.queue_free()
        await get_tree().process_frame

    _fake_connect()
    var playing := await _ten_card_instance(true)
    if playing != null:
        var card0: Control = playing.find_child("HandCard0", true, false)
        _expect(
            card0 is HandCard and (card0 as HandCard).drag_allowed,
            "可出牌手牌应允许拖拽",
        )
        if card0 is HandCard and (card0 as HandCard).drag_allowed:
            var top_left: Vector2 = card0.get_global_rect().position
            var strip := top_left + Vector2(card0.size.x - 16.0, card0.size.y / 2.0)
            var viewport = playing.get_viewport()
            await _push_motion(playing, Vector2(-40, -40))
            await _hover(playing, card0, card0.size / 2)
            await _push_motion(playing, strip)
            await _push_button(playing, strip, true)
            await _push_motion_held(playing, strip + Vector2(24, 0), Vector2(24, 0))
            _expect(viewport.gui_is_dragging(), "交叠带按下并越过阈值应开始拖拽")
            if viewport.gui_is_dragging():
                var data: Variant = viewport.gui_get_drag_data()
                _expect(
                    data is Dictionary and int(data.get("handIndex", -1)) == 0,
                    "交叠带拖拽应拾起本卡",
                )
                viewport.gui_cancel_drag()
        playing.queue_free()
        await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 战斗阶段手牌只读，但悬停仍应完整展示卡片，且不改变拖拽数据。
func _check_hand_hover_in_combat() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    var card: Control = instance.find_child("HandCard0", true, false)
    _expect(
        card is HandCard and not (card as HandCard).drag_allowed,
        "战斗阶段手牌应不可拖动",
    )
    if card is HandCard:
        var slot: Control = card.get_parent() as Control
        var hand_index := int((card as HandCard).payload.get("handIndex", -1))
        await _push_motion(instance, Vector2(-40, -40))
        await _hover(instance, card, card.size / 2)
        _expect(card.scale.is_equal_approx(Vector2(1.12, 1.12)), "战斗阶段手牌仍应悬停放大")
        _expect(slot != null and slot.z_index == 1, "战斗阶段悬停手牌仍应置顶")
        _expect(
            int((card as HandCard).payload.get("handIndex", -1)) == hand_index,
            "战斗阶段悬停不应改变手牌下标",
        )
        await _push_motion(instance, Vector2(-40, -40))
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""


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
    _set_combat(FixtureScript.combat_result())
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var recruit = instance.find_child("RecruitPanel", true, false)
    var combat = instance.find_child("CombatPanel", true, false)
    _expect(combat != null and combat.visible, "战斗阶段应显示战斗视图")
    _expect(recruit != null and not recruit.visible, "战斗阶段不应显示招募视图")
    var hand_panel = instance.find_child("HandStrip", true, false)
    _expect(hand_panel != null and hand_panel.visible, "演示期间手牌仍应可见")
    var banner = instance.find_child("CombatBanner", true, false)
    _expect(
        banner != null
            and not banner.text.contains(TranslationServer.translate("ui.game.combat.win")),
        "动作播完前不应显示结果",
    )
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    _expect(opponent_row != null and opponent_row.get_child_count() == 2, "对手随从行应为 2 张")
    var own_row = instance.find_child("CombatOwnRow", true, false)
    _expect(own_row != null and own_row.get_child_count() == 3, "我方随从行应为 3 张")
    for row in [opponent_row, own_row]:
        if row != null and row.get_child_count() > 0:
            _expect(
                (row.get_child(0) as Control).custom_minimum_size == Vector2(112, 146),
                "战斗只读卡应为 112×146",
            )
    var end_button = instance.find_child("RecruitEndButton", true, false)
    _expect(
        end_button == null or not end_button.is_visible_in_tree(),
        "战斗阶段不应显示结束招募按钮",
    )
    var power_button = instance.find_child("HeroPowerButton0", true, false)
    _expect(power_button != null and not power_button.visible, "演示期间不应显示英雄技能按钮")
    var passive_mark = instance.find_child("HeroPassive0", true, false)
    _expect(
        passive_mark != null and passive_mark.is_visible_in_tree(),
        "演示期间被动技能标记应保留",
    )
    _expect(instance.find_child("ShopPanel", true, false) != null, "商店节点仍在招募面板内")
    var hand_card = instance.find_child("HandCard0", true, false)
    _expect(hand_card != null and not hand_card.drag_allowed, "演示期间手牌不可拖动")
    await _check_combat_no_skip_entry(instance)
    await _check_combat_detail(instance)
    instance.queue_free()
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""


## 演示期间没有跳过或加速入口。首局引导层自带「跳过」，不属于战斗演示，排除在外。
func _check_combat_no_skip_entry(instance: Node) -> void:
    var guide := instance.find_child("FirstGameGuide", true, false)
    for child in instance.find_children("*", "Button", true, false):
        if guide != null and guide.is_ancestor_of(child):
            continue
        var text := str((child as Button).text)
        _expect(not text.contains("跳过"), "演示不应提供「跳过」：%s" % text)
        _expect(not text.contains("加速"), "演示不应提供「加速」：%s" % text)


## 落点门控：拖拽期间阶段改变、结束招募、视图过期、倒计时归零、断线或进入
## 目标待选时，落点不构造意图。
func _check_drop_gate() -> void:
    var gate: Callable = Callable(load("res://scenes/game.gd"), "drop_gate")
    _expect(gate.call(Protocol.PHASE_RECRUIT, false, false, true, false, false), "正常招募态应允许落点")
    _expect(not gate.call(Protocol.PHASE_COMBAT, false, false, true, false, false), "战斗阶段不应允许落点")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, false, false, true, false, true), "已结束招募不应允许落点")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, true, false, true, false, false), "视图过期不应允许落点")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, false, true, true, false, false), "倒计时归零不应允许落点")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, false, false, false, false, false), "未连接不应允许落点")
    _expect(not gate.call(Protocol.PHASE_RECRUIT, false, false, true, true, false), "目标待选不应允许落点")

    # 端到端：拖拽数据有效但连接已断，落点不产生意图。
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [FixtureScript.board_minion("mi_1", "baiyi", 0)]
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    _fake_connect()
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        _fake_disconnect()
        return
    var hand_card = instance.find_child("HandCard0", true, false)
    var board_row = instance.find_child("BoardRow", true, false)
    if hand_card != null and board_row != null:
        var payload: Dictionary = hand_card.payload.duplicate()
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        var local: Vector2 = Vector2(4, 4)
        board_row._drop_data(local, payload)
        await get_tree().process_frame
        _expect(captured.size() == 1, "已连接时落点应产生意图，实际 %d 条" % captured.size())
        captured.clear()
        _fake_disconnect()
        board_row._drop_data(local, payload)
        await get_tree().process_frame
        _expect(captured.is_empty(), "断线后落点不应产生意图，实际 %d 条" % captured.size())
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 阵亡卡立即离场，复生后回到原相对位置。
func _check_combat_reborn_reappears() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Combat"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view := FixtureScript.game_view()
    view["phase"] = "Combat"
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    _set_combat(FixtureScript.combat_result_with_reborn())
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        _clear_combat()
        return
    var own_row = instance.find_child("CombatOwnRow", true, false)
    _expect(own_row != null and own_row.get_child_count() == 2, "阵亡前我方行应为 2 张")
    if own_row != null:
        # 0.6 秒开战画面结束后才开始计数。
        instance._process(0.7)
        instance._process(0.32)
        _expect(own_row.get_child_count() == 1, "阵亡后我方行应只剩 1 张，实际 %d" % own_row.get_child_count())
        if own_row.get_child_count() == 1:
            _expect(str(own_row.get_child(0).get_meta("minion_id", "")) == "mi_1", "留下的是 mi_1")
        instance._process(0.32)
        _expect(own_row.get_child_count() == 2, "复生后我方行应回到 2 张，实际 %d" % own_row.get_child_count())
        if own_row.get_child_count() == 2:
            _expect(str(own_row.get_child(0).get_meta("minion_id", "")) == "mi_0", "复生随从回到原位置 0")
            _expect(_stats_text(own_row.get_child(0)) == "7 / 6", "复生后数值取服务端快照")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 召唤按存活序列下标插入：阵亡锚点不参与计数。
func _check_summon_inserts_at_alive_rank() -> void:
    var playback := CombatPlayback.new()
    playback.start(
        [
            {
                "kind": "death",
                "sourceSide": "own",
                "target": {"minionId": "mi_0", "attack": 1, "health": 0, "alive": false},
            },
            {
                "kind": "summon",
                "summon": {
                    "side": "own",
                    "position": 1,
                    "minion": {"minionId": "mi_9", "cardId": "seedling", "attack": 1, "health": 1},
                },
            },
        ],
        [
            {"minionId": "mi_0", "cardId": "baiyi", "attack": 1, "health": 1, "alive": true},
            {"minionId": "mi_1", "cardId": "airplane", "attack": 2, "health": 2, "alive": true},
            {"minionId": "mi_2", "cardId": "bisheng", "attack": 3, "health": 3, "alive": true},
        ],
        []
    )
    playback.advance(320.0)
    playback.advance(320.0)
    var ids: Array = []
    for minion in playback.own:
        ids.append(str(minion.get("minionId", "")))
    # 存活序列此时是 mi_1、mi_2；rank 1 应插在 mi_2 之前，即向量下标 2
    # （mi_0 的阵亡条目仍占向量下标 0）。
    _expect(ids == ["mi_0", "mi_1", "mi_9", "mi_2"], "召唤应按存活下标插入，实际顺序 %s" % [ids])


## 潜行随从在棋盘卡上显示状态标识；未潜行的随从没有该标识。
func _check_board_stealth_badge() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    var stealth_minion: Dictionary = FixtureScript.board_minion("mi_1", "baiyi", 0)
    stealth_minion["stealth"] = true
    view["self"]["board"] = [
        stealth_minion,
        FixtureScript.board_minion("mi_2", "airplane", 1),
    ]
    view["self"]["hand"] = []
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var stealth_card = instance.find_child("BoardMinion0", true, false)
    var plain_card = instance.find_child("BoardMinion1", true, false)
    var badge = stealth_card.find_child("StealthBadge", true, false) if stealth_card != null else null
    _expect(badge != null, "潜行随从应显示潜行标识")
    if badge != null:
        _expect(badge.text == "潜", "潜行标识文案应为「潜」，实为 %s" % badge.text)
    _expect(
        plain_card != null and plain_card.find_child("StealthBadge", true, false) == null,
        "未潜行随从不应显示潜行标识"
    )
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 回合号不匹配时（`CombatResult` 属于上一回合），不展示旧战报也不播演示。
func _check_stale_combat_not_shown() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Combat"
    GameStore.turn = 4
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view := FixtureScript.game_view()
    view["phase"] = "Combat"
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    GameStore.last_combat = FixtureScript.combat_result()
    GameStore.last_combat_turn = 3
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        _clear_combat()
        return
    var own_row = instance.find_child("CombatOwnRow", true, false)
    var banner = instance.find_child("CombatBanner", true, false)
    _expect(own_row != null and own_row.get_child_count() == 0, "上一回合战报不应渲染随从行")
    _expect(
        banner != null
            and not banner.text.contains(TranslationServer.translate("ui.game.combat.win")),
        "上一回合战报不应显示结果",
    )
    # 事件流已删除（ui_smoke_side 断言 LogText 不存在），旧战报不再有日志行可查。
    instance.queue_free()
    await get_tree().process_frame

    # 同一份战报标成本回合后恢复展示。
    GameStore.last_combat_turn = 4
    var fresh := await _check_scene("res://scenes/game.tscn", false)
    if fresh != null:
        var fresh_row = fresh.find_child("CombatOwnRow", true, false)
        _expect(fresh_row != null and fresh_row.get_child_count() == 3, "本回合战报应渲染 3 张随从")
        fresh.queue_free()
        await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 战斗中重连：Snapshot 先到、CombatResult 后到，先显示的等待提示必须随战报清掉。
func _check_combat_placeholder_cleared() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Combat"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view := FixtureScript.game_view()
    view["phase"] = "Combat"
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    _clear_combat()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    _expect(_placeholder_count(opponent_row) == 1, "战报未到时对手行应显示 1 条等待提示")

    # 战报到达（生产路径由 `CombatResult` 消息写入）。
    _set_combat(FixtureScript.combat_result())
    instance._refresh()
    _expect(_placeholder_count(opponent_row) == 0, "战报到达后等待提示应消失，实际仍有 %d 条"
        % _placeholder_count(opponent_row))
    _expect(opponent_row != null and opponent_row.get_child_count() == 2,
        "对手行应只剩 2 张战报随从，实际 %d 个节点"
        % (0 if opponent_row == null else opponent_row.get_child_count()))
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 棋盘容量取视图：只渲染实际随从，满场判定用 `boardSlots`。
func _check_board_capacity_from_view() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["boardSlots"] = 5
    view["self"]["board"] = [
        FixtureScript.board_minion("mi_1", "baiyi", 0),
        FixtureScript.board_minion("mi_2", "airplane", 1),
    ]
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var board_row = instance.find_child("BoardRow", true, false)
    _expect(board_row != null and board_row.get_child_count() == 2,
        "容量为 5 时棋盘只应渲染 2 个随从，实际 %s" % ("null" if board_row == null else str(board_row.get_child_count())))
    _expect(instance.find_child("BoardHint", true, false) == null, "棋盘不再显示「随从 x/x」提示")
    _expect(board_row != null and not board_row.board_full, "2/5 不应判定为满场")

    # 空棋盘不渲染占位，但整行拖放区必须保留，否则随从无法首次上场。
    var hand_card = instance.find_child("HandCard0", true, false)
    view["self"]["board"] = []
    GameStore.view = view
    GameStore.view_changed.emit()
    await get_tree().process_frame
    _expect(board_row != null and board_row.get_child_count() == 0, "空棋盘不应渲染任何占位节点")
    if board_row != null and hand_card != null:
        _expect(
            is_equal_approx(board_row.get_global_rect().size.y, 146.0),
            "空棋盘仍应保留整行拖放区高度，实际 %s" % board_row.get_global_rect().size.y,
        )
        _expect(
            board_row.hand_drop_accepted(
                board_row.get_global_rect().get_center(), hand_card.payload.duplicate()
            ),
            "空棋盘应接受随从牌",
        )

    # 同一视图改成满场：随从节点数与满场标记都跟着容量走。
    view["self"]["board"] = []
    for index in 5:
        view["self"]["board"].append(FixtureScript.board_minion("mi_%d" % index, "baiyi", index))
    view["self"]["affordances"]["boardMinions"] = []
    GameStore.view = view
    GameStore.view_changed.emit()
    await get_tree().process_frame
    _expect(board_row != null and board_row.board_full, "5/5 应判定为满场")
    _expect(board_row != null and board_row.get_child_count() == 5,
        "满场时应有 5 个随从节点，实际 %s" % ("null" if board_row == null else str(board_row.get_child_count())))
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


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
    # 0.6 秒开战画面结束后才开始计数。
    instance._process(0.7)
    instance._process(0.32)
    _expect(_stats_text(own_row.get_child(0)) == "7 / 6", "第一条攻击后我方首卡应为 7/6，实际 %s" % _stats_text(own_row.get_child(0)))
    _expect(_stats_text(opponent_row.get_child(0)) == "6 / 2", "第一条攻击后对手首卡应为 6/2，实际 %s" % _stats_text(opponent_row.get_child(0)))
    instance._process(0.32)
    _expect(opponent_row.get_child_count() == 1, "阵亡随从应立即离开行内，实际 %d 张" % opponent_row.get_child_count())
    instance._process(0.32)
    _expect(own_row.get_child_count() == 4, "召唤后我方行应为 4 张，实际 %d" % own_row.get_child_count())
    if own_row.get_child_count() == 4:
        _expect(str(own_row.get_child(1).get_meta("minion_id", "")) == "mi_9", "召唤随从应插到下标 1")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""


## 事件到音效：刷新、回合开始入场与买牌各自只播一次，被拒操作没有事件也不响。
func _check_sfx_events() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_RECRUIT
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = FixtureScript.view()
    _clear_combat()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return

    Sfx.clear_log()
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {
        "view": GameStore.view,
        "events": [{"kind": Protocol.EVENT_SHOP_ROLLED}],
    })
    _expect(Sfx.count_cue(Sfx.CUE_REROLL) == 1, "手动刷新应播一次刷新音效")
    _expect(Sfx.count_cue(Sfx.CUE_SHOP_ENTER) == 0, "手动刷新不应播商店入场音效")
    _expect(_sfx_played(Sfx.CUE_REROLL), "刷新音效应加载到默认公共资源")

    Sfx.clear_log()
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {
        "view": GameStore.view,
        "events": [
            {"kind": Protocol.EVENT_TURN_STARTED, "turn": 4},
            {"kind": Protocol.EVENT_SHOP_ROLLED},
        ],
    })
    _expect(Sfx.count_cue(Sfx.CUE_SHOP_ENTER) == 1, "回合开始应播一次商店入场音效")
    _expect(Sfx.count_cue(Sfx.CUE_REROLL) == 0, "回合开始不应叠刷新音效")

    Sfx.clear_log()
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {
        "view": GameStore.view,
        "events": [{"kind": Protocol.EVENT_MINION_BOUGHT, "cardId": "huo"}],
    })
    _expect(Sfx.count_cue(Sfx.CUE_CARD_BUY) == 1, "买牌应播一次购入音效")
    _expect(_sfx_card_id(Sfx.CUE_CARD_BUY) == "huo", "购入音效应带上卡牌 id")
    _expect(
        Sfx.card_path("huo", Sfx.CUE_CARD_BUY).ends_with("cards/default/buy.wav"),
        "没有逐卡音效时应回落到默认目录",
    )
    _expect(_sfx_played(Sfx.CUE_CARD_BUY), "购入音效应加载到默认公共资源")

    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1
    GameStore.phase = ""


## 首回合 `GameStarted` 不带事件：首次招募视图补播一次商店入场，之后不重复。
func _check_sfx_initial_shop_enter() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_RECRUIT
    GameStore.turn = 1
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["turn"] = 1
    GameStore.view = view
    _clear_combat()
    Sfx.clear_log()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    _expect(
        Sfx.count_cue(Sfx.CUE_SHOP_ENTER) == 1,
        "首回合首次进入对局应补播一次商店入场音效",
    )
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {"view": view, "events": []})
    _expect(
        Sfx.count_cue(Sfx.CUE_SHOP_ENTER) == 1,
        "同一局内视图刷新不应重播商店入场音效",
    )
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1
    GameStore.phase = ""


## 冻结/解冻按视图 `frozen` 变化各播一次；重抽不误判成冻结变化。
func _check_sfx_freeze() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_RECRUIT
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    GameStore.view = view
    _clear_combat()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return

    Sfx.clear_log()
    view["self"]["shop"][0]["frozen"] = true
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {"view": view, "events": []})
    _expect(Sfx.count_cue(Sfx.CUE_FREEZE) == 1, "冻结标记出现应播一次冻结音效")
    _expect(Sfx.count_cue(Sfx.CUE_UNFREEZE) == 0, "冻结时不应播解冻音效")

    Sfx.clear_log()
    view["self"]["shop"][3]["frozen"] = false
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {"view": view, "events": []})
    _expect(Sfx.count_cue(Sfx.CUE_UNFREEZE) == 1, "冻结标记消失应播一次解冻音效")
    _expect(Sfx.count_cue(Sfx.CUE_FREEZE) == 0, "解冻时不应播冻结音效")

    Sfx.clear_log()
    var rolled := FixtureScript.view()
    rolled["self"]["shop"][3]["frozen"] = false
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {
        "view": rolled,
        "events": [{"kind": Protocol.EVENT_SHOP_ROLLED}],
    })
    _expect(Sfx.count_cue(Sfx.CUE_FREEZE) == 0, "刷新清掉冻结不应误播冻结音效")
    _expect(Sfx.count_cue(Sfx.CUE_UNFREEZE) == 0, "刷新清掉冻结不应误播解冻音效")
    _expect(Sfx.count_cue(Sfx.CUE_REROLL) == 1, "刷新事件本身仍应播刷新音效")

    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1
    GameStore.phase = ""


## 战斗演示按动作节奏播放攻守双方卡牌的攻击与死亡音效；
## 开战与结果横幅各一条音效，且每场只播一次。
func _check_sfx_combat() -> void:
    Sfx.clear_log()
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    _expect(Sfx.count_cue(Sfx.CUE_COMBAT_START) == 1, "战斗装载应播一次开始音效")
    _expect(_sfx_played(Sfx.CUE_COMBAT_START), "战斗开始音效资源应可播放")
    Sfx.clear_log()
    # 0.6 秒开战画面结束后才开始计数。
    instance._process(0.7)
    instance._process(0.32)
    _expect(Sfx.count_cue(Sfx.CUE_CARD_ATTACK) == 1, "攻击动作应播一次攻击音效")
    _expect(
        _sfx_card_id(Sfx.CUE_CARD_ATTACK) == "bruce-lee",
        "攻击音效应挂到攻方卡牌",
    )
    instance._process(0.32)
    _expect(Sfx.count_cue(Sfx.CUE_CARD_DEATH) == 1, "阵亡动作应播一次死亡音效")
    _expect(
        _sfx_card_id(Sfx.CUE_CARD_DEATH) == "aristotle",
        "死亡音效应挂到阵亡卡牌",
    )
    # 总时长 = 3 × 320 + 2000 = 2960ms；推进到横幅出现。
    instance._process(2.4)
    _expect(Sfx.count_cue(Sfx.CUE_COMBAT_END) == 1, "结果横幅出现应播一次结束音效")
    _expect(_sfx_played(Sfx.CUE_COMBAT_END), "战斗结束音效资源应可播放")
    instance._process(0.5)
    _expect(Sfx.count_cue(Sfx.CUE_COMBAT_END) == 1, "结束音效不应重复")
    _expect(Sfx.count_cue(Sfx.CUE_COMBAT_START) == 0, "战斗推进不应重复开始音效")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""


func _sfx_played(cue: String) -> bool:
    for entry in Sfx.cue_log:
        if str(entry.get("cue", "")) == cue:
            return bool(entry.get("played", false))
    return false


func _sfx_card_id(cue: String) -> String:
    for entry in Sfx.cue_log:
        if str(entry.get("cue", "")) == cue:
            return str(entry.get("cardId", ""))
    return ""


## 播完动作并经过 TAIL_MS 后才显示胜负横幅。
func _check_combat_banner_after_tail() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    var banner = instance.find_child("CombatBanner", true, false)
    var win_text := TranslationServer.translate("ui.game.combat.win")
    # 0.6 秒开战画面结束后才开始计数。
    instance._process(0.7)
    for _step in range(4):
        instance._process(0.32)
    _expect(
        banner != null and not banner.text.contains(win_text),
        "动作刚播完且未到停留结束时不应显示结果",
    )
    instance._process(2.0)
    _expect(
        banner != null and banner.text.contains(win_text),
        "停留结束后应显示结果横幅",
    )
    _expect(
        banner != null and str(banner.text).contains(TranslationServer.translate("ui.game.combat.win")),
        "横幅应显示胜负：%s" % ("" if banner == null else str(banner.text)),
    )
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""


## 演示中重连：按 `startedAt` 反推进度，已播动作不重播。
func _check_combat_resume_from_started_at() -> void:
    # 总时长 = 3 × 320 + 2000 = 2960ms；开战已过约 1 秒时，3 条动作都应落地
    # 并留在横幅停留里——续播点直接落到 960ms 之后，不从头重播。
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_COMBAT
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view := FixtureScript.game_view()
    view["phase"] = Protocol.PHASE_COMBAT
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    var combat := FixtureScript.combat_result()
    combat["startedAt"] = Time.get_datetime_string_from_unix_time(
        int(Time.get_unix_time_from_system()) - 1
    ) + "Z"
    _set_combat(combat)
    Sfx.clear_log()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        _clear_combat()
        return
    _expect(
        Sfx.count_cue(Sfx.CUE_COMBAT_START) == 0,
        "续播进入战斗中段时不应补播战斗开始音效",
    )
    _expect(
        Sfx.count_cue(Sfx.CUE_COMBAT_END) == 0,
        "续播尚未到结果时不应播战斗结束音效",
    )
    var own_row = instance.find_child("CombatOwnRow", true, false)
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    var banner = instance.find_child("CombatBanner", true, false)
    _expect(
        own_row != null and own_row.get_child_count() == 4,
        "续播应按已过时间落地召唤：实际 %d 张" % (0 if own_row == null else own_row.get_child_count())
    )
    _expect(
        opponent_row != null and opponent_row.get_child_count() == 1,
        "续播点前的阵亡动作应立即落地（阵亡卡离场）：实际 %d 张"
        % (0 if opponent_row == null else opponent_row.get_child_count())
    )
    _expect(
        banner != null
            and not banner.text.contains(TranslationServer.translate("ui.game.combat.win")),
        "停留结束前不应显示结果",
    )
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 拖拽出牌：随从牌落在两张卡之间，落点下标取指针左半边的那张卡。
func _check_hand_drag_emits_position() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [
        FixtureScript.board_minion("mi_1", "baiyi", 0),
        FixtureScript.board_minion("mi_2", "airplane", 1),
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
    if hand_card != null:
        _expect(hand_card.find_child("CardFace", true, false) != null, "手牌卡应使用统一卡面")
        _expect(hand_card.find_child("CardName", true, false) != null, "手牌卡面应显示卡名")
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
                _expect(int(intent.get("position", -1)) == 1, "落在第二张卡上应插到位置 1")
        # 落在第一张卡上 → 0；落在末尾空白 → 随从数。
        var first: Control = null
        var last: Control = null
        for child in board_row.get_children():
            if child is Control and child.has_meta("minion_id"):
                if first == null:
                    first = child
                last = child
        if first != null and last != null:
            var on_first: Vector2 = board_row.get_global_transform().affine_inverse() * (first.get_global_rect().position + Vector2(4, 4))
            var first_captured: Array = []
            var collector := func(intent: Dictionary) -> void: first_captured.append(intent)
            instance.intent_sent.connect(collector)
            board_row._drop_data(on_first, payload)
            await get_tree().process_frame
            instance.intent_sent.disconnect(collector)
            _expect(
                first_captured.size() == 1 and int(first_captured[0].get("position", -1)) == 0,
                "落在第一张卡上应为位置 0，实际 %s" % [first_captured]
            )
            var after_last: Vector2 = board_row.get_global_transform().affine_inverse() * (
                last.get_global_rect().end + Vector2(24, last.get_global_rect().size.y * 0.5)
            )
            var tail_captured: Array = []
            var tail_collector := func(intent: Dictionary) -> void: tail_captured.append(intent)
            instance.intent_sent.connect(tail_collector)
            board_row._drop_data(after_last, payload)
            await get_tree().process_frame
            instance.intent_sent.disconnect(tail_collector)
            _expect(
                tail_captured.size() == 1 and int(tail_captured[0].get("position", -1)) == 2,
                "落在末尾空白应为随从数 2，实际 %s" % [tail_captured]
            )
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 手牌拖拽光标规则：棋盘行空白仍可放置；棋盘外、已满与需要目标的空白落点为无效区域。
func _check_hand_drag_cursor_rule() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [
        FixtureScript.board_minion("mi_1", "baiyi", 0),
        FixtureScript.board_minion("mi_2", "airplane", 1),
    ]
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var board_row = instance.find_child("BoardRow", true, false)
    var hand_card = instance.find_child("HandCard0", true, false)
    if board_row != null and hand_card != null:
        var payload: Dictionary = hand_card.payload.duplicate()
        # 卡片不足满槽时居中对齐，行内右侧仍是拖放区。
        var blank_point := Vector2(
            board_row.get_global_rect().end.x - 56.0,
            board_row.get_global_rect().get_center().y,
        )
        _expect(board_row.hand_drop_accepted(blank_point, payload), "棋盘行空白仍应接受随从牌")
        board_row.board_full = true
        _expect(not board_row.hand_drop_accepted(blank_point, payload), "棋盘已满时空白应无效")
        board_row.board_full = false
        var target_payload := {"handIndex": 0, "cardKind": "effect", "requiresTarget": true}
        _expect(
            not board_row.hand_drop_accepted(blank_point, target_payload),
            "需要目标的效果牌落在空白应无效"
        )
        var outside := Vector2(board_row.get_global_rect().end.x + 200, board_row.get_global_rect().get_center().y)
        _expect(not board_row.hand_drop_accepted(outside, payload), "棋盘外应为无效落点")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1


## 真实鼠标事件走完「手牌拖到棋盘行空白」：引擎的命中判定必须把落点交给 BoardDrop。
func _check_hand_drag_drops_on_board_blank() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [FixtureScript.board_minion("mi_1", "baiyi", 0)]
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var board_row = instance.find_child("BoardRow", true, false)
    var hand_card = instance.find_child("HandCard0", true, false)
    _expect(hand_card != null and bool(hand_card.drag_allowed), "手牌应允许拖拽")
    _expect(board_row != null, "棋盘行应存在")
    if hand_card != null and board_row != null:
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        var start: Vector2 = hand_card.get_global_rect().get_center()
        var target := Vector2(
            board_row.get_global_rect().end.x - 56.0,
            board_row.get_global_rect().get_center().y,
        )
        var slot: Control = hand_card.get_parent() as Control
        var payload_before: Dictionary = hand_card.payload.duplicate()
        await _push_motion(instance, Vector2(-40, -40))
        await _push_motion(instance, start)
        _expect(
            hand_card.scale.is_equal_approx(Vector2(1.12, 1.12)),
            "按住前手牌应已悬停放大",
        )
        await _push_button(instance, start, true)
        await _push_motion_held(instance, start + Vector2(24, 0), Vector2(24, 0))
        await _push_motion_held(instance, target, target - start - Vector2(24, 0))
        var viewport = instance.get_viewport()
        _expect(viewport.gui_is_dragging(), "超过拖拽阈值后应进入拖拽状态")
        _expect(hand_card.scale.is_equal_approx(Vector2.ONE), "拖拽开始后手牌应复位尺寸")
        _expect(hand_card.pivot_offset.is_equal_approx(Vector2.ZERO), "拖拽开始后应清空缩放轴")
        _expect(slot != null and slot.z_index == 0, "拖拽开始后应恢复层级")
        _expect(hand_card.payload == payload_before, "拖拽开始不应改变拖拽数据")
        await _push_button(instance, target, false)
        await get_tree().process_frame
        _expect(captured.size() == 1, "手牌落在棋盘空白应产生一条出牌意图，实际 %d" % captured.size())
        if captured.size() == 1:
            var intent: Dictionary = captured[0]
            _expect(str(intent.get("kind", "")) == Protocol.INTENT_PLAY_CARD, "落点意图应为出牌")
            _expect(int(intent.get("handIndex", -1)) == 0, "出牌意图应带手牌下标 0")
            _expect(intent.has("position"), "随从牌落点应带 position")
        if viewport.gui_is_dragging():
            viewport.gui_cancel_drag()
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 手牌拖到已有随从卡上：引擎命中判定同样要放行，落点取该卡下标。
func _check_hand_drag_drops_on_minion_card() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [FixtureScript.board_minion("mi_1", "baiyi", 0)]
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var hand_card = instance.find_child("HandCard0", true, false)
    var minion_card = instance.find_child("BoardMinion0", true, false)
    _expect(hand_card != null and minion_card != null, "场景应渲染手牌与棋盘随从卡")
    if hand_card != null and minion_card != null:
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        await _real_drag(
            instance,
            hand_card.get_global_rect().get_center(),
            minion_card.get_global_rect().get_center()
        )
        _expect(captured.size() == 1, "手牌落在随从卡上应产生一条出牌意图，实际 %d" % captured.size())
        if captured.size() == 1:
            var intent: Dictionary = captured[0]
            _expect(str(intent.get("kind", "")) == Protocol.INTENT_PLAY_CARD, "落点意图应为出牌")
            _expect(int(intent.get("position", -1)) == 0, "落在首张卡上应插到下标 0，实际 %s" % [intent])
        var viewport = instance.get_viewport()
        if viewport != null and viewport.gui_is_dragging():
            viewport.gui_cancel_drag()
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 商店卡拖到手牌带购买：卡带空白与手牌卡上都可落，点击与无效落点不产生意图。
func _check_shop_drag_buys() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    GameStore.view = FixtureScript.view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        _fake_disconnect()
        return
    var shop_card = instance.find_child("ShopSlot0", true, false)
    var sold_out = instance.find_child("ShopSlot1", true, false)
    var hand_card = instance.find_child("HandCard0", true, false)
    var hand_row = instance.find_child("HandStrip", true, false)
    var board_panel = instance.find_child("BoardPanel", true, false)
    _expect(
        shop_card is ShopCard and sold_out is ShopCard and hand_row is GameHandRow,
        "场景应渲染商店卡、手牌卡与手牌行"
    )
    if shop_card is ShopCard and sold_out is ShopCard and hand_row is GameHandRow:
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        var band_point := _hand_band_point(hand_row)
        await _real_drag(instance, shop_card.get_global_rect().get_center(), band_point)
        _expect(captured.size() == 1, "商店卡拖到手牌带空白应产生一条意图，实际 %d" % captured.size())
        if captured.size() == 1:
            _expect(str(captured[0].get("kind", "")) == Protocol.INTENT_BUY_MINION, "落点意图应为购买")
            _expect(int(captured[0].get("slot", -1)) == 0, "购买意图应带槽位 0，实际 %s" % [captured])
        captured.clear()

        if hand_card is HandCard:
            await _real_drag(
                instance, shop_card.get_global_rect().get_center(), hand_card.get_global_rect().get_center()
            )
            _expect(captured.size() == 1, "商店卡落在手牌卡上也应购买，实际 %d" % captured.size())
            if captured.size() == 1:
                _expect(str(captured[0].get("kind", "")) == Protocol.INTENT_BUY_MINION, "手牌卡落点应为购买")
            captured.clear()

        # 不可购买的槽位不能起手；拖到棋盘不产生意图。
        _expect(not sold_out.drag_allowed, "不可购买槽位不应允许拖拽")
        _expect(sold_out._get_drag_data(Vector2.ZERO) == null, "不可购买槽位不应交出拖拽数据")
        _expect(shop_card.drag_allowed, "可购买槽位应允许拖拽")
        await _real_drag(
            instance, shop_card.get_global_rect().get_center(), board_panel.get_global_rect().get_center()
        )
        _expect(captured.is_empty(), "拖到棋盘不应购买，实际 %s" % [captured])

        # 起手预览与棋盘一致：隐藏原卡并在原位放同尺寸透明占位，取消后恢复。
        var shop_row = instance.find_child("ShopRow", true, false)
        _expect(shop_row != null, "商店应有槽位行")
        if shop_row != null:
            shop_card.force_drag(shop_card.payload.duplicate(), Control.new())
            await get_tree().process_frame
            _expect(not shop_card.visible, "商店卡拖起后原卡应隐藏")
            var shop_spacer = shop_row.find_child("ShopDragSpacer", true, false)
            _expect(
                shop_spacer != null
                    and (shop_spacer as Control).custom_minimum_size == shop_card.custom_minimum_size,
                "商店拖拽占位应与被拖卡同尺寸",
            )
            _expect(
                shop_row.get_child_count() == 5,
                "占位应补齐被隐藏的原卡位，实际 %d" % shop_row.get_child_count(),
            )
            var drag_viewport = instance.get_viewport()
            if drag_viewport != null and drag_viewport.has_method("gui_cancel_drag"):
                drag_viewport.gui_cancel_drag()
            await get_tree().process_frame
            _expect(shop_card.visible, "拖拽取消后应恢复原商店卡可见")
            _expect(
                shop_row.find_child("ShopDragSpacer", true, false) == null,
                "拖拽取消后应移除商店拖拽占位",
            )

        # 落点判定：购买带只覆盖贴底卡带，不可购买数据拒绝。
        var band = hand_row.find_child("HandDropBand", true, false)
        _expect(band is HandDrop, "手牌行应有购买落点带")
        if band is HandDrop:
            var band_rect: Rect2 = (band as Control).get_global_rect()
            _expect(
                is_equal_approx(band_rect.size.y, UITheme.stage_metric("card-height")),
                "购买带高度应为卡片高度，实际 %s" % band_rect.size.y
            )
            _expect(
                is_equal_approx(band_rect.end.y, hand_row.get_global_rect().end.y),
                "购买带应贴着手牌底部"
            )
            var payload: Dictionary = shop_card.payload.duplicate()
            _expect(band._can_drop_data(Vector2.ZERO, payload), "购买带应接受商店卡")
            var unbuyable: Dictionary = payload.duplicate()
            unbuyable["buyable"] = false
            _expect(not band._can_drop_data(Vector2.ZERO, unbuyable), "不可购买数据不应被接受")

            # 拖拽途中视图已重建：epoch 不符时不发意图。
            var stale: Dictionary = payload.duplicate()
            stale["epoch"] = -1
            band._drop_data(Vector2.ZERO, stale)
            _expect(captured.is_empty(), "过期 epoch 不应购买，实际 %s" % [captured])

        var viewport = instance.get_viewport()
        if viewport != null and viewport.gui_is_dragging():
            viewport.gui_cancel_drag()
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 三种卡的拖拽预览都保持起手点：指针落在按下时的卡片位置，不跳到预览左上角。
func _check_drag_preview_keeps_pointer() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    GameStore.view = FixtureScript.view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        _fake_disconnect()
        return
    var shop_card = instance.find_child("ShopSlot0", true, false)
    var board_card = instance.find_child("BoardMinion0", true, false)
    var hand_card = instance.find_child("HandCard0", true, false)
    _expect(
        shop_card is ShopCard and board_card is BoardCard and hand_card is HandCard,
        "场景应渲染商店、棋盘与手牌三种可拖拽卡片",
    )
    if shop_card is ShopCard and board_card is BoardCard and hand_card is HandCard:
        await _expect_preview_offset(instance, shop_card, Vector2(28, 36))
        await _expect_preview_offset(instance, board_card, Vector2(12, 48))
        # 手牌悬停以底边中点为轴放大；取底边中点内侧，复位缩放前后起手点只差不到 1px。
        var hand_grab := Vector2(hand_card.size.x * 0.5, hand_card.size.y - 1.0)
        await _expect_preview_offset(instance, hand_card, hand_grab)
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 真实鼠标按在起手点并越过拖拽阈值，断言预览包裹层把卡片反向偏移，随后取消。
func _expect_preview_offset(instance: Node, card: Control, grab: Vector2) -> void:
    _expect(card.drag_allowed, "%s 应允许拖拽" % card.name)
    var viewport: Viewport = instance.get_viewport()
    if viewport != null and viewport.gui_is_dragging():
        viewport.gui_cancel_drag()
        await get_tree().process_frame
    var start: Vector2 = card.get_global_rect().position + grab
    await _push_motion(instance, start)
    await _push_button(instance, start, true)
    await _push_motion_held(instance, start + Vector2(24, 0), Vector2(24, 0))
    _expect(viewport != null and viewport.gui_is_dragging(), "%s 越过阈值后应进入拖拽" % card.name)
    var holder := instance.find_child("DragPreview", true, false) as Control
    _expect(
        holder != null and holder.get_child_count() == 1,
        "%s 拖拽预览应包一层定位容器" % card.name,
    )
    if holder != null and holder.get_child_count() == 1:
        var preview := holder.get_child(0) as Control
        _expect(
            preview.position.distance_to(-grab) <= 1.0,
            "%s 拖拽预览应保持起手点，实际偏移 %s" % [card.name, preview.position],
        )
    if viewport != null and viewport.gui_is_dragging():
        viewport.gui_cancel_drag()
        await get_tree().process_frame


## 手牌带底部购买落点：左侧空白处，避开居中的手牌卡。
func _hand_band_point(hand_row: Control) -> Vector2:
    var rect := hand_row.get_global_rect()
    return Vector2(rect.position.x + 8.0, rect.end.y - 12.0)


## 棋盘卡拖到商店槽位按钮上松手：出售落点必须越过槽位按钮交给商店面板。
func _check_board_drag_sells_over_shop_slot() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [FixtureScript.board_minion("mi_1", "baiyi", 0)]
    view["self"]["affordances"]["boardMinions"] = [
        {"index": 0, "sellable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var card = instance.find_child("BoardMinion0", true, false)
    var shop_slot = instance.find_child("ShopSlot0", true, false)
    _expect(card is BoardCard and shop_slot != null, "场景应渲染棋盘随从卡与商店槽位")
    if card is BoardCard and shop_slot is Control:
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        # 用引擎的 force_drag 起手，再用真实鼠标事件移动与松手。
        card.force_drag(card.payload.duplicate(), Control.new())
        await get_tree().process_frame
        var target: Vector2 = shop_slot.get_global_rect().get_center()
        await _push_motion_held(instance, target, target - card.get_global_rect().get_center())
        await _push_button(instance, target, false)
        await get_tree().process_frame
        _expect(captured.size() == 1, "拖到商店槽位上松手应产生一条出售意图，实际 %d" % captured.size())
        if captured.size() == 1:
            var intent: Dictionary = captured[0]
            _expect(str(intent.get("kind", "")) == Protocol.INTENT_SELL_MINION, "落点意图应为出售")
            _expect(str(intent.get("minionId", "")) == "mi_1", "出售意图应带随从 id")
        var viewport = instance.get_viewport()
        if viewport != null and viewport.gui_is_dragging():
            viewport.gui_cancel_drag()
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 棋盘卡拖拽换位：拖起后隐藏原卡，拖到末尾松手发出「移除后插入下标」意图。
func _check_board_drag_reorders() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [
        FixtureScript.board_minion("mi_1", "baiyi", 0),
        FixtureScript.board_minion("mi_2", "airplane", 1),
        FixtureScript.board_minion("mi_3", "huo", 2),
    ]
    view["self"]["affordances"]["boardMinions"] = [
        {"index": 0, "sellable": true, "disabledReason": null},
        {"index": 1, "sellable": true, "disabledReason": null},
        {"index": 2, "sellable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var board_row = instance.find_child("BoardRow", true, false)
    var card = instance.find_child("BoardMinion1", true, false)
    _expect(card is BoardCard, "棋盘随从卡应为 BoardCard")
    if board_row != null and card is BoardCard:
        var payload: Dictionary = card.payload.duplicate()
        _expect(card.drag_allowed, "招募阶段且已连接时棋盘卡应允许拖拽")
        _expect(int(payload.get("epoch", -1)) == int(instance._view_epoch), "棋盘卡拖拽数据应带当前视图版本")
        _expect(str(payload.get("minionId", "")) == "mi_2", "拖拽数据应带随从 id")
        _expect(int(payload.get("fromIndex", -1)) == 1, "拖拽数据应带原下标")
        # 用引擎的 force_drag 走真实的拖拽开始通知，验证 BoardDrop 会进入预览态。
        card.force_drag(payload.duplicate(), Control.new())
        await get_tree().process_frame
        _expect(board_row._drag_id == "mi_2", "拖拽开始通知应触发棋盘预览")
        _expect(not card.visible, "拖起后原卡应隐藏，不留占位框")
        var drag_spacer = board_row.find_child("BoardDragSpacer", true, false)
        _expect(
            drag_spacer != null and (drag_spacer as Control).custom_minimum_size == Vector2(112, 146),
            "拖拽占位应与卡片同为 112×146",
        )
        # 占位下标要跳过隐藏原卡：gap 2（末尾）落在第 3 个非占位子节点之前。
        _expect(board_row._gap_child_index(2) == 3, "末尾插入下标应换算到隐藏卡位之后")
        var last = instance.find_child("BoardMinion2", true, false)
        if last != null:
            var local: Vector2 = board_row.get_global_transform().affine_inverse() * (
                last.get_global_rect().end + Vector2(8, last.size.y * 0.5)
            )
            _expect(board_row._can_drop_data(local, payload), "棋盘卡应能落在棋盘内")
            var captured: Array = []
            instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
            board_row._drop_data(local, payload)
            await get_tree().process_frame
            _expect(captured.size() == 1, "一次换位落点只应产生一条意图，实际 %d" % captured.size())
            if captured.size() == 1:
                var intent: Dictionary = captured[0]
                _expect(str(intent.get("kind", "")) == Protocol.INTENT_REARRANGE_BOARD, "落点意图应为换位")
                _expect(int(intent.get("fromIndex", -1)) == 1, "换位意图应带原下标 1")
                _expect(int(intent.get("toIndex", -1)) == 2, "拖到末尾应插到下标 2，实际 %s" % [intent])
        _expect(card.visible, "落点后应恢复原卡可见")
        var viewport = instance.get_viewport()
        if viewport != null and viewport.has_method("gui_cancel_drag"):
            viewport.gui_cancel_drag()
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 商店出售落点：只接受可出售的棋盘卡；手牌与不可卖随从拒绝。
func _check_board_sell_drop() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [FixtureScript.board_minion("mi_1", "baiyi", 0)]
    view["self"]["affordances"]["boardMinions"] = [
        {"index": 0, "sellable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var shop = instance.find_child("ShopPanel", true, false)
    var card = instance.find_child("BoardMinion0", true, false)
    _expect(shop is SellDrop, "商店面板应为出售落点")
    if shop != null and card is BoardCard:
        _expect(shop.enabled, "招募阶段且已连接时商店应接受出售拖拽")
        var payload: Dictionary = card.payload.duplicate()
        _expect(
            not shop._can_drop_data(Vector2.ZERO, {"handIndex": 0, "cardKind": "minion"}),
            "手牌不应能拖到商店出售"
        )
        var unsellable := payload.duplicate()
        unsellable["sellable"] = false
        _expect(not shop._can_drop_data(Vector2.ZERO, unsellable), "不可出售的随从不应被商店接受")
        _expect(shop._can_drop_data(Vector2.ZERO, payload), "可出售随从应被商店接受")
        var hint = instance.find_child("SellHint", true, false)
        _expect(
            hint != null and hint.visible
            and str(hint.text) == TranslationServer.translate("ui.game.recruit.sell_hint") % 1,
            "悬停商店应提示松手出售与返还金币",
        )
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        shop._drop_data(Vector2.ZERO, payload)
        await get_tree().process_frame
        _expect(captured.size() == 1, "出售落点应产生一条意图，实际 %d" % captured.size())
        if captured.size() == 1:
            var intent: Dictionary = captured[0]
            _expect(str(intent.get("kind", "")) == Protocol.INTENT_SELL_MINION, "落点意图应为出售")
            _expect(str(intent.get("minionId", "")) == "mi_1", "出售意图应带随从 id")
        _expect(hint != null and not hint.visible, "松手后出售提示应隐藏")
        # 金色随从返还 3 金；提示由拖拽数据里的 golden 决定。
        var golden: Dictionary = payload.duplicate()
        golden["golden"] = true
        _expect(shop._can_drop_data(Vector2.ZERO, golden), "金色随从应被商店接受")
        _expect(
            hint != null and str(hint.text) == TranslationServer.translate("ui.game.recruit.sell_hint") % 3,
            "金色随从应提示返还 3 金",
        )
        shop._clear_hover()
    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 拖拽期间不显示悬停详情：购买经过自己棋盘、出售经过商店卡都不显示，取消后悬停恢复。
func _check_detail_hidden_while_dragging() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    _fake_connect()
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [FixtureScript.board_minion("mi_1", "baiyi", 0)]
    view["self"]["affordances"]["boardMinions"] = [
        {"index": 0, "sellable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        _fake_disconnect()
        return
    var detail = instance.find_child("MinionDetailPanel", true, false)
    var layer = instance.find_child("MinionDetailLayer", true, false)
    var shop_card = instance.find_child("ShopSlot0", true, false)
    var board_card = instance.find_child("BoardMinion0", true, false)
    _expect(
        detail != null and layer != null and shop_card is ShopCard and board_card is BoardCard,
        "场景应渲染详情面板、商店卡与棋盘随从卡"
    )
    if detail == null or layer == null or not (shop_card is ShopCard) or not (board_card is BoardCard):
        instance.queue_free()
        await get_tree().process_frame
        _fake_disconnect()
        GameStore.view = {}
        GameStore.turn = 1
        return
    var viewport = instance.get_viewport()
    var shop_center: Vector2 = shop_card.get_global_rect().get_center()
    var board_center: Vector2 = board_card.get_global_rect().get_center()

    # 基线：普通悬停商店卡显示详情，移出隐藏。
    await _hover(instance, shop_card, shop_card.size / 2)
    _expect(detail.is_visible_in_tree(), "拖拽前悬停商店卡应显示详情")
    await _push_motion(instance, Vector2(-40, -40))
    _expect(not detail.is_visible_in_tree(), "移出商店卡后应隐藏详情")

    # 购买拖拽经过自己棋盘：不显示棋盘卡详情。
    await _push_motion(instance, shop_center)
    await _push_button(instance, shop_center, true)
    await _push_motion_held(instance, shop_center + Vector2(0, 24), Vector2(0, 24))
    await _push_motion_held(instance, board_center, board_center - shop_center - Vector2(0, 24))
    _expect(viewport.gui_is_dragging(), "商店卡越过拖拽阈值后应进入拖拽")
    _expect(not detail.is_visible_in_tree(), "购买拖拽经过棋盘时不应显示详情")
    _expect(layer._hover_card == null, "购买拖拽经过棋盘时详情锚点应为空")
    if viewport.gui_is_dragging():
        viewport.gui_cancel_drag()
    await get_tree().process_frame
    await _hover(instance, shop_card, shop_card.size / 2)
    _expect(detail.is_visible_in_tree(), "购买拖拽取消后悬停商店卡应恢复详情")
    await _push_motion(instance, Vector2(-40, -40))

    # 出售拖拽经过商店卡：不显示商店卡详情。
    await _push_motion(instance, board_center)
    await _push_button(instance, board_center, true)
    await _push_motion_held(instance, board_center + Vector2(0, -24), Vector2(0, -24))
    await _push_motion_held(instance, shop_center, shop_center - board_center - Vector2(0, -24))
    _expect(viewport.gui_is_dragging(), "棋盘卡越过拖拽阈值后应进入拖拽")
    _expect(not detail.is_visible_in_tree(), "出售拖拽经过商店卡时不应显示详情")
    _expect(layer._hover_card == null, "出售拖拽经过商店卡时详情锚点应为空")
    if viewport.gui_is_dragging():
        viewport.gui_cancel_drag()
    await get_tree().process_frame
    await _hover(instance, shop_card, shop_card.size / 2)
    _expect(detail.is_visible_in_tree(), "出售拖拽取消后悬停商店卡应恢复详情")
    await _push_motion(instance, Vector2(-40, -40))
    _expect(not detail.is_visible_in_tree(), "移出商店卡后应再次隐藏详情")

    instance.queue_free()
    await get_tree().process_frame
    _fake_disconnect()
    GameStore.view = {}
    GameStore.turn = 1


## 棋盘已满时棋盘区域一律不接收随从牌，落在随从卡上也直接取消。
func _check_drag_rejected_when_board_full() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    var board: Array = []
    for index in 7:
        board.append(FixtureScript.board_minion("mi_%d" % index, "baiyi", index))
    view["self"]["board"] = board
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var board_row = instance.find_child("BoardRow", true, false)
    var hand_card = instance.find_child("HandCard0", true, false)
    if board_row != null and hand_card != null:
        var payload: Dictionary = hand_card.payload.duplicate()
        var last: Control = null
        for child in board_row.get_children():
            if child is Control and child.has_meta("minion_id"):
                last = child
        _expect(last != null, "满棋盘应有 7 张随从卡")
        if last != null:
            var on_card: Vector2 = board_row.get_global_transform().affine_inverse() * (last.get_global_rect().position + Vector2(4, 4))
            _expect(not board_row._can_drop_data(on_card, payload), "棋盘满时随从牌不应落在随从卡上")
            var blank := Vector2(board_row.size.x + 200, board_row.size.y + 100)
            _expect(not board_row._can_drop_data(blank, payload), "棋盘满时随从牌不应接受空白落点")
            var captured: Array = []
            instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
            board_row._drop_data(on_card, payload)
            await get_tree().process_frame
            _expect(captured.is_empty(), "棋盘满时落点不应产生意图，实际 %d 条" % captured.size())
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
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
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
        "targets": [{"zone": "board", "id": "mi_1"}],
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


## 目标待选：不再弹候选浮层，改由瞄准层承载；其它招募控件全部禁用。
func _check_target_pending_aim() -> void:
    _fake_connect()
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = _target_pending_view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var overlay = instance.find_child("ChooserOverlay", true, false)
    _expect(overlay == null or not overlay.visible, "目标待选不应再弹候选浮层")
    var aim = instance.find_child("AimLayer", true, false)
    _expect(aim != null and aim.pending_active(), "目标待选应开启瞄准层")
    _expect(aim != null and aim._candidates.size() == 2, "瞄准层应收到 2 个候选")
    var source_card := _board_card(instance, "mi_1")
    _expect(aim != null and source_card != null, "待选瞄准夹具缺少来源随从")
    if aim != null and source_card != null:
        _expect(
            aim._source_rect == source_card.get_global_rect(),
            "战吼待选应从入场随从卡起画，实际 %s / 卡片 %s"
                % [aim._source_rect, source_card.get_global_rect()],
        )
    for name in ["RerollButton", "UpgradeButton", "FreezeButton", "RecruitEndButton", "HeroPowerButton0"]:
        var button = instance.find_child(name, true, false)
        _expect(button != null and button.disabled, "目标待选期间 %s 应禁用" % name)
    var hand_card = instance.find_child("HandCard0", true, false)
    _expect(hand_card != null and not hand_card.drag_allowed, "目标待选期间手牌不应允许拖拽")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1
    _fake_disconnect()


## 待选瞄准：松手落在候选上发 ChooseTarget；落在非候选处或按 Esc 发 AbandonTarget。
func _check_pending_aim_abandon() -> void:
    _fake_connect()
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = _target_pending_view()
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var aim = instance.find_child("AimLayer", true, false)
    var card := _board_card(instance, "mi_2")
    _expect(aim != null and card != null, "待选瞄准夹具缺少瞄准层或候选随从")
    if aim != null and card != null:
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        var point: Vector2 = card.get_global_rect().get_center()
        await _push_button(instance, point, true)
        await _push_button(instance, point, false)
        _expect(captured.size() == 1, "选中候选只应发出 1 条意图，实际 %d" % captured.size())
        if captured.size() == 1:
            var intent: Dictionary = captured[0]
            _expect(
                str(intent.get("kind", "")) == Protocol.INTENT_CHOOSE_TARGET,
                "应发出 ChooseTarget",
            )
            var target: Dictionary = intent.get("target", {})
            _expect(str(target.get("zone", "")) == "board", "应带上目标区域")
            _expect(str(target.get("id", "")) == "mi_2", "应带上被选随从 id")

        # 同一待选重新下发：瞄准层按服务端待选重新开启。
        captured.clear()
        GameStore.view_changed.emit()
        await get_tree().process_frame
        await _push_button(instance, Vector2(4, 4), true)
        await _push_button(instance, Vector2(4, 4), false)
        _expect(captured.size() == 1, "落在非候选处应发出 1 条意图，实际 %d" % captured.size())
        if captured.size() == 1:
            _expect(
                str(captured[0].get("kind", "")) == Protocol.INTENT_ABANDON_TARGET,
                "落在非候选处应发 AbandonTarget",
            )

        captured.clear()
        GameStore.view_changed.emit()
        await get_tree().process_frame
        await _push_key(instance, KEY_ESCAPE)
        _expect(captured.size() == 1, "Esc 应发出 1 条意图，实际 %d" % captured.size())
        if captured.size() == 1:
            _expect(
                str(captured[0].get("kind", "")) == Protocol.INTENT_ABANDON_TARGET,
                "Esc 应发 AbandonTarget",
            )
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1
    _fake_disconnect()


## 需要目标的英雄技能：按下按钮起瞄准，落在候选上发 UseHeroPower；
## 落在非候选处或按 Esc 取消，不发意图。
func _check_skill_aim() -> void:
    _fake_connect()
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = FixtureScript.view()
    GameStore.view["self"]["heroPowers"] = [
        {
            "skillId": "universal-manuscript",
            "usesLeft": 1,
            "requiresTarget": true,
            "targetCandidates": [{"zone": "board", "id": "mi_1"}],
            "canUse": true,
            "disabledReason": null,
        },
    ]
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    await _dismiss_guide(instance)
    var aim = instance.find_child("AimLayer", true, false)
    var power = instance.find_child("HeroPowerButton0", true, false)
    var card := _board_card(instance, "mi_1")
    var detail = instance.find_child("MinionDetailLayer", true, false)
    _expect(aim != null and power != null and card != null, "技能瞄准夹具缺少节点")
    if aim != null and power != null and card != null:
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        var button_point: Vector2 = power.get_global_rect().get_center()
        var card_point: Vector2 = card.get_global_rect().get_center()
        await _push_motion(instance, button_point)
        await _push_button(instance, button_point, true)
        _expect(aim.skill_active(), "按下需要目标的技能按钮应进入瞄准")
        var overlay = instance.find_child("ChooserOverlay", true, false)
        _expect(overlay == null or not overlay.visible, "技能瞄准不应再弹候选浮层")
        await _push_motion(instance, card_point)
        _expect(str(aim._hovered.get("id", "")) == "mi_1", "指针落在候选上应命中该候选")
        _expect(detail != null and not detail.is_visible_in_tree(), "瞄准期间不应弹悬停详情")
        await _push_button(instance, card_point, false)
        _expect(not aim.skill_active(), "松手后应结束瞄准")
        _expect(captured.size() == 1, "落在候选中应只发 1 条意图，实际 %d" % captured.size())
        if captured.size() == 1:
            var intent: Dictionary = captured[0]
            _expect(
                str(intent.get("kind", "")) == Protocol.INTENT_USE_HERO_POWER,
                "应发出 UseHeroPower",
            )
            _expect(
                str(intent.get("skillId", "")) == "universal-manuscript",
                "应带技能 id",
            )
            var target: Dictionary = intent.get("target", {})
            _expect(str(target.get("zone", "")) == "board", "应带上目标区域")
            _expect(str(target.get("id", "")) == "mi_1", "应带上命中目标")

        captured.clear()
        await _push_button(instance, button_point, true)
        await _push_motion(instance, Vector2(4, 4))
        await _push_button(instance, Vector2(4, 4), false)
        _expect(not aim.skill_active() and captured.is_empty(), "落在非候选处应取消且不发意图")

        await _push_button(instance, button_point, true)
        await _push_key(instance, KEY_ESCAPE)
        _expect(not aim.skill_active() and captured.is_empty(), "Esc 应取消技能瞄准且不发意图")
        # 收尾松开鼠标，避免把按住状态留给后续用例。
        await _push_button(instance, button_point, false)
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1
    _fake_disconnect()


## 需要目标的效果牌拖动：瞄准层画同一套视觉，落点仍由棋盘提交 PlayCard。
func _check_effect_drag_aim_visual() -> void:
    _fake_connect()
    GameStore.player_id = "p0"
    GameStore.phase = "Recruit"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [FixtureScript.board_minion("mi_1", "baiyi")]
    view["self"]["hand"] = [
        FixtureScript.hand("erfenfa", true, false, null, null, null, [{"zone": "board", "id": "mi_1"}]),
    ]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    GameStore.view = view
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    await _dismiss_guide(instance)
    var aim = instance.find_child("AimLayer", true, false)
    var card := instance.find_child("HandCard0", true, false)
    var target := _board_card(instance, "mi_1")
    _expect(aim != null and card != null and target != null, "效果牌瞄准夹具缺少节点")
    if aim != null and card != null and target != null:
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        var start: Vector2 = card.get_global_rect().get_center()
        var point: Vector2 = target.get_global_rect().get_center()
        await _push_motion(instance, start)
        await _push_button(instance, start, true)
        await _push_motion_held(instance, start + Vector2(24, 0), Vector2(24, 0))
        _expect(aim._mode == GameAimLayer.Mode.DRAG, "拖动需要目标的牌时应显示瞄准弧线")
        var viewport := instance.get_viewport()
        if viewport.gui_is_dragging():
            viewport.gui_cancel_drag()
        await get_tree().process_frame
        await _push_button(instance, start, false)
        _expect(aim._mode == GameAimLayer.Mode.NONE, "拖拽结束后收起瞄准视觉")
        # 无头视口不更新真实指针位置，落点判定直接按棋盘局部坐标调用。
        var board_row = instance.find_child("BoardRow", true, false)
        var local: Vector2 = board_row.get_global_transform().affine_inverse() * point
        var payload: Dictionary = card.payload.duplicate()
        _expect(board_row._can_drop_data(local, payload), "需要目标的牌落在候选随从上应被接受")
        board_row._drop_data(local, payload)
        await get_tree().process_frame
        _expect(captured.size() == 1, "效果牌落在候选上应发 1 条意图，实际 %d" % captured.size())
        if captured.size() == 1:
            var intent: Dictionary = captured[0]
            _expect(str(intent.get("kind", "")) == Protocol.INTENT_PLAY_CARD, "应发出 PlayCard")
            _expect(str(intent.get("target", {}).get("id", "")) == "mi_1", "应带命中目标")
        _expect(aim._mode == GameAimLayer.Mode.NONE, "拖拽结束后收起瞄准视觉")
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.turn = 1
    _fake_disconnect()


## 首局引导的底栏会盖住英雄技能按钮：真实输入用例先卸掉引导。
func _dismiss_guide(instance: Node) -> void:
    var guide := instance.find_child("FirstGameGuide", true, false)
    if guide != null:
        guide.queue_free()
        await get_tree().process_frame


## 棋盘上按 `minion_id` 找随从卡。
func _board_card(instance: Node, minion_id: String) -> Control:
    var board_row = instance.find_child("BoardRow", true, false)
    if board_row == null:
        return null
    for child in board_row.get_children():
        if child is Control and child.has_meta("minion_id"):
            if str(child.get_meta("minion_id")) == minion_id:
                return child
    return null


## 战斗阶段即使留有跨回合的发现待选，也不能把招募弹层盖在演示上。
func _check_overlay_hidden_in_combat() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = "Combat"
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view := FixtureScript.game_view()
    view["phase"] = "Combat"
    view["opponentPlayerId"] = "p2"
    view["self"]["pendingChoices"] = [{"kind": "discover", "candidates": ["huo", "lun", "che"]}]
    GameStore.view = view
    _set_combat(FixtureScript.combat_result())
    var instance := await _check_scene("res://scenes/game.tscn", false)
    if instance == null:
        return
    var overlay = instance.find_child("ChooserOverlay", true, false)
    _expect(overlay != null and not overlay.visible, "战斗阶段不应弹出招募待选弹层")
    var aim = instance.find_child("AimLayer", true, false)
    _expect(aim != null and not aim.aiming(), "战斗阶段不应进入瞄准")
    var captured: Array = []
    instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
    instance._on_choose(0)
    await get_tree().process_frame
    _expect(captured.is_empty(), "战斗阶段不应发出待选意图，实际 %d 条" % captured.size())
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""


## 进入新回合招募时必须丢弃上一场战报：没有本回合 `CombatResult` 的座位
## （未配对、已出局）否则会把旧战斗当成新战斗重播。
func _check_last_combat_cleared_on_next_recruit() -> void:
    GameStore.reset_room()
    var combat_view: Dictionary = FixtureScript.view()
    combat_view["phase"] = "Combat"
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {"view": combat_view, "events": []})
    _set_combat(FixtureScript.combat_result())
    _expect(not GameStore.last_combat.is_empty(), "战斗阶段应保留本场战报")

    var recruit_view: Dictionary = FixtureScript.view()
    recruit_view["turn"] = 4
    GameStore.handle(Protocol.MSG_STATE_UPDATE, {"view": recruit_view, "events": []})
    _expect(
        GameStore.last_combat.is_empty(),
        "进入下一回合招募后应清空上一场战报，实际 %s" % [GameStore.last_combat]
    )
    GameStore.reset_room()


## 双方实例编号相同时（镜像战），动作必须按服务端下发侧别落地，不能只看 id。
func _check_combat_playback_uses_sides() -> void:
    var playback := CombatPlayback.new()
    playback.start(
        [
            {
                "kind": "attack",
                "sourceSide": "own",
                "targetSide": "opponent",
                "source": {"minionId": "mi_0", "attack": 1, "health": 2, "alive": true},
                "target": {"minionId": "mi_0", "attack": 3, "health": 4, "alive": true},
            },
        ],
        [{"minionId": "mi_0", "cardId": "baiyi", "attack": 5, "health": 5, "alive": true}],
        [{"minionId": "mi_0", "cardId": "baiyi", "attack": 6, "health": 6, "alive": true}]
    )
    playback.advance(320.0)
    _expect(
        int(playback.own[0].get("attack", 0)) == 1 and int(playback.own[0].get("health", 0)) == 2,
        "己方随从应写入 source 状态：实际 %s" % [playback.own[0]]
    )
    _expect(
        int(playback.opponent[0].get("attack", 0)) == 3 and int(playback.opponent[0].get("health", 0)) == 4,
        "对手随从应写入 target 状态：实际 %s" % [playback.opponent[0]]
    )


## 玩法扩充的新动作（拟态/吞噬/引爆/剧毒/复苏/回手）必须把服务端快照写进
## 回放；零件回收只影响手牌，回放不报错即可。
func _check_combat_playback_new_actions() -> void:
    var playback := CombatPlayback.new()
    playback.start(
        [
            {
                "kind": "mimic",
                "sourceSide": "own",
                "targetSide": "opponent",
                "source": {"minionId": "mi_0", "attack": 7, "health": 7, "alive": true},
                "target": {"minionId": "mo_0", "attack": 1, "health": 1, "alive": true},
            },
            {
                "kind": "devour",
                "sourceSide": "own",
                "targetSide": "opponent",
                "source": {"minionId": "mi_1", "attack": 3, "health": 4, "alive": true},
                "target": {"minionId": "mo_1", "attack": 1, "health": 0, "alive": false},
            },
            {
                "kind": "detonate",
                "targetSide": "opponent",
                "damage": 2,
                "target": {"minionId": "mo_2", "attack": 1, "health": 0, "alive": false},
            },
            {
                "kind": "poison",
                "targetSide": "opponent",
                "target": {"minionId": "mo_0", "attack": 1, "health": 0, "alive": false},
            },
            {
                "kind": "revive",
                "sourceSide": "own",
                "target": {"minionId": "mi_2", "attack": 1, "health": 3, "alive": true},
            },
            {
                "kind": "recall",
                "sourceSide": "own",
                "target": {"minionId": "mi_2", "attack": 1, "health": 0, "alive": false},
            },
            {
                "kind": "salvage",
                "sourceSide": "own",
                "cardId": "nanobot",
            },
        ],
        [
            {"minionId": "mi_0", "cardId": "baiyi", "attack": 1, "health": 1, "alive": true},
            {"minionId": "mi_1", "cardId": "baiyi", "attack": 1, "health": 1, "alive": true},
            {"minionId": "mi_2", "cardId": "baiyi", "attack": 1, "health": 0, "alive": false},
        ],
        [
            {"minionId": "mo_0", "cardId": "baiyi", "attack": 1, "health": 1, "alive": true},
            {"minionId": "mo_1", "cardId": "baiyi", "attack": 1, "health": 1, "alive": true},
            {"minionId": "mo_2", "cardId": "baiyi", "attack": 1, "health": 1, "alive": true},
        ]
    )

    playback.advance(320.0)
    _expect(
        int(playback.own[0].get("attack", 0)) == 7 and int(playback.own[0].get("health", 0)) == 7,
        "拟态应写入来源快照：实际 %s" % [playback.own[0]]
    )
    _expect(
        int(playback.opponent[0].get("health", 0)) == 1,
        "拟态不应改动目标快照：实际 %s" % [playback.opponent[0]]
    )

    playback.advance(320.0)
    _expect(
        int(playback.own[1].get("attack", 0)) == 3 and int(playback.own[1].get("health", 0)) == 4,
        "吞噬应写入来源快照：实际 %s" % [playback.own[1]]
    )
    _expect(not bool(playback.opponent[1].get("alive", true)), "被吞噬的目标应阵亡")

    playback.advance(320.0)
    _expect(not bool(playback.opponent[2].get("alive", true)), "引爆目标应阵亡")

    playback.advance(320.0)
    _expect(not bool(playback.opponent[0].get("alive", true)), "剧毒目标应阵亡")

    playback.advance(320.0)
    _expect(bool(playback.own[2].get("alive", false)), "复苏后随从应重新存活")
    _expect(int(playback.own[2].get("health", 0)) == 3, "复苏后生命取服务端快照")

    playback.advance(320.0)
    _expect(not bool(playback.own[2].get("alive", true)), "回手后随从应离开棋盘")

    playback.advance(320.0)
    _expect(bool(playback.own[0].get("alive", false)), "零件回收不应改动任何随从快照")


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
    var end_button = instance.find_child("RecruitEndButton", true, false)
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
    var end_button = instance.find_child("RecruitEndButton", true, false)
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
        _expect(
            str(status.text).contains(TranslationServer.translate("ui.game.status.intent_not_sent")),
            "未连接时操作应提示未发出，实际：%s" % status.text,
        )
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
    _expect(
        UI.error_text("NOT_ENOUGH_GOLD") == TranslationServer.translate("error.NOT_ENOUGH_GOLD"),
        "NOT_ENOUGH_GOLD 应有中文文案",
    )
    _expect(
        UI.error_text("INVALID_STAGE") == TranslationServer.translate("error.INVALID_STAGE"),
        "INVALID_STAGE 应有中文文案",
    )
    _expect(UI.error_text("SOME_UNKNOWN_CODE") == "SOME_UNKNOWN_CODE", "未知错误码应回退为原文")


func _check_result() -> void:
    var previous_ranking = GameStore.ranking
    var previous_seed = GameStore.game_seed
    var previous_stats = GameStore.self_stats
    var previous_room_id = GameStore.room_id
    var previous_player_id = GameStore.player_id
    var previous_players = GameStore.room_players
    var previous_view = GameStore.view

    GameStore.player_id = "p0"
    GameStore.room_id = "r_test"
    GameStore.room_players = []
    GameStore.view = {}
    GameStore.ranking = [
        {"playerId": "p0", "place": 1, "survivedTurns": 9, "damageDealt": 31},
        {"playerId": "p1", "place": 2},
    ]
    GameStore.game_seed = "abc123"
    GameStore.self_stats = {
        "place": 1,
        "survivedTurns": 9,
        "finalHp": 17,
        "finalEra": 5,
        "remainingGold": 4,
        "damageDealt": 31,
        "damageTaken": 12,
        "buys": 6,
        "sells": 3,
        "rerolls": 8,
        "triples": 2,
        "combatWins": 4,
        "combatDraws": 1,
        "combatLosses": 2,
        "heroPowerUses": 5,
        "board": [
            {"cardId": "bruce-lee", "golden": true, "attack": 7, "health": 8},
            {"cardId": "aristotle", "golden": false, "attack": 3, "health": 4},
        ],
    }
    var instance := await _check_scene("res://scenes/result.tscn", false)
    if instance == null:
        _restore_result_state(
            previous_ranking, previous_seed, previous_stats, previous_room_id,
            previous_player_id, previous_players, previous_view,
        )
        return

    var list = instance.find_child("RankingList", true, false)
    if list == null:
        _expect(false, "结算场景缺少 RankingList")
    else:
        _expect(list.get_child_count() == 2, "结算场景排名行数为 %d，期望 2" % list.get_child_count())
        if list.get_child_count() == 2:
            var first_row := TranslationServer.translate("ui.result.ranking_row") % [1, "p0", 9, 31]
            var second_row := TranslationServer.translate("ui.result.ranking_row") % [2, "p1", 0, 0]
            _expect(str(list.get_child(0).text) == first_row, "完整排名行不符：%s" % list.get_child(0).text)
            _expect(str(list.get_child(1).text) == second_row, "缺失字段应按 0 回退：%s" % list.get_child(1).text)

    var section = instance.find_child("StatsSection", true, false)
    if section == null:
        _expect(false, "结算场景缺少 StatsSection")
    else:
        var rows := section.find_children("StatRow_*", "", true, false)
        _expect(rows.size() == 12, "结算数据行数为 %d，期望 12" % rows.size())
        var stats_text := _node_text(section)
        for expected in [
            TranslationServer.translate("ui.result.stat.survived_turns") + "：9",
            TranslationServer.translate("ui.result.stat.final_hp") + "：17",
            TranslationServer.translate("ui.result.stat.final_era") + "：" + CardDB.era_name(5),
            TranslationServer.translate("ui.result.stat.remaining_gold") + "：4",
            TranslationServer.translate("ui.result.stat.damage_dealt") + "：31",
            TranslationServer.translate("ui.result.stat.damage_taken") + "：12",
            TranslationServer.translate("ui.result.stat.buys") + "：6",
            TranslationServer.translate("ui.result.stat.sells") + "：3",
            TranslationServer.translate("ui.result.stat.rerolls") + "：8",
            TranslationServer.translate("ui.result.stat.triples") + "：2",
            TranslationServer.translate("ui.result.stat.combat_wins") + "：4",
            TranslationServer.translate("ui.result.stat.combat_draws") + "：1",
            TranslationServer.translate("ui.result.stat.combat_losses") + "：2",
            TranslationServer.translate("ui.result.stat.hero_power_uses") + "：5",
        ]:
            _expect(stats_text.contains(expected), "结算数据缺少「%s」：%s" % [expected, stats_text])

    var board = instance.find_child("BoardList", true, false)
    if board == null:
        _expect(false, "结算场景缺少 BoardList")
    else:
        _expect(board.get_child_count() == 2, "阵容项数为 %d，期望 2" % board.get_child_count())
        var board_text := _node_text(board)
        _expect(board_text.contains("★"), "金色随从应带标记：%s" % board_text)
        _expect(board_text.contains("7/8") and board_text.contains("3/4"), "阵容应显示攻防：%s" % board_text)

    var confirm := instance.find_child("ConfirmButton", true, false)
    _expect(confirm != null, "结算场景缺少确定按钮")
    if confirm != null:
        _expect(
            str(confirm.text) == TranslationServer.translate("ui.result.confirm"),
            "确定按钮文案不符：%s" % confirm.text,
        )
        _expect(
            str(confirm.text) == "确定",
            "确定按钮文案应为「确定」：%s" % confirm.text,
        )
    _expect(
        instance.find_child("BackToRoomButton", true, false) == null,
        "结算页不应再有返回房间按钮",
    )
    _expect(
        instance.find_child("BackToLobbyButton", true, false) == null,
        "结算页不应再有返回大厅按钮",
    )

    # 房间在对局结束时已销毁，结算页不展示任何房间状态提示。
    _expect(
        instance.find_child("RoomClosedNotice", true, false) == null,
        "结算页不应再有房间关闭提示",
    )
    instance.queue_free()
    await get_tree().process_frame

    GameStore.self_stats = {}
    var empty_instance := await _check_scene("res://scenes/result.tscn", false)
    if empty_instance == null:
        _restore_result_state(
            previous_ranking, previous_seed, previous_stats, previous_room_id,
            previous_player_id, previous_players, previous_view,
        )
        return
    var empty_section = empty_instance.find_child("StatsSection", true, false)
    if empty_section == null:
        _expect(false, "空统计时结算场景缺少 StatsSection")
    else:
        var empty_text := _node_text(empty_section)
        for expected in [
            TranslationServer.translate("ui.result.stat.survived_turns") + "：0",
            TranslationServer.translate("ui.result.stat.final_hp") + "：0",
            TranslationServer.translate("ui.result.stat.final_era") + "：" + CardDB.era_name(0),
            TranslationServer.translate("ui.result.stat.remaining_gold") + "：0",
            TranslationServer.translate("ui.result.stat.combat_wins") + "：0",
            TranslationServer.translate("ui.result.stat.hero_power_uses") + "：0",
        ]:
            _expect(empty_text.contains(expected), "空统计应按 0 显示「%s」：%s" % [expected, empty_text])
    var empty_board = empty_instance.find_child("BoardList", true, false)
    if empty_board == null:
        _expect(false, "空阵容时结算场景缺少 BoardList")
    else:
        _expect(empty_board.get_child_count() == 1, "空阵容应只有占位项")
        _expect(
            _node_text(empty_board).contains(TranslationServer.translate("ui.result.board_empty")),
            "空阵容应显示占位文案",
        )
    _expect(empty_instance.find_child("ConfirmButton", true, false) != null, "空统计时缺少确定按钮")
    _expect(
        empty_instance.find_child("BackToRoomButton", true, false) == null,
        "空统计时也不应有返回房间按钮",
    )
    empty_instance.queue_free()
    await get_tree().process_frame

    _restore_result_state(
        previous_ranking, previous_seed, previous_stats, previous_room_id,
        previous_player_id, previous_players, previous_view,
    )


## 结算页点「确定」：清空房间并导航大厅。
func _check_confirm_navigation() -> void:
    GameStore.player_id = "p0"
    GameStore.room_id = "r_test"
    GameStore.room_players = []
    GameStore.view = {}
    GameStore.ranking = []
    GameStore.self_stats = {}
    GameStore.game_seed = "seed"
    var instance := await _check_scene("res://scenes/result.tscn", false)
    if instance == null:
        return
    var confirm = instance.find_child("ConfirmButton", true, false)
    if confirm == null:
        _expect(false, "结算场景缺少确定按钮")
        return

    # 同 room_ops：把结算页挂到 root 并设为当前场景，切场景不会销毁冒烟用例节点。
    remove_child(instance)
    get_tree().root.add_child(instance)
    get_tree().current_scene = instance
    confirm.pressed.emit()
    await get_tree().process_frame
    await get_tree().process_frame

    _expect(GameStore.room_id == "", "点确定后应清空房间，实际：%s" % GameStore.room_id)
    var current := get_tree().current_scene
    _expect(
        current != null and str(current.scene_file_path) == "res://scenes/lobby.tscn",
        "点确定后应切换到大堂，实际：%s"
        % ("<null>" if current == null else str(current.scene_file_path)),
    )

    if current != null and current != self:
        current.queue_free()
    get_tree().current_scene = self
    await get_tree().process_frame
    GameStore.room_id = ""
    GameStore.player_id = ""
    GameStore.room_players = []
    GameStore.ranking = []
    GameStore.self_stats = {}
    GameStore.game_seed = ""
    _fake_disconnect()


func _restore_result_state(
    ranking, seed_text, stats, room_id, player_id, players, view,
) -> void:
    GameStore.ranking = ranking
    GameStore.game_seed = seed_text
    GameStore.self_stats = stats
    GameStore.room_id = room_id
    GameStore.player_id = player_id
    GameStore.room_players = players
    GameStore.view = view
