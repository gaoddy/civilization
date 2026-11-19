extends Node
## 安全区冒烟：桌面端包裹后布局不变，触控端边距与命中高度符合约定。
##
## 桌面页面逐个实例化，断言包裹层四边为 0、原直接子节点仍可按名定位；
## 触控边距用 `apply_for_test` 注入，不依赖真实设备的安全区与键盘。

var _failures := 0

const PAGES := [
    {
        "path": "res://scenes/login.tscn",
        "children": 2,
        "direct_names": [],
        "deep_names": [],
    },
    {
        "path": "res://scenes/lobby.tscn",
        "children": 3,
        "direct_names": ["InvitePopup"],
        "deep_names": ["GalleryButton", "FriendCodeInput"],
    },
    {
        "path": "res://scenes/room.tscn",
        "children": 4,
        "direct_names": ["SkillDismissLayer", "DraftPanel", "SkillLayer"],
        "deep_names": ["ConfirmButton", "OtherPlayers", "HeroCandidates"],
    },
    {
        "path": "res://scenes/game.tscn",
        "children": 6,
        "direct_names": ["MinionDetailLayer", "ReportLayer", "ChooserOverlay", "FirstGameGuide"],
        "deep_names": ["CenterPanel", "StatusLabel", "PlayersPanel"],
    },
    {
        "path": "res://scenes/result.tscn",
        "children": 2,
        "direct_names": [],
        "deep_names": ["ResultScroll", "ConfirmButton", "ResultActions"],
    },
    {
        "path": "res://scenes/gallery.tscn",
        "children": 1,
        "direct_names": ["CardGallery"],
        "deep_names": ["GalleryHeader", "GalleryClose"],
    },
]


func _ready() -> void:
    await _check_desktop()
    await _check_touch()
    await _check_compact()
    ScreenProfile.set_device_for_test("")
    if _failures > 0:
        get_tree().quit(1)
        return
    print("UI_SMOKE_OK")
    get_tree().quit(0)


func _check_desktop() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_DESKTOP)
    _check(UITheme.hit_primary() == 44, "桌面主操作命中高度应保持 44")
    _check(UITheme.hit_small() == 32, "桌面次要操作命中高度应保持 32")

    for page_case in PAGES:
        var page := await _instantiate(str(page_case["path"]))
        if page == null:
            continue
        var area := _safe_area(page)
        _check(area != null, "%s 缺少 SafeAreaMargin" % page_case["path"])
        if area != null:
            _check(page.get_child_count() == 1, "%s 根节点应只剩 SafeAreaMargin" % page_case["path"])
            _check(area.get_child_count() == int(page_case["children"]),
                "%s 原直接子节点数量应保持 %d" % [page_case["path"], int(page_case["children"])])
            _check(_margins(area) == Vector4i.ZERO, "%s 桌面安全区边距应为 0" % page_case["path"])
            for node_name in page_case["direct_names"]:
                var node := page.find_child(str(node_name), true, false)
                _check(node != null and node.get_parent() == area,
                    "%s 的 %s 应仍在安全区内" % [page_case["path"], node_name])
            for node_name in page_case["deep_names"]:
                _check(page.find_child(str(node_name), true, false) != null,
                    "%s 应能按名找到 %s" % [page_case["path"], node_name])
            var again := UI.safe_area(page)
            _check(again == area, "%s 重复调用应返回同一个 SafeAreaMargin" % page_case["path"])
            _check(_count_safe_areas(page) == 1,
                "%s 重复调用后应只有一个 SafeAreaMargin" % page_case["path"])
        page.queue_free()
        await get_tree().process_frame


func _check_touch() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    # 无头环境没有真实窗口尺寸，`hit_min` 按 44/32 原值返回；
    # 真机换算值由 ui_smoke_platform 用固定窗口断言。
    _check(UITheme.hit_primary() >= 44,
        "触控主操作命中高度不应低于 44，实际 %d" % UITheme.hit_primary())
    _check(UITheme.hit_small() >= 32,
        "触控次要操作命中高度不应低于 32，实际 %d" % UITheme.hit_small())

    var page := Control.new()
    page.name = "SafeAreaFixture"
    add_child(page)
    var area := UI.safe_area(page)
    _check(area != null, "触控夹具应创建 SafeAreaMargin")
    if area == null:
        page.queue_free()
        return
    area.apply_for_test(Vector4i(6, 4, 8, 10), 20)
    _check(_margins(area) == Vector4i(6, 4, 8, 20),
        "键盘展开时底部应取键盘高度 20，实际 %s" % str(_margins(area)))
    area.apply_for_test(Vector4i(6, 4, 8, 10), 0)
    _check(_margins(area) == Vector4i(6, 4, 8, 10),
        "键盘收起后底部应回到安全区 10，实际 %s" % str(_margins(area)))
    _check(UI.safe_area(page) == area, "触控夹具重复调用应返回同一个 SafeAreaMargin")
    _check(_count_safe_areas(page) == 1, "触控夹具重复调用后应只有一个 SafeAreaMargin")
    page.queue_free()
    await get_tree().process_frame


## iPhone 基准 960×540：六个页面在此尺寸下实例化，关键节点不得越界。
func _check_compact() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    var compact := Vector2(960, 540)
    for page_case in PAGES:
        var host := Control.new()
        host.name = "CompactHost"
        host.size = compact
        add_child(host)
        var page := await _instantiate_into(str(page_case["path"]), host)
        if page != null:
            _check(page.size.is_equal_approx(compact),
                "%s 在 960×540 下应铺满宿主，实际 %s" % [page_case["path"], str(page.size)])
            for node_name in page_case["deep_names"]:
                var node := page.find_child(str(node_name), true, false)
                _check(node is Control, "%s 的 %s 应为 Control" % [page_case["path"], node_name])
                if node is Control:
                    var rect: Rect2 = (node as Control).get_global_rect()
                    var bounds := Rect2(host.get_global_rect().position, compact)
                    _check(bounds.grow(1.0).encloses(rect),
                        "%s 的 %s 越界：%s 不在 %s 内" % [page_case["path"], node_name, str(rect), str(bounds)])
        host.queue_free()
        await get_tree().process_frame


func _instantiate_into(path: String, host: Control) -> Node:
    var packed = load(path)
    if packed == null:
        _check(false, "无法加载场景 %s" % path)
        return null
    var instance = packed.instantiate()
    if instance == null:
        _check(false, "无法实例化场景 %s" % path)
        return null
    host.add_child(instance)
    await get_tree().process_frame
    await get_tree().process_frame
    return instance


func _instantiate(path: String) -> Node:
    var packed = load(path)
    if packed == null:
        _check(false, "无法加载场景 %s" % path)
        return null
    var instance = packed.instantiate()
    if instance == null:
        _check(false, "无法实例化场景 %s" % path)
        return null
    add_child(instance)
    await get_tree().process_frame
    await get_tree().process_frame
    return instance


func _safe_area(page: Node) -> SafeAreaMargin:
    for child in page.get_children():
        if child is SafeAreaMargin:
            return child
    return null


func _count_safe_areas(page: Node) -> int:
    var count := 0
    for child in page.get_children():
        if child is SafeAreaMargin:
            count += 1
    return count


func _margins(area: MarginContainer) -> Vector4i:
    return Vector4i(
        area.get_theme_constant("margin_left"),
        area.get_theme_constant("margin_top"),
        area.get_theme_constant("margin_right"),
        area.get_theme_constant("margin_bottom"),
    )


func _check(ok: bool, message: String) -> void:
    if ok:
        return
    _failures += 1
    push_error("LAYOUT_SMOKE_FAIL: " + message)
