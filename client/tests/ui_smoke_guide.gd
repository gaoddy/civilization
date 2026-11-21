extends "res://tests/smoke_case.gd"
## T10 首局引导冒烟：自动展示、步骤推进与条件跳过、跳过落盘、手动重看、
## 输入穿透、重连续接与无本地记录场景。

const CONFIG_PATH := "user://first_game_guide.cfg"
const SEEN_SECTION := "seen"
const GUIDE_PLAYER := "guide_p0"


func _ready() -> void:
    # 用独立 playerId；先清掉本地记录，确保首次展示路径可复现。
    _remove_config_file()
    await _check_first_show()
    await _check_next_skips_step2()
    await _check_skip_marks_seen()
    await _check_manual()
    await _check_manual_in_combat()
    await _check_input_passthrough()
    await _check_resume_step()
    await _check_missing_config()
    _remove_config_file()
    GameStore.guide_step = -1
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


func _mount(player_id: String, view: Dictionary, guide_step := -1) -> Node:
    GameStore.player_id = player_id
    GameStore.phase = str(view.get("phase", GameStore.phase))
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.guide_step = guide_step
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
    GameStore.guide_step = -1


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


func _seen(player_id: String) -> bool:
    var config := ConfigFile.new()
    if config.load(CONFIG_PATH) != OK:
        return false
    return bool(config.get_value(SEEN_SECTION, player_id, false))


func _remove_config_file() -> void:
    if FileAccess.file_exists(CONFIG_PATH):
        DirAccess.remove_absolute(ProjectSettings.globalize_path(CONFIG_PATH))


## 1. 新 playerId 首次进入：引导显示第 1 步，金币格与右栏入口节点名固定。
func _check_first_show() -> void:
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true))
    if instance == null:
        return
    _expect(_guide(instance) != null, "对局页应挂载 FirstGameGuide")
    _expect(_guide_visible(instance), "首次进入应自动展示引导")
    _expect(_guide_step(instance) == 0, "首次应显示第 1 步，实际 %d" % _guide_step(instance))
    _expect(
        _guide_text(instance).contains(TranslationServer.translate("ui.guide.step1.title")),
        "第 1 步应显示商店与金币标题",
    )
    _expect(instance.find_child("GoldTile", true, false) != null, "金币格应命名为 GoldTile")
    _expect(instance.find_child("HintButton", true, false) != null, "右栏应有 HintButton")
    _expect(instance.find_child("LeaveGameButton", true, false) != null, "右栏应有 LeaveGameButton")
    await _unmount(instance)


## 2. 「下一步」推进；招募阶段无手牌时自动跳过第 2 步。
func _check_next_skips_step2() -> void:
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(false, true))
    if instance == null:
        return
    _expect(_guide_step(instance) == 0, "无手牌时仍应先显示第 1 步")
    var next := instance.find_child("GuideNext", true, false)
    _expect(next != null, "引导应有 GuideNext")
    if next != null:
        await _push_click(instance, next.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(
        _guide_step(instance) == 2,
        "无手牌时应跳过第 2 步到第 3 步，实际 %d" % _guide_step(instance),
    )
    _expect(
        _guide_text(instance).contains(TranslationServer.translate("ui.guide.step3.title")),
        "跳过第 2 步后应显示结束招募标题",
    )
    await _unmount(instance)


## 3. 「跳过」写入 seen；重建页面不再自动展示。
func _check_skip_marks_seen() -> void:
    var view := _recruit_view(true, true)
    var instance := await _mount(GUIDE_PLAYER, view)
    if instance == null:
        return
    var skip := instance.find_child("GuideSkip", true, false)
    _expect(skip != null, "引导应有 GuideSkip")
    if skip != null:
        await _push_click(instance, skip.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(not _guide_visible(instance), "跳过后引导应隐藏")
    _expect(int(GameStore.guide_step) == -1, "跳过后会话步骤应为 -1")
    _expect(_seen(GUIDE_PLAYER), "跳过后应写入 seen")
    await _unmount(instance)

    var again := await _mount(GUIDE_PLAYER, view)
    if again != null:
        _expect(not _guide_visible(again), "已看过时重建页面不应自动展示")
        await _unmount(again)


## 4. 手动重看从第 1 步开始，且不读写 seen。
func _check_manual() -> void:
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true))
    if instance == null:
        return
    var guide = _guide(instance)
    _expect(guide != null and not guide.visible, "已看过时默认不展示")
    if guide != null:
        guide.show_manual()
    await get_tree().process_frame
    _expect(_guide_visible(instance), "手动重看应展示引导")
    _expect(_guide_step(instance) == 0, "手动重看应从第 1 步开始")
    _expect(_seen(GUIDE_PLAYER), "手动重看不应清除 seen")

    var skip := instance.find_child("GuideSkip", true, false)
    if skip != null:
        await _push_click(instance, skip.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(int(GameStore.guide_step) == -1, "手动关闭应清空会话步骤")
    _expect(_seen(GUIDE_PLAYER), "手动关闭不应改写 seen")
    await _unmount(instance)


## 5. 战斗阶段手动重看：不受阶段门控，立即展示战斗步骤。
func _check_manual_in_combat() -> void:
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true))
    if instance == null:
        return
    var guide = _guide(instance)
    if guide != null:
        guide.set_phase(Protocol.PHASE_COMBAT, GameStore.view)
        guide.show_manual()
    await get_tree().process_frame
    _expect(_guide_visible(instance), "战斗阶段手动重看应立即可见")
    _expect(_guide_step(instance) == 3, "战斗阶段手动重看应展示战斗步骤")
    await _unmount(instance)


## 6. 引导显示时拖商店卡到手牌带仍能走到购买路径（压暗层不拦截输入）。
func _check_input_passthrough() -> void:
    _remove_config_file()
    _fake_connect()
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true))
    if instance == null:
        _fake_disconnect()
        return
    _expect(_guide_visible(instance), "验证输入穿透时引导应可见")
    var guide = _guide(instance)
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


## 6. GameStore.guide_step = 2 后重建：直接恢复第 3 步。
func _check_resume_step() -> void:
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true), 2)
    if instance == null:
        return
    _expect(_guide_visible(instance), "恢复会话步骤时应展示引导")
    _expect(_guide_step(instance) == 2, "应从 guide_step 恢复第 3 步")
    _expect(
        _guide_text(instance).contains(TranslationServer.translate("ui.guide.step3.title")),
        "恢复后应显示结束招募标题",
    )
    await _unmount(instance)


## 7. 本机无 first_game_guide.cfg 时不报错，按首次展示。
func _check_missing_config() -> void:
    _remove_config_file()
    _expect(not FileAccess.file_exists(CONFIG_PATH), "前置条件：本地记录文件应不存在")
    var instance := await _mount(GUIDE_PLAYER, _recruit_view(true, true))
    if instance == null:
        return
    _expect(_guide_visible(instance), "无本地记录时应按首次展示")
    _expect(_guide_step(instance) == 0, "无本地记录时应停在第 1 步")
    await _unmount(instance)
