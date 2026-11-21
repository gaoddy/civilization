class_name SmokeCase
extends Node
## 冒烟用例公共基座：场景装载、真实输入事件注入、战斗状态摆位与结果输出。
##
## 具体用例脚本 `extends "res://tests/smoke_case.gd"`，在 `_ready` 末尾调用
## `_finish()`；成功打印 `UI_SMOKE_OK` 并以 0 退出。

const FixtureScript = preload("res://tests/fixture.gd")

var _failures := 0


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


## 按住左键的鼠标移动；`relative` 用于累计拖拽阈值。
func _push_motion_held(instance: Node, canvas_point: Vector2, relative: Vector2) -> void:
    var event := InputEventMouseMotion.new()
    event.position = canvas_point
    event.global_position = canvas_point
    event.relative = relative
    event.button_mask = MOUSE_BUTTON_MASK_LEFT
    instance.get_viewport().push_input(event, true)
    await get_tree().process_frame


## 真实鼠标拖拽：按下起手、越过阈值、移动到落点、松手。
func _real_drag(instance: Node, start: Vector2, target: Vector2) -> void:
    await _push_motion(instance, start)
    await _push_button(instance, start, true)
    await _push_motion_held(instance, start + Vector2(24, 0), Vector2(24, 0))
    await _push_motion_held(instance, target, target - start - Vector2(24, 0))
    await _push_button(instance, target, false)
    await get_tree().process_frame


## 画布坐标处的鼠标按下/松开事件。
func _push_button(instance: Node, canvas_point: Vector2, pressed: bool) -> void:
    var event := InputEventMouseButton.new()
    event.button_index = MOUSE_BUTTON_LEFT
    event.pressed = pressed
    event.position = canvas_point
    event.global_position = canvas_point
    event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
    instance.get_viewport().push_input(event, true)
    await get_tree().process_frame


## 把一次按键按下推给页面（`Esc` 等单次按键）。
func _push_key(instance: Node, keycode: Key) -> void:
    var event := InputEventKey.new()
    event.keycode = keycode
    event.physical_keycode = keycode
    event.pressed = true
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


## 构造战斗阶段的场景，返回实例（调用方负责释放）；`combat` 为空时用默认战报。
func _combat_instance(deadline: String, combat: Dictionary = {}) -> Node:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_COMBAT
    GameStore.turn = 3
    GameStore.deadline = deadline
    var view := FixtureScript.game_view()
    view["phase"] = Protocol.PHASE_COMBAT
    view["opponentPlayerId"] = "p2"
    GameStore.view = view
    _set_combat(combat if not combat.is_empty() else FixtureScript.combat_result())
    return await _check_scene("res://pages/game/game.tscn", false)


func _stats_text(panel: Control) -> String:
    var attack = panel.find_child("CardAttack", true, false)
    var health = panel.find_child("CardHealth", true, false)
    if attack == null or health == null:
        return ""
    return "%s / %s" % [str(attack.text), str(health.text)]


## 给当前回合装上战报（生产路径由 `CombatResult` 消息写入，这里直接摆状态）。
func _set_combat(combat: Dictionary) -> void:
    GameStore.last_combat = combat
    GameStore.last_combat_turn = GameStore.turn


func _clear_combat() -> void:
    GameStore.last_combat = {}
    GameStore.last_combat_turn = -1


## 装作已连接：`Net.is_open()` 只看 `socket` 的就绪状态。
func _fake_connect() -> void:
    Net.socket = FixtureScript.FakeSocket.new()


func _fake_disconnect() -> void:
    Net.socket = null


## 行内的等待提示：是控件但没有 `minion_id` 元数据（随从卡一定带该元数据）。
func _placeholder_count(row: Node) -> int:
    if row == null:
        return 0
    var count := 0
    for child in row.get_children():
        if child is Control and not child.has_meta("minion_id"):
            count += 1
    return count


## 目标待选的视图夹具：队首是 target 待选，棋盘两个随从。
func _target_pending_view() -> Dictionary:
    var view: Dictionary = FixtureScript.view()
    view["self"]["board"] = [
        FixtureScript.board_minion("mi_1", "baiyi", 0),
        FixtureScript.board_minion("mi_2", "airplane", 1),
    ]
    view["self"]["hand"] = [FixtureScript.hand("archimedes", false)]
    var choice := {
        "sourceMinionId": "mi_1",
        "candidates": [
            {"zone": "board", "id": "mi_1"},
            {"zone": "board", "id": "mi_2"},
        ],
    }
    choice["kind"] = "target"
    view["self"]["pendingChoices"] = [choice]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
    ]
    return view


## 输出冒烟结果：全部通过打印标记并以 0 退出，否则以 1 退出。
func _finish() -> void:
    if _failures == 0:
        print("UI_SMOKE_OK")
        get_tree().quit(0)
    else:
        get_tree().quit(1)
