extends "res://tests/smoke_case.gd"
## 触控冒烟：详情层长按状态机与手牌槽触摸复位。
##
## 用真实 InputEventScreenTouch / InputEventScreenDrag 推到视口，覆盖短按、
## 长按、抬手隐藏、位移取消、多触点与桌面鼠标回退。

const DETAIL_CARD_POSITION := Vector2(100, 100)
const DETAIL_CARD_SIZE := Vector2(112, 146)


func _ready() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    await _check_detail_long_press()
    await _check_room_touch_select()
    await _check_aim_touch()
    await _check_hand_slot_reset()
    ScreenProfile.set_device_for_test("")
    _finish()


## 详情层：触控短按不显示、长按 450ms 显示、抬手隐藏、位移取消；
## 桌面模式下鼠标按下仍按原有行为显示。
func _check_detail_long_press() -> void:
    var stage := Control.new()
    stage.size = Vector2(1280, 720)
    add_child(stage)
    var layer := GameDetailLayer.new()
    stage.add_child(layer)
    var card := Control.new()
    card.position = DETAIL_CARD_POSITION
    card.size = DETAIL_CARD_SIZE
    stage.add_child(card)
    layer.track(card, {"detail_kind": "shop", "cardId": "adam-smith"})
    await get_tree().process_frame

    var point: Vector2 = card.get_global_rect().get_center()

    # 短按不足 450ms：不显示。
    await _push_touch(point, 0, true)
    await get_tree().create_timer(0.1).timeout
    await _push_touch(point, 0, false)
    _expect(not layer.visible, "短按不应显示详情")

    # 长按 450ms：显示；抬手隐藏。
    await _push_touch(point, 0, true)
    await get_tree().create_timer(0.5).timeout
    await get_tree().process_frame
    _expect(layer.visible, "长按 450ms 应显示详情")
    await _push_touch(point, 0, false)
    _expect(not layer.visible, "抬手应隐藏详情")

    # 按下后位移超过 8 个画布单位：取消长按。
    await _push_touch(point, 0, true)
    await _push_drag(point + Vector2(9, 0), 0)
    await get_tree().create_timer(0.5).timeout
    await get_tree().process_frame
    _expect(not layer.visible, "位移超过阈值应取消长按")
    await _push_touch(point + Vector2(9, 0), 0, false)
    _expect(not layer.visible, "取消后抬手不应显示详情")

    # 多触点：第二个触点不改变第一个触点的长按归属。
    await _push_touch(point, 0, true)
    await _push_touch(Vector2(600, 500), 1, true)
    await get_tree().create_timer(0.5).timeout
    await get_tree().process_frame
    _expect(layer.visible, "多触点时应以第一个触点的长按为准")
    await _push_touch(point, 0, false)
    _expect(not layer.visible, "第一个触点抬手应隐藏详情")
    await _push_touch(Vector2(600, 500), 1, false)

    # 模态浮层打开（blocked）时按原有规则不显示。
    layer.blocked = true
    await _push_touch(point, 0, true)
    await get_tree().create_timer(0.5).timeout
    await get_tree().process_frame
    _expect(not layer.visible, "blocked 时不应显示详情")
    await _push_touch(point, 0, false)
    layer.blocked = false

    # 桌面端鼠标按下仍显示，移出隐藏。
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_DESKTOP)
    await _push_button(self, point, true)
    await get_tree().process_frame
    _expect(layer.visible, "桌面模式鼠标按下仍应显示详情")
    await _push_button(self, point, false)
    await _push_motion(self, Vector2(-40, -40))
    _expect(not layer.visible, "桌面鼠标移出后应隐藏详情")
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)

    stage.queue_free()
    await get_tree().process_frame
    await get_tree().process_frame


## 选英雄页：长按显示详情且抬手不选中，短按选中，位移超阈值取消长按且不选中。
func _check_room_touch_select() -> void:
    GameStore.player_id = "p0"
    GameStore.room_id = "r_touch"
    GameStore.room_code = "ABC123"
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "ready": false, "isHost": true,
         "heroCardId": null, "heroCandidates": ["gauss", "mozi"]},
    ]
    var instance := await _check_scene("res://scenes/room.tscn", false)
    if instance == null:
        return
    var candidates := instance.find_child("HeroCandidates", true, false)
    _expect(candidates != null and candidates.get_child_count() == 2, "触控房间页应渲染 2 张候选卡")
    if candidates != null and candidates.get_child_count() == 2:
        var first: Control = candidates.get_child(0)
        var second: Control = candidates.get_child(1)
        var point := first.get_global_rect().get_center()
        var panel := instance.find_child("RoomHeroDetail", true, false)
        var first_frame := first.find_child("SelectedFrame", true, false)
        var second_frame := second.find_child("SelectedFrame", true, false)

        # 长按 450ms 显示详情；抬手隐藏且不选中。
        await _push_touch(point, 0, true)
        await get_tree().create_timer(0.5).timeout
        await get_tree().process_frame
        _expect(panel != null and panel.visible, "长按候选卡应显示详情")
        _expect(
            instance.get("_touch_elapsed_ms") != null
            and float(instance.get("_touch_elapsed_ms")) >= float(GameDetailLayer.LONG_PRESS_MS),
            "页面应按与详情层相同的累计时钟判定长按",
        )
        var touch_layer := instance.find_child("HeroDetailLayer", true, false)
        _expect(
            touch_layer != null and panel != null
            and touch_layer.get_global_rect().encloses(panel.get_global_rect()),
            "手机端详情卡应始终留在安全区内",
        )
        await _push_touch(point, 0, false)
        _expect(panel != null and not panel.visible, "抬手应隐藏详情")
        _expect(first_frame != null and not first_frame.is_visible_in_tree(), "长按抬手不应选中")

        # 短按选中。
        await _push_touch(point, 0, true)
        await get_tree().create_timer(0.1).timeout
        await _push_touch(point, 0, false)
        _expect(first_frame != null and first_frame.is_visible_in_tree(), "短按候选卡应选中")

        # 在第二张上位移超过 8：取消长按且不选中。
        var second_point := second.get_global_rect().get_center()
        await _push_touch(second_point, 0, true)
        await _push_drag(second_point + Vector2(9, 0), 0)
        await get_tree().create_timer(0.5).timeout
        await get_tree().process_frame
        _expect(panel != null and not panel.visible, "位移超过阈值应取消长按")
        await _push_touch(second_point + Vector2(9, 0), 0, false)
        _expect(first_frame != null and first_frame.is_visible_in_tree(), "位移后不应改变原选中")
        _expect(second_frame != null and not second_frame.is_visible_in_tree(), "位移后不应选中第二张")

        # 触控进行中候选被重排：页面必须丢弃这次触点，抬手不得访问已释放的卡片。
        await _push_touch(second_point, 0, true)
        var rebuilt := GameStore.room_players.duplicate(true)
        rebuilt[0]["heroCandidates"] = ["gauss", "mozi", "luban"]
        GameStore.room_players = rebuilt
        GameStore.room_changed.emit()
        await get_tree().process_frame
        await get_tree().process_frame
        _expect(instance.get("_touch_index") == -1, "候选重排后应丢弃进行中的触点")
        _expect(instance.get("_touch_card") == null, "候选重排后不应保留旧卡片引用")
        await _push_touch(second_point, 0, false)
        var rebuilt_cards := instance.find_child("HeroCandidates", true, false)
        var rebuilt_selected := 0
        if rebuilt_cards != null:
            for card in rebuilt_cards.get_children():
                var frame = card.find_child("SelectedFrame", true, false)
                if frame != null and frame.is_visible_in_tree():
                    rebuilt_selected += 1
        _expect(rebuilt_selected == 1, "重排后抬手不得额外选中卡片，实际：%d" % rebuilt_selected)

        # 多触点：第二个触点不接管长按，也不触发选择。
        if rebuilt_cards != null and rebuilt_cards.get_child_count() == 3:
            var finger_card: Control = rebuilt_cards.get_child(1)
            var extra_card: Control = rebuilt_cards.get_child(2)
            var finger_point := finger_card.get_global_rect().get_center()
            var extra_point := extra_card.get_global_rect().get_center()
            await _push_touch(finger_point, 0, true)
            await _push_touch(extra_point, 1, true)
            await get_tree().create_timer(0.5).timeout
            await get_tree().process_frame
            _expect(panel != null and panel.visible, "多触点时应以第一个触点显示详情")
            await _push_touch(extra_point, 1, false)
            await get_tree().process_frame
            _expect(panel != null and panel.visible, "第二个触点抬手不应关闭第一个触点的详情")
            await _push_touch(finger_point, 0, false)
            _expect(panel != null and not panel.visible, "第一个触点抬手应关闭详情")
            var multi_selected := 0
            for card in rebuilt_cards.get_children():
                var multi_frame = card.find_child("SelectedFrame", true, false)
                if multi_frame != null and multi_frame.is_visible_in_tree():
                    multi_selected += 1
            _expect(multi_selected == 1, "多触点抬手不得改选，选中框数量：%d" % multi_selected)

    instance.queue_free()
    await get_tree().process_frame
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.room_players = []

    # 已确认后长按仍显示详情、抬手不得改选，选中框保留在已确认的英雄上。
    GameStore.player_id = "p0"
    GameStore.room_id = "r_touch_locked"
    GameStore.room_code = "ABC123"
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "ready": true, "isHost": true,
         "heroCardId": "gauss", "heroCandidates": ["gauss", "mozi"]},
    ]
    var locked_instance := await _check_scene("res://scenes/room.tscn", false)
    if locked_instance != null:
        var locked_cards := locked_instance.find_child("HeroCandidates", true, false)
        var locked_panel := locked_instance.find_child("RoomHeroDetail", true, false)
        if locked_cards != null and locked_cards.get_child_count() == 2:
            var locked_second: Control = locked_cards.get_child(1)
            var locked_point := locked_second.get_global_rect().get_center()
            await _push_touch(locked_point, 0, true)
            await get_tree().create_timer(0.5).timeout
            await get_tree().process_frame
            _expect(locked_panel != null and locked_panel.visible, "已确认后长按仍应显示详情")
            await _push_touch(locked_point, 0, false)
            _expect(locked_panel != null and not locked_panel.visible, "已确认后抬手应隐藏详情")
            var selected_frames := 0
            for card in locked_cards.get_children():
                var frame = card.find_child("SelectedFrame", true, false)
                if frame != null and frame.is_visible_in_tree():
                    selected_frames += 1
            _expect(selected_frames == 1, "已确认后长按不得改选，选中框数量：%d" % selected_frames)
            var locked_first_frame = locked_cards.get_child(0).find_child("SelectedFrame", true, false)
            _expect(locked_first_frame != null and locked_first_frame.is_visible_in_tree(),
                "已确认后长按不得把选中框移到其他卡")
        locked_instance.queue_free()
        await get_tree().process_frame
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.room_players = []


## 瞄准层：触控按住拖出与松手提交；技能模式由按钮按下开启时也要接管触点编号。
func _check_aim_touch() -> void:
    var stage := Control.new()
    stage.size = Vector2(1280, 720)
    add_child(stage)
    var aim := GameAimLayer.new()
    stage.add_child(aim)
    var source := _aim_card(stage, "board", "mi_src", Vector2(40, 40))
    var candidate := _aim_card(stage, "board", "mi_1", Vector2(300, 300))
    await get_tree().process_frame

    var chosen: Array = []
    var cancelled := []
    aim.target_chosen.connect(func(target: Dictionary) -> void: chosen.append(target))
    aim.aim_cancelled.connect(func() -> void: cancelled.append(true))
    var candidate_point: Vector2 = candidate.get_global_rect().get_center()
    var candidates := [{"zone": "board", "id": "mi_1"}]

    # 技能模式：按钮按下已把 `_pressed` 置真，但本层没见过起始触点。
    aim.begin_skill(source.get_global_rect(), candidates)
    _expect(aim.skill_active(), "技能瞄准应开启")
    await _push_drag(candidate_point, 0)
    _expect(str(aim._hovered.get("id", "")) == "mi_1", "触控拖动应跟随触点并命中候选")
    await _push_touch(candidate_point, 0, false)
    _expect(not aim.skill_active(), "触控抬手后应结束技能瞄准")
    _expect(chosen.size() == 1, "触控抬手应提交命中候选，实际 %d" % chosen.size())
    _expect(cancelled.is_empty(), "命中候选不应发出取消")

    # 待选模式：按下开始瞄准，拖到候选后抬手提交。
    chosen.clear()
    aim.begin_pending("mi_src", candidates)
    _expect(aim.pending_active(), "战吼待选应开启瞄准")
    await _push_touch(Vector2(20, 20), 0, true)
    await _push_drag(candidate_point, 0)
    await _push_touch(candidate_point, 0, false)
    _expect(not aim.pending_active(), "触控抬手后应结束待选瞄准")
    _expect(chosen.size() == 1, "待选触控抬手应提交候选，实际 %d" % chosen.size())

    # 待选模式落在非候选处：抬手视为放弃。
    chosen.clear()
    var abandoned := []
    aim.target_abandoned.connect(func() -> void: abandoned.append(true))
    aim.begin_pending("mi_src", candidates)
    await _push_touch(Vector2(20, 20), 0, true)
    await _push_touch(Vector2(20, 20), 0, false)
    _expect(abandoned.size() == 1 and chosen.is_empty(), "待选落空应发放弃且不提交目标")

    stage.queue_free()
    await get_tree().process_frame


## 瞄准候选用控件：命中只看 `target_zone` / `target_id` 元数据。
func _aim_card(stage: Control, zone: String, id: String, position: Vector2) -> Control:
    var card := Control.new()
    card.position = position
    card.size = Vector2(112, 146)
    card.set_meta("target_zone", zone)
    card.set_meta("target_id", id)
    stage.add_child(card)
    return card


## 手牌槽：触摸释放、拖拽取消、视图刷新都要复位缩放、层级与右侧槽位鼠标过滤。
func _check_hand_slot_reset() -> void:
    var parent := Control.new()
    add_child(parent)
    var slot_zero := HandSlot.new()
    var slot_one := HandSlot.new()
    parent.add_child(slot_zero)
    parent.add_child(slot_one)
    var card_zero := _make_hand_card()
    var card_one := _make_hand_card()
    slot_zero.setup(card_zero, 80.0)
    slot_one.setup(card_one, 80.0)
    await get_tree().process_frame

    var hover_scale := Vector2.ONE * UITheme.factor("hand-hover-scale")

    # 触摸按下触发放大与置顶，右侧槽位让出鼠标。
    slot_zero._on_card_mouse_entered()
    _expect(slot_zero.z_index == 1, "触摸按下后手牌槽应置顶")
    _expect(card_zero.scale.is_equal_approx(hover_scale), "触摸按下后手牌应放大")
    _expect(card_one.mouse_filter == Control.MOUSE_FILTER_IGNORE, "置顶时右侧槽位应让出鼠标")

    # 触摸释放：真实 ScreenTouch 释放事件推到视口，由 HandSlot._input 复位。
    await _push_touch(card_zero.get_global_rect().get_center(), 0, false)
    _expect(slot_zero.z_index == 0, "触摸释放后应恢复层级")
    _expect(card_zero.scale.is_equal_approx(Vector2.ONE), "触摸释放后应恢复原尺寸")
    _expect(card_one.mouse_filter == Control.MOUSE_FILTER_STOP, "触摸释放后应恢复右侧槽位鼠标过滤")

    # 拖拽取消：drag_started 已连接复位。
    slot_zero._on_card_mouse_entered()
    card_zero.drag_started.emit()
    _expect(slot_zero.z_index == 0, "拖拽取消后应恢复层级")
    _expect(card_zero.scale.is_equal_approx(Vector2.ONE), "拖拽取消后应恢复原尺寸")
    _expect(card_one.mouse_filter == Control.MOUSE_FILTER_STOP, "拖拽取消后应恢复右侧槽位鼠标过滤")

    # 拖拽结束通知（取消拖拽）同样复位。
    slot_zero._on_card_mouse_entered()
    slot_zero.notification(Control.NOTIFICATION_DRAG_END)
    _expect(slot_zero.z_index == 0, "拖拽结束通知后应恢复层级")
    _expect(card_zero.scale.is_equal_approx(Vector2.ONE), "拖拽结束通知后应恢复原尺寸")
    _expect(card_one.mouse_filter == Control.MOUSE_FILTER_STOP, "拖拽结束通知后应恢复右侧槽位鼠标过滤")

    # 视图刷新：setup 新卡时显式复位旧状态。
    slot_zero._on_card_mouse_entered()
    var card_two := _make_hand_card()
    slot_zero.setup(card_two, 80.0)
    card_zero.free()
    _expect(slot_zero.z_index == 0, "重建卡片后应恢复层级")
    _expect(card_two.scale.is_equal_approx(Vector2.ONE), "重建卡片后应恢复原尺寸")
    _expect(card_one.mouse_filter == Control.MOUSE_FILTER_STOP, "重建卡片后应恢复右侧槽位鼠标过滤")

    parent.free()
    await get_tree().process_frame


func _make_hand_card() -> HandCard:
    var card := HandCard.new()
    card.custom_minimum_size = Vector2(112, 146)
    return card


func _push_touch(canvas_point: Vector2, index: int, pressed: bool) -> void:
    var event := InputEventScreenTouch.new()
    event.index = index
    event.position = canvas_point
    event.pressed = pressed
    get_viewport().push_input(event, true)
    await get_tree().process_frame


func _push_drag(canvas_point: Vector2, index: int) -> void:
    var event := InputEventScreenDrag.new()
    event.index = index
    event.position = canvas_point
    get_viewport().push_input(event, true)
    await get_tree().process_frame
