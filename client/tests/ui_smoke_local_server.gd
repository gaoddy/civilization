extends Node
## 本地调试服务冒烟：断言纯函数、默认不启用状态与房间状态搬运，不启动任何进程。

var _failures := 0


func _ready() -> void:
    _check(LocalServer.room_code_from_line("ROOM_CODE=ABC123") == "ABC123", "房间码")
    _check(LocalServer.room_code_from_line("  ROOM_CODE=XYZ\r") == "XYZ", "前后空白")
    _check(LocalServer.room_code_from_line("ROOM_CODE=") == "", "空房间码")
    _check(LocalServer.room_code_from_line("在客户端大厅输入房间码 ABC123") == "", "日志噪声")
    _check(LocalServer.room_code_from_line("DEV_DONE turns=10 ranking=8") == "", "结束行")
    var paths := LocalServer.sidecar_paths("/nonexistent-local-debug-dir")
    _check(paths["server"].ends_with("/room-service"), "服务端路径")
    _check(paths["sim"].ends_with("/sim"), "机器人路径")
    _check(not paths["server_exists"] and not paths["sim_exists"], "缺失时不启用")
    _check(LocalServer.should_enable(true, ""), "带特性时启用")
    _check(LocalServer.should_enable(false, "1"), "环境变量启用")
    _check(not LocalServer.should_enable(false, ""), "无特性且无环境变量时不启用")
    _check(not LocalServer.should_enable(false, "0"), "环境变量为 0 时不启用")
    _check(LocalServer.sidecar_base_dir("/exec", "/sidecar") == "/sidecar", "sidecar 目录覆盖")
    _check(LocalServer.sidecar_base_dir("/exec", "  ") == "/exec", "sidecar 目录回退")
    _check(not LocalServer.enabled, "无 embedded_server 特性且无环境变量时不启用")
    _check_room_state()
    await _check_lobby_hero_pick_navigation()
    if _failures > 0:
        get_tree().quit(1)
        return
    print("UI_SMOKE_OK")
    get_tree().quit(0)


## 房间等待区状态搬运：补位开关、座位、房主、准备与选英雄信号。
func _check_room_state() -> void:
    GameStore.player_id = "p0"
    var room_payload := {
        "roomId": "r1",
        "code": "ABC123",
        "hostId": "p0",
        "phase": Protocol.PHASE_LOBBY,
        "botFill": true,
        "players": [
            {"playerId": "p0", "displayName": "p0", "ready": true, "isHost": true, "seat": 0},
            {"playerId": "p2", "displayName": "p2", "ready": false, "isHost": false, "seat": 2},
        ],
    }
    GameStore.handle(Protocol.MSG_ROOM_UPDATE, room_payload)
    _check(GameStore.room_bot_fill, "room update bot fill")
    _check(GameStore.my_seat() == 0, "my seat")
    _check(GameStore.am_host(), "am host")
    _check(GameStore.my_ready(), "my ready by seat")

    GameStore.handle(Protocol.MSG_STATE_UPDATE, {
        "view": {"phase": Protocol.PHASE_RECRUIT, "turn": 3},
        "events": [],
    })
    var phase_before := GameStore.phase
    var view_before: Dictionary = GameStore.view
    var hero_events: Array = []
    GameStore.hero_pick_started.connect(
        func() -> void: hero_events.append(GameStore.hero_pick)
    )
    var hero_payload := {
        "deadline": "2026-11-24T12:00:30Z",
        "candidates": ["gauss", "mozi"],
        "players": [{"seat": 0, "displayName": "p0", "isBot": false, "picked": false}],
    }
    GameStore.handle(Protocol.MSG_HERO_PICK_STARTED, hero_payload)
    _check(hero_events.size() == 1, "hero pick signal once")
    _check(GameStore.hero_pick == hero_payload, "hero pick payload")
    _check(GameStore.phase == phase_before, "hero pick keeps phase")
    _check(GameStore.view == view_before, "hero pick keeps view")

    GameStore.handle(Protocol.MSG_GAME_STARTED, {"view": {}})
    _check(GameStore.hero_pick.is_empty(), "game started clears hero pick")

    GameStore.handle(Protocol.MSG_HERO_PICK_STARTED, hero_payload)
    GameStore.clear_game_view()
    _check(GameStore.hero_pick.is_empty(), "clear game view clears hero pick")
    _check(not GameStore.room_bot_fill, "clear game view clears bot fill")

    GameStore.handle(Protocol.MSG_ROOM_UPDATE, room_payload)
    GameStore.handle(Protocol.MSG_HERO_PICK_STARTED, hero_payload)
    GameStore.reset_room()
    _check(GameStore.hero_pick.is_empty(), "reset room clears hero pick")
    _check(not GameStore.room_bot_fill, "reset room clears bot fill")
    GameStore.player_id = ""


## 自动匹配局：大厅页收到 `HeroPickStarted` 后直接进入对局页，重复信号不重复导航。
func _check_lobby_hero_pick_navigation() -> void:
    # 等 root 结束当前场景的初始化，否则 `_ready` 内的 `add_child` 会被拒绝。
    await get_tree().process_frame
    GameStore.player_id = "p0"
    var scene := load(UINavigation.PAGE_LOBBY) as PackedScene
    if scene == null:
        _check(false, "大厅场景存在")
        return
    var instance := scene.instantiate()
    # current_scene 必须是 root 的直接子节点：大厅页挂到 root 后再设为当前场景，
    # 冒烟用例节点仍留在树中继续断言。
    get_tree().root.add_child(instance)
    get_tree().current_scene = instance
    await get_tree().process_frame

    var hero_payload := {
        "deadline": "2026-11-24T12:00:30Z",
        "candidates": ["gauss", "mozi"],
        "players": [{"seat": 0, "displayName": "p0", "isBot": false, "picked": false}],
    }
    GameStore.handle(Protocol.MSG_HERO_PICK_STARTED, hero_payload)
    # 第二次信号不应再切一次场景。
    GameStore.handle(Protocol.MSG_HERO_PICK_STARTED, hero_payload)
    await get_tree().process_frame
    await get_tree().process_frame

    var current := get_tree().current_scene
    _check(
        current != null and str(current.scene_file_path) == UINavigation.PAGE_GAME,
        "HeroPickStarted 后应导航到对局页，实际：%s"
        % ("<null>" if current == null else str(current.scene_file_path)),
    )
    if current != null and current != self:
        current.queue_free()
    get_tree().current_scene = self
    GameStore.hero_pick = {}
    GameStore.player_id = ""
    await get_tree().process_frame


func _check(ok: bool, label: String) -> void:
    if not ok:
        push_error("LOCAL_SERVER_SMOKE_FAIL: " + label)
        _failures += 1
