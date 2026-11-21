extends Node
## 运行态视图预览：真实窗口装载页面/场景，注入夹具，可选高亮子节点。
##
## 入口见 `make ui-preview`。不截图；默认保持窗口打开，`--preview-quit` 时自检后退出。
## 页面脚本实现 `static func preview(opts) -> Node` 时优先调用；组件预览通过
## `--preview-setup` 指定构建脚本。

const PreviewFixture = preload("res://tests/preview_fixture.gd")
const PreviewFocus = preload("res://tests/preview_focus.gd")

const SETTLE_FRAMES := 12
const WINDOW_TITLE := "Civilization View Preview"

var _quit_when_done := false
var _page: Node = null
var _focus_layer: Control = null


func _ready() -> void:
    var args := _parse_args(OS.get_cmdline_user_args())
    _quit_when_done = bool(args.get("quit", false))
    if DisplayServer.get_name() == "headless":
        _fail("headless")
        get_tree().quit(2)
        return
    _apply_device(str(args.get("device", "")))
    var fixture := str(args.get("fixture", ""))
    if fixture == "":
        fixture = PreviewFixture.default_for_page(str(args.get("page", "")))
    if not PreviewFixture.apply(fixture):
        _fail("fixture_unknown name=%s available=%s" % [
            fixture, ",".join(PreviewFixture.names()),
        ])
        _done(1)
        return
    print("VIEW_PREVIEW_FIXTURE name=%s" % fixture)
    var page := _build_page(args)
    if page == null:
        _done(1)
        return
    add_child(page)
    _page = page
    await _wait_frames(int(args.get("frames", SETTLE_FRAMES)))
    var focus_name_out := ""
    var focus_label := str(args.get("focus", ""))
    var focus_path := str(args.get("focus_path", ""))
    if focus_label != "" or focus_path != "":
        var focus_node := _find_focus(page, focus_label, focus_path)
        if focus_node == null:
            _fail("focus_not_found name=%s path=%s" % [focus_label, focus_path])
            _done(1)
            return
        if not (focus_node is Control):
            _fail("focus_not_control name=%s" % focus_node.name)
            _done(1)
            return
        var focus_control := focus_node as Control
        _add_focus(focus_control)
        focus_name_out = str(focus_control.name)
        print("VIEW_PREVIEW_FOCUS name=%s class=%s rect=%s visible=%s" % [
            focus_control.name,
            focus_control.get_class(),
            str(focus_control.get_global_rect()),
            str(focus_control.is_visible_in_tree()),
        ])
    var expects: Array = args.get("expect", [])
    if not _check_expects(page, expects):
        _done(1)
        return
    print("VIEW_PREVIEW_OK page=%s focus=%s" % [
        str(args.get("page", "")), focus_name_out,
    ])
    _done(0)


func _parse_args(raw: PackedStringArray) -> Dictionary:
    var args := {
        "page": "",
        "scene": "",
        "focus": "",
        "focus_path": "",
        "device": "",
        "fixture": "",
        "setup": "",
        "expect": [],
        "frames": SETTLE_FRAMES,
        "quit": false,
    }
    for item in raw:
        var text := str(item)
        var split := text.find("=")
        var key := text if split < 0 else text.substr(0, split)
        var value := "" if split < 0 else text.substr(split + 1)
        match key:
            "--preview-page":
                args["page"] = value
            "--preview-scene":
                args["scene"] = value
            "--preview-focus":
                args["focus"] = value
            "--preview-focus-path":
                args["focus_path"] = value
            "--preview-device":
                args["device"] = value
            "--preview-fixture":
                args["fixture"] = value
            "--preview-setup":
                args["setup"] = value
            "--preview-expect":
                args["expect"].append(value)
            "--preview-frames":
                args["frames"] = maxi(int(value), 1) if value.is_valid_int() else SETTLE_FRAMES
            "--preview-quit":
                args["quit"] = true
    return args


func _apply_device(kind: String) -> void:
    var device := ScreenProfile.parse_device(kind)
    if device != "":
        ScreenProfile.set_device_for_test(device)
    var canvas := ScreenProfile.base_canvas()
    var window := get_window()
    window.title = WINDOW_TITLE
    window.content_scale_size = canvas
    window.size = canvas
    print("VIEW_PREVIEW_WINDOW driver=%s device=%s window=%s content_scale=%s scale=%.2f" % [
        DisplayServer.get_name(),
        device if device != "" else "auto",
        str(window.size),
        str(window.content_scale_size),
        window.content_scale_factor,
    ])


func _build_page(args: Dictionary) -> Node:
    var setup := str(args.get("setup", ""))
    if setup != "":
        var setup_script = load(setup)
        if not _has_method(setup_script, "build"):
            _fail("setup_missing_build path=%s" % setup)
            return null
        var built = _call_static(setup_script, "build", args)
        if not (built is Node):
            _fail("setup_build_failed path=%s" % setup)
            return null
        print("VIEW_PREVIEW_SCENE setup=%s" % setup)
        return built
    var page_name := str(args.get("page", ""))
    var scene_path := str(args.get("scene", ""))
    if scene_path == "" and page_name != "":
        scene_path = "res://pages/%s/%s.tscn" % [page_name, page_name]
    if scene_path == "":
        _fail("usage page_or_scene_required")
        return null
    # 页面脚本带静态 preview 时优先用它；任意场景仍走场景实例化。
    if page_name != "":
        var script_path := "res://pages/%s/%s.gd" % [page_name, page_name]
        if ResourceLoader.exists(script_path):
            var script = load(script_path)
            if _has_method(script, "preview"):
                var previewed = _call_static(script, "preview", args)
                if previewed is Node:
                    print("VIEW_PREVIEW_SCENE page=%s scene=%s entry=preview" % [
                        page_name, scene_path,
                    ])
                    return previewed
    var packed = load(scene_path)
    if not (packed is PackedScene):
        _fail("scene_load_failed path=%s" % scene_path)
        return null
    var instance = (packed as PackedScene).instantiate()
    if instance == null:
        _fail("scene_instantiate_failed path=%s" % scene_path)
        return null
    print("VIEW_PREVIEW_SCENE page=%s scene=%s entry=instantiate" % [page_name, scene_path])
    return instance


## 静态方法经由脚本实例调用：GDScript 静态函数可被实例调用，`Object.call`
## 也能按名字分派，避免为每种视图写死分支。
func _has_method(script, method: String) -> bool:
    if not (script is Script):
        return false
    for info in script.get_script_method_list():
        if str(info.get("name", "")) == method:
            return true
    return false


func _call_static(script, method: String, opts: Dictionary):
    var probe = script.new()
    if probe == null:
        return null
    var result = probe.call(method, opts)
    if result != probe and probe is Node and not probe.is_inside_tree():
        probe.free()
    return result


func _wait_frames(count: int) -> void:
    for _i in range(maxi(count, 1)):
        await get_tree().process_frame


func _find_focus(page: Node, label: String, path: String) -> Node:
    if path != "":
        return page.get_node_or_null(NodePath(path))
    return page.find_child(label, true, false)


func _add_focus(target: Control) -> void:
    var layer := PreviewFocus.new()
    layer.target = target
    _focus_layer = layer
    add_child(layer)


func _check_expects(page: Node, specs: Array) -> bool:
    for spec in specs:
        var parts := str(spec).split("=", true, 1)
        if parts.size() != 2 or parts[0] == "" or not parts[1].is_valid_int():
            _fail("expect_bad spec=%s" % str(spec))
            return false
        var expected := int(parts[1])
        # 座位行按 playerId 命名（SeatRow_p0…），按子串统计即可数到全部行。
        var actual := 0
        for node in page.find_children("*", "", true, false):
            if str(node.name).contains(parts[0]):
                actual += 1
        print("VIEW_PREVIEW_EXPECT name=%s expected=%d actual=%d" % [
            parts[0], expected, actual,
        ])
        if actual != expected:
            _fail("expect_mismatch name=%s expected=%d actual=%d" % [
                parts[0], expected, actual,
            ])
            return false
    return true


func _fail(reason: String) -> void:
    print("VIEW_PREVIEW_FAIL reason=%s" % reason)


func _done(code: int) -> void:
    if not _quit_when_done:
        return
    # 立即释放预览树再退出，避免退出时的 RID / ObjectDB 泄漏告警。
    if is_instance_valid(_focus_layer):
        _focus_layer.free()
    if is_instance_valid(_page):
        _page.free()
    get_tree().quit(code)
