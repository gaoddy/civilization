extends "res://tests/smoke_case.gd"
## 战斗视图冒烟：顶部恢复对手英雄、双方随从行夹住河道，结果追加在河道回合数之后；
## 演示播完只回报一次 `CombatPlayed`，战斗阶段不显示招募倒计时。
##
## 断言在演示播放中与播放结束两种状态下都要成立。

## 记录发送内容的假连接：只实现战斗回报用例关心的接口。
class RecordingSocket extends RefCounted:
    var sent: Array = []

    func get_ready_state() -> int:
        return WebSocketPeer.STATE_OPEN

    func poll() -> void:
        pass

    func get_available_packet_count() -> int:
        return 0

    func send_text(text: String) -> int:
        sent.append(text)
        return OK


func _ready() -> void:
    await _check_combat_center_only()
    await _check_banner_variants()
    await _check_combat_played_once()
    await _check_started_at_seek()
    await _check_combat_intro_pauses_playback()
    await _check_combat_intro_skips_on_resume()
    await _check_combat_intro_ghost()
    await _check_combat_hides_countdown()
    await _check_attack_collision_flight()
    await _check_fx_local_scaled()
    await _check_death_fades_after_strike()
    await _check_fixed_slots()
    _finish()


func _check_combat_center_only() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    _expect_combat_center(instance, false)
    # 开战画面 1200ms + 3 条动作 × 320ms + 2000ms 横幅停留之外再多推一拍，
    # 确保进入结束态。
    for _step in 12:
        instance._process(0.4)
    _expect_combat_center(instance, true)
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 横幅文案：夹具胜者为我方，显示「胜利」、对手名与伤害；
## 平局夹具显示「平局」与 0 伤害；幽灵战夹具追加「（幽灵）」。
func _check_banner_variants() -> void:
    var draw := FixtureScript.combat_result()
    draw["winnerId"] = null
    draw["damage"] = 0
    await _expect_banner(
        draw,
        [
            TranslationServer.translate("ui.game.combat.draw"),
            "机器人C",
            "0",
        ],
        "平局",
    )
    var ghost := FixtureScript.combat_result()
    ghost["mirror"] = true
    await _expect_banner(
        ghost,
        [
            TranslationServer.translate("ui.game.combat.win"),
            "机器人C" + TranslationServer.translate("ui.game.combat.ghost_suffix"),
        ],
        "幽灵战",
    )


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
    var instance := await _check_scene("res://pages/game/game.tscn", false)
    if instance == null:
        _clear_combat()
        return
    for _step in 12:
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


## 战斗回报：演示播完后发送一次 `CombatPlayed`，同一回合继续推进不重复发送。
func _check_combat_played_once() -> void:
    var socket := RecordingSocket.new()
    Net.socket = socket
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        _fake_disconnect()
        return
    for _step in 12:
        instance._process(0.4)
    var sent := _sent_count(socket, Protocol.MSG_COMBAT_PLAYED)
    _expect(sent == 1, "演示完成后应发送一次 CombatPlayed，实际 %d 次" % sent)
    for _step in 5:
        instance._process(0.4)
    sent = _sent_count(socket, Protocol.MSG_COMBAT_PLAYED)
    _expect(sent == 1, "同一回合继续推进不应重复发送 CombatPlayed，实际 %d 次" % sent)
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""
    _fake_disconnect()


## `startedAt` 已过去较久：首帧就按服务端开战时刻对齐到结束态，不从头重播。
func _check_started_at_seek() -> void:
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
        int(Time.get_unix_time_from_system()) - 10
    ) + "Z"
    _set_combat(combat)
    Sfx.clear_log()
    var instance := await _check_scene("res://pages/game/game.tscn", false)
    if instance == null:
        _clear_combat()
        return
    _expect(
        Sfx.count_cue(Sfx.CUE_COMBAT_START) == 0,
        "续播到结束态不应补播战斗开始音效",
    )
    _expect(
        Sfx.count_cue(Sfx.CUE_COMBAT_END) == 1,
        "续播到结束态应补播一次战斗结束音效",
    )
    var banner = instance.find_child("CombatBanner", true, false)
    _expect(
        banner != null
            and banner.text.contains(TranslationServer.translate("ui.game.combat.win")),
        "startedAt 已过去较久时首帧应直接进入结束态，实际「%s」"
            % ("" if banner == null else banner.text),
    )
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 开战画面：首条动作前出现，显示期间暂停演示；1.2 秒后淡出并恢复推进。
func _check_combat_intro_pauses_playback() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    var intro = instance.find_child("CombatIntro", true, false)
    _expect(intro != null, "首次战斗应创建开战画面")
    if intro != null:
        _expect(intro.visible, "首条动作前开战画面应可见")
        _expect(
            intro.mouse_filter == Control.MOUSE_FILTER_IGNORE,
            "开战画面不应拦截鼠标",
        )
        var caption = intro.find_child("CombatIntroCaption", true, false)
        _expect(
            caption is Label
                and (caption as Label).text
                    == TranslationServer.translate("ui.game.combat.intro")
                and (caption as Label).text != "ui.game.combat.intro",
            "开战画面应显示已翻译的「战斗开始」",
        )
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    var first = (
        opponent_row.get_child(0)
        if opponent_row != null and opponent_row.get_child_count() > 0
        else null
    )
    _expect(
        first != null and _stats_text(first) == "6 / 8",
        "开战画面显示期间不应应用首条动作",
    )
    # 场景装载本身会吃掉几帧，取 0.4 秒留出余量。
    instance._process(0.4)
    _expect(intro != null and intro.visible, "0.4 秒内开战画面应仍在显示")
    first = (
        opponent_row.get_child(0)
        if opponent_row != null and opponent_row.get_child_count() > 0
        else null
    )
    _expect(
        first != null and _stats_text(first) == "6 / 8",
        "开战画面显示期间不应推进战斗演示",
    )
    instance._process(CombatIntro.INTRO_MS / 1000.0)
    _expect(intro != null and not intro.visible, "开战画面应在总时长后淡出")
    instance._process(0.4)
    first = (
        opponent_row.get_child(0)
        if opponent_row != null and opponent_row.get_child_count() > 0
        else null
    )
    _expect(
        first != null and _stats_text(first) == "6 / 2",
        "开战画面结束后应恢复推进并应用首条动作",
    )
    instance.queue_free()
    await get_tree().process_frame
    _reset_store()


## 重连续播进入战斗中段：不显示开战画面。
func _check_combat_intro_skips_on_resume() -> void:
    var combat := FixtureScript.combat_result()
    combat["startedAt"] = Time.get_datetime_string_from_unix_time(
        int(Time.get_unix_time_from_system()) - 1
    ) + "Z"
    var instance := await _combat_instance("2030-01-01T00:00:00Z", combat)
    if instance == null:
        _reset_store()
        return
    var intro = instance.find_child("CombatIntro", true, false)
    _expect(
        intro == null or not intro.visible,
        "续播进入战斗中段不应显示开战画面",
    )
    instance.queue_free()
    await get_tree().process_frame
    _reset_store()


## 幽灵战的开战画面：对手块带幽灵角标。
func _check_combat_intro_ghost() -> void:
    var combat := FixtureScript.combat_result()
    combat["mirror"] = true
    var instance := await _combat_instance("2030-01-01T00:00:00Z", combat)
    if instance == null:
        _reset_store()
        return
    var intro = instance.find_child("CombatIntro", true, false)
    _expect(intro != null and intro.visible, "幽灵战也应显示开战画面")
    if intro != null:
        _expect(
            intro.find_child("GhostBadge", true, false) != null,
            "幽灵战开战画面应带幽灵角标",
        )
    instance.queue_free()
    await get_tree().process_frame
    _reset_store()


## 战斗阶段河道右端是「战斗中」徽标、不出现招募倒计时；回到招募恢复倒计时。
func _check_combat_hides_countdown() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    var recruit_river: Control = instance.find_child("RecruitRiver", true, false)
    _expect(
        recruit_river == null or not recruit_river.is_visible_in_tree(),
        "战斗阶段不应显示招募倒计时",
    )
    var pill: Label = instance.find_child("CombatStatusPill", true, false)
    _expect(
        pill != null
            and pill.is_visible_in_tree()
            and pill.text == TranslationServer.translate("ui.game.side_panel.combat"),
        "战斗河道右端应显示战斗中，实际「%s」" % ("" if pill == null else pill.text),
    )
    instance.queue_free()
    await get_tree().process_frame

    GameStore.phase = Protocol.PHASE_RECRUIT
    var view := FixtureScript.game_view()
    view["phase"] = Protocol.PHASE_RECRUIT
    GameStore.view = view
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var recruit := await _check_scene("res://pages/game/game.tscn", false)
    if recruit != null:
        var combat_river: Control = recruit.find_child("CombatRiver", true, false)
        _expect(
            combat_river == null or not combat_river.is_visible_in_tree(),
            "招募阶段不应显示战斗河道",
        )
        var recruit_timer: Label = recruit.find_child("RecruitTimerLabel", true, false)
        var remaining_prefix := TranslationServer.translate(
            "ui.game.side_panel.remaining_seconds"
        ).split("%")[0]
        _expect(
            recruit_timer != null and recruit_timer.text.contains(remaining_prefix),
            "招募阶段应恢复剩余秒数，实际「%s」" % ("" if recruit_timer == null else recruit_timer.text),
        )
        recruit.queue_free()
        await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 攻方卡主动飞向目标再飞回：飞行中出现覆盖层克隆卡与原位隐藏，回位后原卡恢复。
func _check_attack_collision_flight() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    var source := _combat_card(instance, "mi_0")
    var target := _combat_card(instance, "mo_0")
    _expect(source != null, "攻方卡应在场")
    _expect(target != null, "目标卡应在场")
    if source == null or target == null:
        instance.queue_free()
        await get_tree().process_frame
        _reset_store()
        return
    var fx := instance.find_child("CombatFxLayer", true, false)
    _expect(fx != null, "战斗视图应有演出覆盖层")
    if fx != null:
        _expect(
            fx.mouse_filter == Control.MOUSE_FILTER_IGNORE,
            "演出覆盖层不应拦截鼠标",
        )
    # 开战画面结束后才开始计数；第一条攻击在 320ms 到期。
    instance._process(CombatIntro.INTRO_MS / 1000.0 + 0.1)
    instance._process(0.32)
    var clone := _fx_child(fx, "CombatFxAttack")
    _expect(clone != null, "攻击动作应产生飞行中的克隆卡")
    var source_hidden := source.modulate.a < 0.5
    _expect(
        source_hidden,
        "攻方原卡起飞后应隐藏，实际透明度 %.2f" % source.modulate.a,
    )
    _expect(
        _combat_card(instance, "mo_0") != null,
        "攻击动作不得让目标提前消失",
    )

    # 回程；回位后克隆释放、原卡恢复。
    instance._process(0.1)
    _expect(_fx_child(fx, "CombatFxAttack") != null, "回程中克隆卡应仍在")
    instance._process(0.2)
    _expect(_fx_child(fx, "CombatFxAttack") == null, "回位后克隆卡应释放")
    var returned := _combat_card(instance, "mi_0")
    _expect(
        returned != null and returned.modulate.a > 0.5,
        "攻方回位后原卡应重新显示",
    )
    instance.queue_free()
    await get_tree().process_frame
    _reset_store()


## 缩放内容框下，演出层把画布坐标换算到自身局部时按整体变换计算，
## 不能只减 `global_position`，否则克隆卡与伤害飘字会偏移。
func _check_fx_local_scaled() -> void:
    var host := Control.new()
    host.size = Vector2(1280, 480)
    add_child(host)
    var frame := Control.new()
    frame.size = Vector2(1280, 720)
    frame.scale = Vector2(1280.0 / 1920.0, 1280.0 / 1920.0)
    host.add_child(frame)
    var card := Control.new()
    card.position = Vector2(400, 300)
    card.size = Vector2(84, 92)
    frame.add_child(card)
    var fx := CombatFx.new()
    fx.size = Vector2(1280, 720)
    frame.add_child(fx)
    await get_tree().process_frame
    var expected := fx.get_global_transform().affine_inverse() * card.global_position
    _expect(
        fx._local_point(card.global_position).is_equal_approx(expected),
        "缩放内容框下演出层应把画布坐标换算为局部坐标",
    )
    host.queue_free()
    await get_tree().process_frame


## 阵亡卡在攻方回位后消失：淡出时覆盖层仍有克隆，行内原卡已不可见。
func _check_death_fades_after_strike() -> void:
    var instance := await _combat_instance("2030-01-01T00:00:00Z")
    if instance == null:
        return
    # 开战画面结束后才开始计数；320ms 攻击、640ms 阵亡，推进到 0.66s。
    instance._process(CombatIntro.INTRO_MS / 1000.0 + 0.1)
    instance._process(0.5)
    instance._process(0.16)
    _expect(
        _combat_card(instance, "mo_0") == null,
        "阵亡后目标卡应移出阵容行",
    )
    var fx := instance.find_child("CombatFxLayer", true, false)
    _expect(_fx_child(fx, "CombatFxFade") != null, "阵亡后原位应留一张淡出中的克隆卡")
    instance._process(0.2)
    _expect(_fx_child(fx, "CombatFxFade") == null, "淡出结束后克隆卡应释放")
    instance.queue_free()
    await get_tree().process_frame
    _reset_store()


## 固定占位：一方随从全灭后该行高度与位置不变；结果出现前后两行与河道都不动。
func _check_fixed_slots() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_COMBAT
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view := FixtureScript.game_view()
    view["phase"] = Protocol.PHASE_COMBAT
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    _set_combat(FixtureScript.combat_result_wipe())
    var instance := await _check_scene("res://pages/game/game.tscn", false)
    if instance == null:
        _reset_store()
        return
    var opponent_row: Control = instance.find_child("CombatOpponentRow", true, false)
    var own_row: Control = instance.find_child("CombatOwnRow", true, false)
    var river: Control = instance.find_child("CombatRiver", true, false)
    var banner: Label = instance.find_child("CombatBanner", true, false)
    _expect(opponent_row != null and own_row != null, "战斗视图应有两行随从")
    _expect(river != null and banner != null, "战斗视图应有河道与结果文本")
    if opponent_row == null or own_row == null or river == null or banner == null:
        instance.queue_free()
        await get_tree().process_frame
        _reset_store()
        return
    var opponent_y := opponent_row.global_position.y
    var own_y := own_row.global_position.y
    var river_y := river.global_position.y
    var row_height := opponent_row.size.y
    var river_height := river.size.y
    _expect(
        is_equal_approx(row_height, UITheme.metric("card-height")),
        "随从行应有固定行高 %d，实际 %.1f" % [UITheme.metric("card-height"), row_height],
    )
    _expect(
        is_equal_approx(river_height, UITheme.metric("river-height")),
        "河道应有固定行高 %d，实际 %.1f" % [UITheme.metric("river-height"), river_height],
    )
    _expect(
        opponent_y < river_y and river_y < own_y,
        "对手行、河道、我方行应自上而下排列",
    )
    var turn_text := TranslationServer.translate("ui.game.side_panel.turn") % 3
    _expect(banner.text == turn_text, "结果出现前河道只显示回合数，实际「%s」" % banner.text)

    # 开战画面结束后，320ms 的阵亡动作让对手行清空；
    # 行高与两行位置都必须保持不变。
    instance._process(CombatIntro.INTRO_MS / 1000.0 + 0.1)
    instance._process(0.4)
    await get_tree().process_frame
    _expect(opponent_row.get_child_count() == 0, "对手随从全灭后行内应为空")
    _expect(_placeholder_count(opponent_row) == 0, "空行不应显示提示文案")
    _expect(
        is_equal_approx(opponent_row.size.y, row_height),
        "空行应与有随从时等高，实际 %.1f" % opponent_row.size.y,
    )
    _expect(
        is_equal_approx(opponent_row.global_position.y, opponent_y),
        "对手行不应因清空而移动",
    )
    _expect(
        is_equal_approx(own_row.global_position.y, own_y),
        "我方行不应因对手清空而移动",
    )
    _expect(
        is_equal_approx(river.global_position.y, river_y),
        "河道不应因对手清空而移动",
    )

    # 推进到结果出现：结果追加在回合数之后，两行与河道都不动。
    for _step in 8:
        instance._process(0.4)
    await get_tree().process_frame
    _expect(
        banner.text.contains(TranslationServer.translate("ui.game.combat.win")),
        "演示播完后河道应显示结果，实际「%s」" % banner.text,
    )
    _expect(banner.text.contains(turn_text), "结果应追加在回合数之后，实际「%s」" % banner.text)
    _expect(
        is_equal_approx(river.size.y, river_height),
        "结果出现不应改变河道高度，实际 %.1f" % river.size.y,
    )
    _expect(
        is_equal_approx(opponent_row.global_position.y, opponent_y),
        "结果出现后对手行位置不变",
    )
    _expect(
        is_equal_approx(own_row.global_position.y, own_y),
        "结果出现后我方行位置不变",
    )

    instance.queue_free()
    await get_tree().process_frame
    _reset_store()


## 阵容行内按实例 id 找随从卡。
func _combat_card(instance: Node, minion_id: String) -> Control:
    for row_name in ["CombatOwnRow", "CombatOpponentRow"]:
        var row = instance.find_child(row_name, true, false)
        if row == null:
            continue
        for child in row.get_children():
            if (
                child is Control
                and not child.is_queued_for_deletion()
                and str(child.get_meta("minion_id", "")) == minion_id
            ):
                return child
    return null


## 覆盖层上指定名称且尚未排队释放的节点。
func _fx_child(fx: Node, node_name: String) -> Control:
    if fx == null:
        return null
    for child in fx.get_children():
        if str(child.name) == node_name and not child.is_queued_for_deletion():
            return child
    return null


## 清空战斗摆位，回到初始状态。
func _reset_store() -> void:
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""
    GameStore.deadline = ""


## 已发送的信封中某种消息的数量。
func _sent_count(socket: RecordingSocket, type: String) -> int:
    var count := 0
    for text in socket.sent:
        var envelope = JSON.parse_string(str(text))
        if envelope is Dictionary and str(envelope.get("type", "")) == type:
            count += 1
    return count


## 战斗组合节点：
## - 旧的双方英雄条必须不存在，顶部恢复对手身份块；
## - 对手行、河道、我方行必须保留且顺序固定；
## - 结果文本播放前只有回合数，播放后追加结果。
func _expect_combat_center(instance: Node, finished: bool) -> void:
    var stage := "播放结束后" if finished else "播放中"
    _expect(instance.find_child("CombatOpponentStrip", true, false) == null, "战斗视图不应有对手英雄条（%s）" % stage)
    _expect(instance.find_child("CombatSelfStrip", true, false) == null, "战斗视图不应有我方英雄条（%s）" % stage)

    var opponent_view = instance.find_child("OpponentView", true, false)
    _expect(opponent_view != null, "战斗顶部应恢复对手身份块（%s）" % stage)
    if opponent_view != null:
        var portrait = opponent_view.find_child("OpponentPortrait", true, false)
        _expect(portrait != null, "对手身份块应含头像（%s）" % stage)
        _expect(portrait is GameHeroCard, "对手头像应复用英雄卡组件（%s）" % stage)
        _expect(
            portrait != null and portrait.custom_minimum_size == Vector2(84, 84),
            "对手英雄卡应为 84×84（%s）" % stage,
        )

    var banner = instance.find_child("CombatBanner", true, false)
    var river = instance.find_child("CombatRiver", true, false)
    var combat_gold = instance.find_child("CombatGoldTile", true, false)
    _expect(
        combat_gold != null and combat_gold.is_visible_in_tree(),
        "战斗河道应显示金币（%s）" % stage,
    )
    var opponent_row = instance.find_child("CombatOpponentRow", true, false)
    var own_row = instance.find_child("CombatOwnRow", true, false)
    _expect(river != null, "战斗河道应存在（%s）" % stage)
    _expect(banner != null, "结果文本保留（%s）" % stage)
    _expect(opponent_row != null, "对手随从行仍在（%s）" % stage)
    _expect(own_row != null, "我方随从行仍在（%s）" % stage)
    _expect_combat_row_order(instance)

    if banner == null:
        return
    var turn_text := TranslationServer.translate("ui.game.side_panel.turn") % GameStore.turn
    if finished:
        _expect(
            banner.text.contains(TranslationServer.translate("ui.game.combat.win")),
            "动作播完并停留后应显示结果，实际「%s」" % banner.text,
        )
        _expect(
            banner.text.contains(turn_text),
            "结果应追加在回合数之后，实际「%s」" % banner.text,
        )
    else:
        _expect(
            banner.text == turn_text,
            "动作播完前河道只显示回合数，实际「%s」" % banner.text,
        )
        if opponent_row != null:
            _expect(opponent_row.get_child_count() == 2, "对手随从行应为 2 张")
        if own_row != null:
            _expect(own_row.get_child_count() == 3, "我方随从行应为 3 张")


## 战斗组合子节点顺序：对手身份块 → 对手行 → 河道 → 我方行（间距块可在其间）。
func _expect_combat_row_order(instance: Node) -> void:
    var body = instance.find_child("CombatBody", true, false)
    if body == null:
        _expect(false, "战斗视图应有 CombatBody")
        return
    var order: Array = []
    for position in body.get_child_count():
        order.append(str(body.get_child(position).name))
    var expected := [
        "OpponentView",
        "CombatOpponentGap",
        "CombatOpponentRowCenter",
        "CombatRiverGap",
        "CombatRiver",
        "CombatOwnGap",
        "CombatOwnRowCenter",
    ]
    _expect(
        order == expected,
        "战斗组合子节点顺序应为 %s，实际 %s" % [expected, order],
    )
