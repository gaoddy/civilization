extends Node
## 页面线框图探针：装载页面、解算真实布局，把可见 Control 的全局矩形导出为 JSON。
##
## 必须作为项目主场景运行，例如
## `godot --headless --path client res://tests/wireframe_probe.tscn -- --wireframe-scene=...`；
## `--script` 模式不注册 autoload，页面脚本会因缺全局标识符编译失败。
## 用户参数：`--wireframe-scene=res://...`、`--wireframe-out=/绝对路径.json`、
## `--wireframe-device=phone|tablet|desktop`（指定后按该设备档位解算布局）。
## 栅格化模式传 `--wireframe-svg=`、`--wireframe-png=` 与 `--wireframe-scale=`。

const TEXT_CLASSES := ["Label", "Button", "RichTextLabel", "LineEdit", "CheckBox", "OptionButton"]
const SETTLE_FRAMES := 6


func _ready() -> void:
    var scene_path := ""
    var out_path := ""
    var device := ""
    var svg_path := ""
    var png_path := ""
    var scale := 1.0
    for arg in OS.get_cmdline_user_args():
        if arg.begins_with("--wireframe-scene="):
            scene_path = arg.trim_prefix("--wireframe-scene=")
        elif arg.begins_with("--wireframe-out="):
            out_path = arg.trim_prefix("--wireframe-out=")
        elif arg.begins_with("--wireframe-device="):
            device = arg.trim_prefix("--wireframe-device=")
        elif arg.begins_with("--wireframe-svg="):
            svg_path = arg.trim_prefix("--wireframe-svg=")
        elif arg.begins_with("--wireframe-png="):
            png_path = arg.trim_prefix("--wireframe-png=")
        elif arg.begins_with("--wireframe-scale="):
            scale = arg.trim_prefix("--wireframe-scale=").to_float()
    if svg_path != "":
        _rasterize(svg_path, png_path, scale)
        return
    if scene_path.is_empty() or out_path.is_empty():
        push_error("线框探针需要 --wireframe-scene 与 --wireframe-out")
        get_tree().quit(2)
        return
    if device != "":
        ScreenProfile.set_device_for_test(device)
        # 无头窗口不做内容缩放，手动切到该设备档位的基准画布。
        get_window().content_scale_size = ScreenProfile.base_canvas()
    var packed := load(scene_path) as PackedScene
    if packed == null:
        push_error("无法加载场景 %s" % scene_path)
        get_tree().quit(3)
        return
    var page := packed.instantiate()
    add_child(page)
    for _frame in SETTLE_FRAMES:
        await get_tree().process_frame
    var nodes: Array = []
    _collect(page, nodes, 0)
    var size := get_viewport().get_visible_rect().size
    var file := FileAccess.open(out_path, FileAccess.WRITE)
    if file == null:
        push_error("无法写入 %s（%s）" % [out_path, error_string(FileAccess.get_open_error())])
        get_tree().quit(4)
        return
    file.store_string(JSON.stringify({
        "size": [size.x, size.y],
        "device": ScreenProfile.device_kind(),
        "nodes": nodes,
    }))
    file.close()
    print("WIREFRAME_OK nodes=%d -> %s" % [nodes.size(), out_path])
    get_tree().quit(0)


## 栅格化：把 SVG 按 `scale` 画成 PNG。引擎自带的 ThorVG 不渲染文本，
## 没有浏览器可用时只作为兜底；正常路径由 `scripts/ui_wireframe.py` 调浏览器截图。
func _rasterize(source: String, png_path: String, scale: float) -> void:
    if png_path.is_empty() or scale <= 0.0:
        push_error("栅格化需要 --wireframe-png 与正的 --wireframe-scale")
        get_tree().quit(5)
        return
    var image := Image.new()
    var error := image.load_svg_from_string(FileAccess.get_file_as_string(source), scale)
    if error == OK:
        error = image.save_png(png_path)
    if error != OK:
        push_error("无法生成 %s（%s）" % [png_path, error_string(error)])
        get_tree().quit(6)
        return
    print("WIREFRAME_PNG_OK %dx%d -> %s" % [image.get_width(), image.get_height(), png_path])
    get_tree().quit(0)


func _collect(node: Node, out: Array, depth: int) -> void:
    if node is Control:
        var control := node as Control
        var rect := control.get_global_rect()
        if control.is_visible_in_tree() and rect.size.x > 0.0 and rect.size.y > 0.0:
            var entry := {
                "name": String(control.name),
                "class": control.get_class(),
                "rect": [rect.position.x, rect.position.y, rect.size.x, rect.size.y],
                "depth": depth,
            }
            for text_class in TEXT_CLASSES:
                if control.is_class(text_class):
                    entry["text"] = control.get("text")
                    break
            out.append(entry)
    for child in node.get_children():
        _collect(child, out, depth + 1)
