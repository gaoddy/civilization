extends "res://tests/smoke_case.gd"
## 首局引导冒烟：默认隐藏、顶部「提示」手动打开、步骤推进、跳过与输入穿透。

const GUIDE_PLAYER := "guide_p0"


func _ready() -> void:
    await _check_default_hidden()
    await _check_manual_and_next()
    await _check_skip()
    await _check_manual_in_combat()
    await _check_input_passthrough()
    GameStore.view = {}
    GameStore.phase = ""
    _finish()


func _recruit_view(has_hand: bool, has_board: bool) -> Dictionary:
    var view: Dictionary = FixtureScript.game_view()
    view["phase"] = Protocol.PHASE_RECRUIT
    if not has_hand:
        view["self"]["hand"] = []
        view["self"]["affordances"]["handCards"] = []
    if not has_board:
        view["self"]["board"] = []
        view["self"]["affordances"]["boardMinions"] = []
    return view


func _mount(player_id: String, view: Dictionary) -> Node:
    GameStore.player_id = player_id
    GameStore.phase = str(view.get("phase", GameStore.phase))
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = view
    _clear_combat()
    var instance := await _check_scene("res://pages/game/game.tscn", false)
    await get_tree().process_frame
    return instance


func _unmount(instance: Node) -> void:
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    GameStore.phase = ""
    GameStore.turn = 1


func _guide(instance: Node):
    return instance.find_child("FirstGameGuide", true, false)


func _guide_visible(instance: Node) -> bool:
    var guide = _guide(instance)
    return guide != null and bool(guide.visible)


func _guide_step(instance: Node) -> int:
    var guide = _guide(instance)
    return -1 if guide == null else int(guide.step)


func _guide_text(instance: Node) -> String:
    var guide = _guide(instance)
    return "" if guide == null else _node_text(guide)


## 1. 进入对局默认不展示；退出后再进仍保持隐藏。
func _check_default_hidden() -> void:
    var view := _recruit_view(true, true)
    var instance := await _mount(GUIDE_PLAYER, view)
    if instance == null:
        return
    _expect(_guide(instance) != null, "对局页应挂载 FirstGameGuide")
    _expect(not _guide_visible(instance), "进入对局不应自动展示引导")
    _expect(_guide_step(instance) == -1, "默认关闭时步骤应为 -1")
    _expect(instance.find_child("HintButton", true, false) != null, "顶部应有 HintButton")
    _expect(instance.find_child("LeaveGameButton", true, false) != null, "顶部应有 LeaveGameButton")
    await _unmount(instance)

    var again := await _mount(GUIDE_PLAYER, view)
    if again != null:
        _expect(not _guide_visible(again), "再次进入对局仍不应自动展示")
        await _unmount(again)


## 2. 点顶部「提示」从第 1 步打开；「下一步」按步骤顺序推进。
func _check_manual_and_next() -> void:
    _fake_connect()
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(false, true))
    if instance == null:
        _fake_disconnect()
        return
    var hint := instance.find_child("HintButton", true, false)
    _expect(hint != null and not hint.disabled, "已连接时「提示」应可用")
    if hint != null:
        await _push_click(instance, hint.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(_guide_visible(instance), "点「提示」后应展示引导")
    _expect(_guide_step(instance) == 0, "应从第 1 步开始，实际 %d" % _guide_step(instance))
    _expect(
        _guide_text(instance).contains(TranslationServer.translate("ui.guide.step1.title")),
        "第 1 步应显示商店与金币标题",
    )

    var next := instance.find_child("GuideNext", true, false)
    _expect(next != null, "引导应有 GuideNext")
    if next != null:
        await _push_click(instance, next.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(_guide_step(instance) == 1, "下一步应到第 2 步，实际 %d" % _guide_step(instance))
    _expect(
        _guide_text(instance).contains(TranslationServer.translate("ui.guide.step2.title")),
        "第 2 步应显示出战标题",
    )
    await _unmount(instance)
    _fake_disconnect()


## 3. 「跳过」只收起引导，不写本地记录。
func _check_skip() -> void:
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true))
    if instance == null:
        return
    var guide = _guide(instance)
    if guide != null:
        guide.show_manual()
    await get_tree().process_frame
    _expect(_guide_visible(instance), "手动打开后应展示引导")

    var skip := instance.find_child("GuideSkip", true, false)
    _expect(skip != null, "引导应有 GuideSkip")
    if skip != null:
        await _push_click(instance, skip.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(not _guide_visible(instance), "跳过后引导应隐藏")
    _expect(_guide_step(instance) == -1, "跳过后步骤应为 -1")
    await _unmount(instance)


## 4. 战斗阶段手动打开：直接从战斗步骤开始。
func _check_manual_in_combat() -> void:
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true))
    if instance == null:
        return
    var guide = _guide(instance)
    if guide != null:
        guide.set_phase(Protocol.PHASE_COMBAT, GameStore.view)
        guide.show_manual()
    await get_tree().process_frame
    _expect(_guide_visible(instance), "战斗阶段手动打开应立即可见")
    _expect(_guide_step(instance) == 3, "战斗阶段应展示战斗步骤")
    await _unmount(instance)


## 5. 引导显示时拖商店卡到手牌带仍能走到购买路径（压暗层不拦截输入）。
func _check_input_passthrough() -> void:
    _fake_connect()
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true))
    if instance == null:
        _fake_disconnect()
        return
    var guide = _guide(instance)
    if guide != null:
        guide.show_manual()
    await get_tree().process_frame
    _expect(_guide_visible(instance), "验证输入穿透时引导应可见")
    var overlay := guide.find_child("GuideOverlay", true, false) if guide != null else null
    var bar := guide.find_child("GuideBar", true, false) if guide != null else null
    var shade := guide.find_child("GuideShade0", true, false) if guide != null else null
    _expect(
        overlay != null and overlay.mouse_filter == Control.MOUSE_FILTER_IGNORE,
        "引导外层应忽略鼠标",
    )
    _expect(
        shade != null and shade.mouse_filter == Control.MOUSE_FILTER_IGNORE,
        "压暗层应忽略鼠标",
    )
    _expect(
        bar != null and bar.mouse_filter == Control.MOUSE_FILTER_STOP,
        "提示条自身应接收点击",
    )
    var captured: Array = []
    instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
    var slot := instance.find_child("ShopSlot0", true, false)
    var hand_row := instance.find_child("HandStrip", true, false)
    _expect(slot != null, "应存在可购买商店槽位")
    _expect(hand_row is Control, "应存在手牌带落点")
    if slot != null and hand_row is Control:
        var rect := (hand_row as Control).get_global_rect()
        await _real_drag(
            instance,
            slot.get_global_rect().get_center(),
            Vector2(rect.position.x + 8.0, rect.end.y - 12.0),
        )
    _expect(
        captured.size() == 1
            and str(captured[0].get("kind", "")) == Protocol.INTENT_BUY_MINION,
        "引导显示时拖商店卡到手牌带应仍购买，实际 %s" % [captured],
    )
    await _unmount(instance)
    _fake_disconnect()
