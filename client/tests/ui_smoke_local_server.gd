extends Node
## 本地调试服务冒烟：只断言纯函数与无特性时的启用状态，不启动任何进程。

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
    _check(not LocalServer.enabled, "无 embedded_server 特性时不启用")
    if _failures > 0:
        get_tree().quit(1)
        return
    print("UI_SMOKE_OK")
    get_tree().quit(0)


func _check(ok: bool, label: String) -> void:
    if not ok:
        push_error("LOCAL_SERVER_SMOKE_FAIL: " + label)
        _failures += 1
