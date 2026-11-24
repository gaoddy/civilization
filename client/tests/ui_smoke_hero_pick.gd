extends "res://tests/smoke_case.gd"
## 对局页选英雄覆盖层冒烟：候选、倒计时、座位点、确认锁定、超时提示与释放。


func _ready() -> void:
    await _check_overlay_renders_candidates()
    await _check_overlay_seat_dots()
    await _check_match_seat_resolves_self()
    await _check_confirm_locks()
    await _check_offline_confirm()
    await _check_timeout_hint()
    await _check_overlay_freed_after_start()
    await _check_background_input_blocked()
    await _check_phone_layout_fits()
    _finish()


## 截止时间：当前时刻加指定秒数，转成 RFC3339（UTC）。
func _deadline_in(seconds: int) -> String:
    var target := int(Time.get_unix_time_from_system()) + seconds
    return Time.get_datetime_string_from_unix_time(target) + "Z"


## HeroPickStarted 载荷：4 名候选取样自卡牌数据；players 固定 8 个座位。
func _payload(candidates: Array, picked_seats: Array, deadline := "") -> Dictionary:
    var players: Array = []
    for seat in range(8):
        players.append({
            "seat": seat,
            "displayName": "玩家%d" % seat,
            "isBot": seat > 0,
            "picked": picked_seats.has(seat),
        })
    return {
        "deadline": deadline if deadline != "" else _deadline_in(30),
        "candidates": candidates.duplicate(),
        "players": players,
    }


func _candidates() -> Array:
    return ["adam-smith", "darwin", "gauss", "lavoisier"]


## 挂载对局页；本人座位由 room_players 的 playerId → seat 映射定位。
func _mount(payload: Dictionary) -> Node:
    _fake_connect()
    GameStore.player_id = "p0"
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "seat": 0},
        {"playerId": "p1", "displayName": "机器人B", "seat": 1},
    ]
    GameStore.match_seat = -1
    GameStore.phase = ""
    GameStore.turn = 1
    GameStore.view = {}
    GameStore.hero_pick = payload
    var instance := await _check_scene("res://pages/game/game.tscn", false)
    await get_tree().process_frame
    return instance


func _unmount(instance: Node) -> void:
    if instance != null and is_instance_valid(instance):
        instance.queue_free()
    await get_tree().process_frame
    GameStore.hero_pick = {}
    GameStore.view = {}
    GameStore.phase = ""
    GameStore.room_players = []
    GameStore.match_seat = -1
    _fake_disconnect()


func _overlay_of(instance: Node) -> Control:
    return instance.find_child("HeroPickOverlay", true, false) as Control


## 4 张候选、标题、00:30 倒计时与「已选 0/8」。
func _check_overlay_renders_candidates() -> void:
    var instance := await _mount(_payload(_candidates(), []))
    if instance == null:
        return
    var overlay := _overlay_of(instance)
    _expect(overlay != null, "对局页应挂载 HeroPickOverlay")
    if overlay == null:
        await _unmount(instance)
        return

    var title := overlay.find_child("HeroPickTitle", true, false) as Label
    _expect(
        title != null and title.text == TranslationServer.translate("ui.game.hero_pick.title"),
        "覆盖层应显示选英雄标题",
    )
    var timer := overlay.find_child("HeroPickTimer", true, false) as Label
    # 场景首次装载可能耗时一秒左右，允许 30 秒倒计时显示为 29。
    _expect(
        timer != null and timer.text in ["00:30", "00:29"],
        "倒计时应为 00:30（装载耗时下可为 00:29），实际「%s」" % ("" if timer == null else timer.text),
    )
    var progress := overlay.find_child("HeroPickProgress", true, false) as Label
    _expect(
        progress != null
            and progress.text == TranslationServer.translate("ui.game.hero_pick.progress") % 0,
        "应显示已选 0/8，实际「%s」" % ("" if progress == null else progress.text),
    )

    var list := overlay.find_child("HeroPickCandidates", true, false) as HeroPickList
    _expect(list != null, "覆盖层应有候选行")
    if list != null:
        _expect(list.cards().size() == 4, "候选行应有 4 张卡，实际 %d" % list.cards().size())
        for index in range(list.cards().size()):
            var card := list.cards()[index]
            _expect(
                card.hero_id == _candidates()[index],
                "候选卡顺序应与载荷一致，第 %d 张为 %s" % [index, card.hero_id],
            )

    await _unmount(instance)


## 3 个 picked 座位点，本人点带金色外圈。
func _check_overlay_seat_dots() -> void:
    var instance := await _mount(_payload(_candidates(), [0, 1, 2]))
    if instance == null:
        return
    var overlay := _overlay_of(instance)
    if overlay == null:
        _expect(false, "对局页应挂载 HeroPickOverlay")
        await _unmount(instance)
        return

    var seats := overlay.find_child("HeroPickSeats", true, false) as HBoxContainer
    _expect(seats != null and seats.get_child_count() == 8, "座位点应有 8 个")
    var picked_count := 0
    var self_dot: Control = null
    var other_ring_visible := false
    if seats != null:
        for child in seats.get_children():
            var dot := child as Control
            if bool(dot.get_meta("picked", false)):
                picked_count += 1
            if bool(dot.get_meta("is_self", false)):
                self_dot = dot
            elif dot.find_child("SeatRing", true, false) is Control:
                other_ring_visible = other_ring_visible or (dot.find_child("SeatRing", true, false) as Control).visible
    _expect(picked_count == 3, "应有 3 个已选座位点，实际 %d" % picked_count)
    _expect(self_dot != null, "本人座位点应可识别")
    if self_dot != null:
        _expect(int(self_dot.get_meta("seat", -1)) == 0, "本人应为座位 0")
        var ring := self_dot.find_child("SeatRing", true, false) as Control
        _expect(ring != null and ring.visible, "本人座位点应显示金色外圈")
        if ring != null:
            var ring_style := ring.get_theme_stylebox("panel") as StyleBoxFlat
            _expect(
                ring_style != null and ring_style.border_color.is_equal_approx(UITheme.color("color-accent")),
                "本人座位外圈应使用强调色",
            )
    _expect(not other_ring_visible, "非本人座位点不应显示外圈")

    # 本人座位（座位 0）已在服务端 picked：重连后应回填为已确认态。
    var progress := overlay.find_child("HeroPickProgress", true, false) as Label
    _expect(
        progress != null
            and progress.text == TranslationServer.translate("ui.game.hero_pick.progress") % 3,
        "已选进度应为已选 3/8，实际「%s」" % ("" if progress == null else progress.text),
    )
    var confirm := overlay.find_child("HeroPickConfirm", true, false) as Button
    _expect(
        confirm != null and confirm.disabled
            and confirm.text == TranslationServer.translate("ui.game.hero_pick.confirmed"),
        "服务端本人已选时确认按钮应回填为已确认",
    )
    var list := overlay.find_child("HeroPickCandidates", true, false) as HeroPickList
    _expect(list != null and list.cards().size() == 4, "覆盖层应有 4 张候选")
    if list != null:
        for card in list.cards():
            _expect(card.locked, "服务端本人已选时候选应锁定")

    await _unmount(instance)


## 匹配局不下发 `RoomUpdate`：本人座位必须回退 `match_seat`，并按服务端 picked 回填锁定态。
func _check_match_seat_resolves_self() -> void:
    _fake_connect()
    GameStore.player_id = "p0"
    GameStore.room_players = []
    GameStore.match_seat = 2
    GameStore.phase = ""
    GameStore.turn = 1
    GameStore.view = {}
    GameStore.hero_pick = _payload(_candidates(), [2])
    var instance := await _check_scene("res://pages/game/game.tscn", false)
    if instance == null:
        GameStore.match_seat = -1
        GameStore.hero_pick = {}
        _fake_disconnect()
        return
    await get_tree().process_frame

    var overlay := _overlay_of(instance)
    _expect(overlay != null, "匹配局应挂载 HeroPickOverlay")
    if overlay == null:
        await _unmount(instance)
        return

    var seats := overlay.find_child("HeroPickSeats", true, false) as HBoxContainer
    var self_dot: Control = null
    if seats != null:
        for child in seats.get_children():
            var dot := child as Control
            if bool(dot.get_meta("is_self", false)):
                self_dot = dot
    _expect(self_dot != null, "匹配局本人座位点应可识别")
    if self_dot != null:
        _expect(int(self_dot.get_meta("seat", -1)) == 2, "匹配局本人应为座位 2")
        var ring := self_dot.find_child("SeatRing", true, false) as Control
        _expect(ring != null and ring.visible, "匹配局本人座位点应显示金色外圈")

    var confirm := overlay.find_child("HeroPickConfirm", true, false) as Button
    _expect(
        confirm != null and confirm.disabled
            and confirm.text == TranslationServer.translate("ui.game.hero_pick.confirmed"),
        "匹配局服务端本人已选时确认按钮应为已确认并禁用",
    )
    var list := overlay.find_child("HeroPickCandidates", true, false) as HeroPickList
    _expect(list != null and list.cards().size() == 4, "匹配局应显示 4 张候选")
    if list != null:
        for card in list.cards():
            _expect(card.locked, "匹配局服务端本人已选时候选应锁定")

    # 后续广播把本座位 picked 改回 false：锁定态必须按服务端权威解除。
    GameStore.hero_pick = _payload(_candidates(), [0, 1])
    GameStore.hero_pick_started.emit()
    await get_tree().process_frame

    var refreshed := _overlay_of(instance)
    _expect(refreshed != null, "广播后覆盖层应仍在")
    if refreshed != null:
        var refreshed_list := refreshed.find_child("HeroPickCandidates", true, false) as HeroPickList
        var refreshed_confirm := refreshed.find_child("HeroPickConfirm", true, false) as Button
        _expect(
            refreshed_confirm != null
                and refreshed_confirm.text == TranslationServer.translate("ui.game.hero_pick.confirm"),
            "广播解锁后确认按钮应恢复原文案",
        )
        if refreshed_list != null and refreshed_confirm != null and not refreshed_list.cards().is_empty():
            var first := refreshed_list.cards()[0]
            _expect(not first.locked, "广播解锁后候选应解除锁定")
            await _push_click(instance, first.get_global_rect().get_center())
            _expect(not refreshed_confirm.disabled, "广播解锁后选中候选应可确认")

    await _unmount(instance)


## 点确认后按钮禁用且文案改「已确认」，再点击其他卡不改变选中。
func _check_confirm_locks() -> void:
    var instance := await _mount(_payload(_candidates(), []))
    if instance == null:
        return
    var overlay := _overlay_of(instance)
    if overlay == null:
        _expect(false, "对局页应挂载 HeroPickOverlay")
        await _unmount(instance)
        return

    var list := overlay.find_child("HeroPickCandidates", true, false) as HeroPickList
    var confirm := overlay.find_child("HeroPickConfirm", true, false) as Button
    _expect(list != null and confirm != null, "候选行与确认按钮应存在")
    if list == null or confirm == null or list.cards().size() < 2:
        await _unmount(instance)
        return

    var first := list.cards()[0]
    var second := list.cards()[1]
    await _push_click(instance, first.get_global_rect().get_center())
    _expect(first.selected, "点击候选卡应选中")
    _expect(not confirm.disabled, "选中后确认按钮应可用")

    await _push_click(instance, confirm.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(confirm.disabled, "确认后按钮应禁用")
    _expect(
        confirm.text == TranslationServer.translate("ui.game.hero_pick.confirmed"),
        "确认后按钮文案应为已确认，实际「%s」" % confirm.text,
    )
    _expect(first.selected and first.locked, "确认后本人候选应保持选中并锁定")
    _expect(second.locked, "确认后其余候选应锁定")

    await _push_click(instance, second.get_global_rect().get_center())
    _expect(first.selected and not second.selected, "确认后再点击其他卡不应改变选中")

    await _unmount(instance)


## 断线时确认不发出意图、不本地锁定，并在覆盖层内提示；重连后确认恢复可用。
func _check_offline_confirm() -> void:
    var instance := await _mount(_payload(_candidates(), []))
    if instance == null:
        return
    var overlay := _overlay_of(instance)
    if overlay == null:
        _expect(false, "对局页应挂载 HeroPickOverlay")
        await _unmount(instance)
        return

    var list := overlay.find_child("HeroPickCandidates", true, false) as HeroPickList
    var confirm := overlay.find_child("HeroPickConfirm", true, false) as Button
    var status := overlay.find_child("HeroPickStatusLine", true, false) as Label
    if list == null or confirm == null or status == null or list.cards().is_empty():
        _expect(false, "候选行、确认按钮与状态行应存在")
        await _unmount(instance)
        return

    var first := list.cards()[0]
    await _push_click(instance, first.get_global_rect().get_center())
    _fake_disconnect()
    await _push_click(instance, confirm.get_global_rect().get_center())
    await get_tree().process_frame

    _expect(not first.locked, "断线时确认不应本地锁定候选")
    _expect(
        confirm.text == TranslationServer.translate("ui.game.hero_pick.confirm"),
        "断线时确认按钮应保持原文案，实际「%s」" % confirm.text,
    )
    _expect(status.visible, "断线时应在覆盖层内显示提示")
    _expect(
        status.text == TranslationServer.translate("ui.game.status.disconnected"),
        "断线提示应为已断开，实际「%s」" % status.text,
    )

    _fake_connect()
    await _push_click(instance, confirm.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(first.selected and first.locked, "重连后确认应锁定候选")
    _expect(
        confirm.text == TranslationServer.translate("ui.game.hero_pick.confirmed") and confirm.disabled,
        "重连后确认按钮应变为已确认并禁用",
    )

    await _unmount(instance)


## 截止时间已过：倒计时停在 00:00 并显示超时提示。
func _check_timeout_hint() -> void:
    var instance := await _mount(_payload(_candidates(), [], "2000-01-01T00:00:00Z"))
    if instance == null:
        return
    var overlay := _overlay_of(instance)
    if overlay == null:
        _expect(false, "对局页应挂载 HeroPickOverlay")
        await _unmount(instance)
        return

    var timer := overlay.find_child("HeroPickTimer", true, false) as Label
    var timeout := overlay.find_child("HeroPickTimeout", true, false) as Label
    _expect(timer != null and timer.text == "00:00", "超时后倒计时应停在 00:00")
    _expect(
        timeout != null and timeout.visible
            and timeout.text == TranslationServer.translate("ui.game.hero_pick.timeout"),
        "超时后应显示超时提示",
    )
    var list := overlay.find_child("HeroPickCandidates", true, false) as HeroPickList
    var confirm := overlay.find_child("HeroPickConfirm", true, false) as Button
    if list != null and confirm != null and not list.cards().is_empty():
        await _push_click(instance, list.cards()[0].get_global_rect().get_center())
        _expect(confirm.disabled, "超时后确认按钮应保持禁用")

    await _unmount(instance)


## hero_pick 清空后 `_refresh` 释放覆盖层，页面恢复正常。
func _check_overlay_freed_after_start() -> void:
    var instance := await _mount(_payload(_candidates(), []))
    if instance == null:
        return
    _expect(_overlay_of(instance) != null, "初始应挂载覆盖层")

    GameStore.hero_pick = {}
    instance._refresh()
    await get_tree().process_frame
    await get_tree().process_frame
    _expect(_overlay_of(instance) == null, "选英雄结束后应释放覆盖层")
    _expect(is_instance_valid(instance) and instance.is_inside_tree(), "释放覆盖层后页面应仍可用")

    await _unmount(instance)


## 覆盖层接收输入并铺满整屏；背景使用 0.82 遮罩。
func _check_background_input_blocked() -> void:
    var instance := await _mount(_payload(_candidates(), []))
    if instance == null:
        return
    var overlay := _overlay_of(instance)
    if overlay == null:
        _expect(false, "对局页应挂载 HeroPickOverlay")
        await _unmount(instance)
        return

    _expect(overlay.mouse_filter == Control.MOUSE_FILTER_STOP, "覆盖层应接收输入")
    var page := instance as Control
    var page_rect := page.get_global_rect()
    var overlay_rect := overlay.get_global_rect()
    _expect(
        overlay_rect.position.is_equal_approx(page_rect.position)
            and overlay_rect.size.is_equal_approx(page_rect.size),
        "覆盖层应铺满整屏：页面 %s，覆盖层 %s" % [str(page_rect), str(overlay_rect)],
    )
    var scrim := overlay.find_child("HeroPickScrim", true, false) as ColorRect
    _expect(
        scrim != null and scrim.color.is_equal_approx(UITheme.alpha("color-bg", 0.82)),
        "背景遮罩应为 color-bg 0.82",
    )
    var parent := overlay.get_parent()
    _expect(
        parent != null and parent.get_child(parent.get_child_count() - 1) == overlay,
        "覆盖层应位于页面所有覆盖层之上",
    )

    await _unmount(instance)


## 手机档位 960×540：4 张候选、座位点与确认按钮全部落在覆盖层内。
func _check_phone_layout_fits() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    GameStore.player_id = "p0"
    GameStore.room_players = [{"playerId": "p0", "displayName": "玩家A", "seat": 0}]
    var host := Control.new()
    host.name = "PhoneHost"
    host.size = Vector2(960, 540)
    host.custom_minimum_size = host.size
    add_child(host)
    await get_tree().process_frame

    var overlay := HeroPickOverlay.new()
    overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
    host.add_child(overlay)
    overlay.setup(_payload(_candidates(), []), "p0")
    await get_tree().process_frame
    await get_tree().process_frame

    var bounds := Rect2(Vector2.ZERO, host.size)
    var column := overlay.find_child("HeroPickColumn", true, false) as Control
    _expect(
        column != null and bounds.encloses(column.get_global_rect()),
        "手机档位下覆盖层内容不应溢出：%s" % ("" if column == null else str(column.get_global_rect())),
    )
    var list := overlay.find_child("HeroPickCandidates", true, false) as HeroPickList
    var confirm := overlay.find_child("HeroPickConfirm", true, false) as Control
    _expect(list != null and list.cards().size() == 4, "手机档位仍应显示 4 张候选")
    if list != null:
        var expected_width := roundi(
            float(UITheme.metric("hero-detail-width")) * HeroPickList.PHONE_WIDTH_FACTOR
        )
        var cards_width := float(UITheme.space("space-xs")) * 3.0
        for card in list.cards():
            _expect(
                card.custom_minimum_size.x == expected_width,
                "手机档位候选卡宽应为 0.78 缩放后的 %d，实际 %d" % [expected_width, card.custom_minimum_size.x],
            )
            cards_width += card.custom_minimum_size.x
        _expect(
            cards_width <= host.size.x,
            "手机档位 4 张候选总宽 %.1f 超出 %.1f" % [cards_width, host.size.x],
        )
    _expect(
        confirm != null and bounds.encloses(confirm.get_global_rect()),
        "手机档位确认按钮不应溢出",
    )

    host.queue_free()
    await get_tree().process_frame
    GameStore.room_players = []
    ScreenProfile.set_device_for_test("")
