extends Node
## 无头冒烟：实例化各场景，确认脚本与控件构建无错误。

const SCENES := [
    "res://scenes/login.tscn",
    "res://scenes/lobby.tscn",
    "res://scenes/room.tscn",
]

var _failures := 0


func _ready() -> void:
    for path in SCENES:
        var packed = load(path)
        if packed == null:
            push_error("无法加载场景 %s" % path)
            _failures += 1
            continue
        var instance = packed.instantiate()
        if instance == null:
            push_error("无法实例化场景 %s" % path)
            _failures += 1
            continue
        add_child(instance)
        await get_tree().process_frame
        await get_tree().process_frame
        instance.queue_free()
    if _failures == 0:
        print("UI_SMOKE_OK")
        get_tree().quit(0)
    else:
        get_tree().quit(1)
