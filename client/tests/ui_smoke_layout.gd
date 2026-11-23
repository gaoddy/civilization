extends Node
## 安全区冒烟：桌面端包裹后布局不变，触控端边距与命中高度符合约定。
##
## 桌面页面逐个实例化，断言包裹层四边为 0、原直接子节点仍可按名定位；
## 触控边距用 `apply_for_test` 注入，不依赖真实设备的安全区与键盘。

var _failures := 0

const PAGES := [
    {
        "path": "res://pages/login/login.tscn",
        "children": 1,
        "direct_names": [],
        "deep_names": [],
    },
    {
        "path": "res://pages/lobby/lobby.tscn",
        "children": 4,
        "direct_names": ["InvitePopup", "FriendsPanel", "QueuePanel"],
        "deep_names": ["GalleryButton", "FriendsButton", "LeaderboardButton"],
    },
    {
        "path": "res://pages/room/room.tscn",
        "children": 2,
        "direct_names": ["DraftPanel", "HeroDetailLayer"],
        "deep_names": ["ConfirmButton", "OtherPlayers", "HeroCandidates"],
    },
    {
        "path": "res://pages/game/game.tscn",
        "children": 8,
        "direct_names": [
            "AimLayer",
            "MinionDetailLayer",
            "ChooserOverlay",
            "FirstGameGuide",
            "GameToolsOverlay",
            "StatusModal",
            "CombatHeroClash",
        ],
        "deep_names": ["CenterPanel", "GameTools", "PlayersPanel", "BodyRow"],
    },
    {
        "path": "res://pages/result/result.tscn",
        "children": 1,
        "direct_names": [],
        "deep_names": ["ResultScroll", "ConfirmButton", "ResultActions"],
    },
    {
        "path": "res://pages/gallery/gallery.tscn",
        "children": 1,
        "direct_names": ["CardGallery"],
        "deep_names": ["GalleryHeader", "GalleryClose"],
    },
    {
        "path": "res://pages/leaderboard/leaderboard.tscn",
        "children": 1,
        "direct_names": [],
        "deep_names": ["LeaderboardScroll", "LeaderboardList", "BackButton"],
    },
    {
        "path": "res://pages/profile/profile.tscn",
        "children": 1,
        "direct_names": [],
        "deep_names": ["ProfileScroll", "ProfileContent", "BackButton"],
    },
]


func _ready() -> void:
    await _check_early_safe_area()
    _check_control_defaults()
    await _check_hero_cover_art()
    await _check_desktop()
    await _check_drag_preview()
    await _check_aim_local()
    await _check_detail_local_composed()
    await _check_touch()
    await _check_compact()
    await _check_tall()
    ScreenProfile.set_device_for_test("")
    if _failures > 0:
        get_tree().quit(1)
        return
    print("UI_SMOKE_OK")
    get_tree().quit(0)


## 页面在 `_ready()` 第一句调用 `safe_area` 时还没有子节点：
## 之后加入的直接子节点必须自动收进安全区，背景仍置底留在框外，顺序不变。
func _check_early_safe_area() -> void:
    var page := Control.new()
    page.name = "EarlySafeAreaPage"
    page.size = Vector2(1280, 720)
    add_child(page)
    var area := UILayout.safe_area(page)
    _check(area != null, "先调用 safe_area 应创建安全区")
    var background := UIControls.background()
    page.add_child(background)
    var first := Control.new()
    first.name = "FirstContent"
    page.add_child(first)
    var second := Control.new()
    second.name = "SecondContent"
    page.add_child(second)
    await get_tree().process_frame
    await get_tree().process_frame
    _check(page.get_child_count() == 2,
        "先调用 safe_area 后加入的内容应收进安全区，根节点实际 %d 个" % page.get_child_count())
    _check(background.get_parent() == page and page.get_child(0) == background,
        "后加入的背景应置底留在框外")
    _check(first.get_parent() == area and second.get_parent() == area,
        "后加入的内容应在安全区内")
    _check(first.get_index() < second.get_index(), "安全区应保持加入顺序")
    var body := page.find_child(UILayout.PAGE_BODY_NAME, true, false) as Control
    _check(body != null and body.get_parent() != page,
        "先调用 safe_area 应创建设计画布内的 PageBody")
    if body != null:
        _check(body.find_child(UILayout.PAGE_SURFACE_NAME, false, false) != null,
            "PageBody 应包含 PageSurface")
    page.queue_free()
    await get_tree().process_frame


## 工厂默认值必须落在令牌上：标签默认正文字号，边距默认大号间距。
func _check_control_defaults() -> void:
    var default_label := UIControls.label("默认字号")
    _check(default_label.get_theme_font_size("font_size") == UITheme.font_size("font-body"),
        "标签默认字号应为 font-body 令牌，实际 %d"
        % default_label.get_theme_font_size("font_size"))
    default_label.free()
    var default_margin := UIControls.margin(Control.new())
    _check(default_margin.get_theme_constant("margin_left") == UITheme.space("space-lg"),
        "边距默认宽度应为 space-lg 令牌，实际 %d"
        % default_margin.get_theme_constant("margin_left"))
    default_margin.free()


## 对局页英雄卡封面：方形头像铺满整张方形卡，手机档位 64×64。
func _check_hero_cover_art() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    var card := GameHeroCard.new()
    add_child(card)
    card.refresh("darwin", 30, 40)
    await get_tree().process_frame
    _check(card.custom_minimum_size == Vector2(64, 64),
        "手机英雄卡应为 64×64，实际 %s" % str(card.custom_minimum_size))
    var art := card.find_child("AvatarFrameArt", true, false) as TextureRect
    _check(art != null, "英雄卡封面应加载头像素材")
    if art != null:
        _check(art.stretch_mode == TextureRect.STRETCH_KEEP_ASPECT_COVERED,
            "英雄卡封面应裁切铺满，当前拉伸模式 %d" % art.stretch_mode)
        _check(art.size.x >= card.size.x - 4.0 and art.size.y >= card.size.y - 4.0,
            "英雄卡封面应铺满整卡，封面 %s，卡 %s" % [str(art.size), str(card.size)])
    card.queue_free()
    await get_tree().process_frame


func _check_desktop() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_DESKTOP)
    _check(UITheme.hit_primary() == 44, "桌面主操作命中高度应保持 44")
    _check(UITheme.hit_small() == 32, "桌面次要操作命中高度应保持 32")
    var input := UIControls.line_edit("房间码", 220)
    _check(input is LineEdit, "输入框工厂应返回 LineEdit")
    _check(input.custom_minimum_size == Vector2(220, 34),
        "桌面输入框应保留 34 高与指定宽度，实际 %s" % str(input.custom_minimum_size))
    input.free()

    for page_case in PAGES:
        var page := await _instantiate(str(page_case["path"]))
        if page == null:
            continue
        var area := _safe_area(page)
        _check(area != null, "%s 缺少 SafeAreaMargin" % page_case["path"])
        if area != null:
            var frame := page.get_node_or_null(UILayout.CONTENT_FRAME_NAME) as Control
            _check(frame != null and frame.get_parent() == page, "%s 应有 ContentFrame" % page_case["path"])
            var root: Control = null
            if frame != null:
                root = frame.get_node_or_null(UILayout.CONTENT_ROOT_NAME) as Control
            _check(root != null, "%s 的 ContentFrame 应包含 ContentRoot" % page_case["path"])
            if root != null:
                _check(root.size.is_equal_approx(Vector2(ScreenProfile.base_canvas())),
                    "%s 的 ContentRoot 应为基准尺寸" % page_case["path"])
                var ratio := root.size.x / root.size.y
                _check(is_equal_approx(ratio, 16.0 / 9.0), "%s 的 ContentRoot 应保持基准比例" % page_case["path"])
                var body := root.get_node_or_null(UILayout.PAGE_BODY_NAME) as Control
                _check(body != null, "%s 的 ContentRoot 应包含 PageBody" % page_case["path"])
                if body != null:
                    _check(body.get_parent() == root,
                        "%s 的 PageBody 应直接挂在 ContentRoot 下" % page_case["path"])
                    _check(body.size.is_equal_approx(Vector2(ScreenProfile.base_canvas())),
                        "%s 的 PageBody 应为设计画布尺寸，实际 %s" % [page_case["path"], str(body.size)])
                    var surface := body.get_node_or_null(UILayout.PAGE_SURFACE_NAME) as Control
                    _check(surface != null and surface.get_parent() == body,
                        "%s 的 PageBody 应包含 PageSurface" % page_case["path"])
                    if surface != null:
                        _check(surface.mouse_filter == Control.MOUSE_FILTER_IGNORE,
                            "%s 的 PageSurface 应放行输入" % page_case["path"])
                        _check(surface.size.is_equal_approx(Vector2(ScreenProfile.base_canvas())),
                            "%s 的 PageSurface 应铺满页面容器" % page_case["path"])
                        if str(page_case["path"]).ends_with("game.tscn"):
                            _check(surface is TextureRect,
                                "%s 的对局页 PageSurface 应为纹理图" % page_case["path"])
                            if surface is TextureRect:
                                var texture := (surface as TextureRect).texture
                                _check(texture != null and str(texture.resource_path).ends_with("bg_game_desktop.png"),
                                    "%s 的桌面档位应使用 bg_game_desktop.png" % page_case["path"])
                        else:
                            _check(surface is Panel,
                                "%s 的非对局页 PageSurface 应为语义面板" % page_case["path"])
                _check(area.get_parent() == body,
                    "%s 的 SafeAreaMargin 应挂在 PageBody 下" % page_case["path"])
            _check(page.get_child_count() == 2, "%s 根节点应为背景 + ContentFrame" % page_case["path"])
            var background := page.get_node_or_null("PageBackground")
            _check(background != null and background.get_parent() == page,
                "%s 背景应留在安全区外" % page_case["path"])
            _check(page.get_child_count() > 0 and page.get_child(0) == background,
                "%s 背景应置底、排在 ContentFrame 之前" % page_case["path"])
            _check(area.get_child_count() == int(page_case["children"]),
                "%s 安全区内直接子节点数量应为 %d" % [page_case["path"], int(page_case["children"])])
            _check(_margins(area) == Vector4i.ZERO, "%s 桌面安全区边距应为 0" % page_case["path"])
            for node_name in page_case["direct_names"]:
                var node := page.find_child(str(node_name), true, false)
                _check(node != null and node.get_parent() == area,
                    "%s 的 %s 应仍在安全区内" % [page_case["path"], node_name])
            for node_name in page_case["deep_names"]:
                _check(page.find_child(str(node_name), true, false) != null,
                    "%s 应能按名找到 %s" % [page_case["path"], node_name])
            var again := UILayout.safe_area(page)
            _check(again == area, "%s 重复调用应返回同一个 SafeAreaMargin" % page_case["path"])
            _check(_count_safe_areas(page) == 1,
                "%s 重复调用后应只有一个 SafeAreaMargin" % page_case["path"])
            _check(_count_named(page, UILayout.PAGE_BODY_NAME) == 1,
                "%s 重复调用后应只有一个 PageBody" % page_case["path"])
            _check(_count_named(page, UILayout.PAGE_SURFACE_NAME) == 1,
                "%s 重复调用后应只有一个 PageSurface" % page_case["path"])
        page.queue_free()
        await get_tree().process_frame


## 拖拽预览在缩放内容框下仍与卡片同尺寸、起手点对齐。
func _check_drag_preview() -> void:
    var page := Control.new()
    page.size = Vector2(1280, 480)
    add_child(page)
    var area := UILayout.safe_area(page)
    var card := Control.new()
    card.size = Vector2(84, 92)
    area.add_child(card)
    var scale := UILayout.root_scale(card)
    area.remove_child(card)
    var preview := UIDrag.preview(card, Vector2(10, 20), area)
    _check(is_equal_approx(preview.scale.x, scale), "拖拽预览应按内容框缩放")
    _check(card.position.is_equal_approx(Vector2(-10, -20)), "拖拽预览起手点应对齐")
    preview.free()
    page.queue_free()
    await get_tree().process_frame


## 超宽视口下瞄准层把全局矩形换算到内容框局部坐标。
func _check_aim_local() -> void:
    var host := Control.new()
    host.name = "AimHost"
    host.size = Vector2(1280, 480)
    add_child(host)
    var page := await _instantiate_into("res://pages/game/game.tscn", host)
    if page != null:
        var aim := page.find_child("AimLayer", true, false) as Control
        _check(aim != null, "对局页缺少 AimLayer")
        if aim != null:
            var global := Rect2(400, 100, 80, 80)
            aim.begin_skill(global, [{"zone": "board", "id": "none"}])
            var local: Rect2 = aim.source_rect_local()
            _check(local.position.is_equal_approx(aim.get_global_transform().affine_inverse() * global.position),
                "瞄准层源矩形应转为局部坐标")
            _check(is_equal_approx(local.size.x, global.size.x / aim.get_global_transform().get_scale().x),
                "瞄准层源矩形尺寸应除以内容框缩放")
            _check(Rect2(Vector2.ZERO, aim.size).encloses(local), "瞄准层源矩形应落在层内")
            aim.clear()
    host.queue_free()
    await get_tree().process_frame


## 内容框原点不为零且整体缩放时，详情层仍把全局矩形换算到自身局部坐标。
## 只减 `global_position`、忽略内容框缩放的做法会在这里失败。
func _check_detail_local_composed() -> void:
    var host := Control.new()
    host.name = "DetailComposedHost"
    host.size = Vector2(1280, 480)
    add_child(host)
    var area := UILayout.safe_area(host)
    var frame := host.get_node(UILayout.CONTENT_FRAME_NAME) as Control
    var root := frame.get_node(UILayout.CONTENT_ROOT_NAME) as Control
    _check(frame.position.x > 0.0, "组合用例应有非零内容框原点，实际 %s" % str(frame.position))
    _check(not is_equal_approx(root.scale.x, 1.0),
        "组合用例应有非 1 的内容框缩放，实际 %s" % str(root.scale))
    var layer := DetailLayer.new()
    area.add_child(layer)
    layer.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    var anchor := Control.new()
    anchor.position = Vector2(400, 300)
    anchor.size = Vector2(84, 92)
    area.add_child(anchor)
    await get_tree().process_frame
    var global := anchor.get_global_rect()
    var local: Rect2 = layer._local_rect(global)
    var expected := layer.get_global_transform().affine_inverse() * global.position
    _check(local.position.is_equal_approx(expected),
        "详情层源矩形应按整体变换换算，实际 %s 期望 %s" % [str(local.position), str(expected)])
    var layer_scale := layer.get_global_transform().get_scale().x
    _check(is_equal_approx(local.size.x, global.size.x / layer_scale),
        "详情层源矩形尺寸应除以内容框缩放")
    host.queue_free()
    await get_tree().process_frame


func _check_touch() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    # 无头环境没有真实窗口尺寸，`hit_min` 按 44/32 原值返回；
    # 真机换算值由 ui_smoke_platform 用固定窗口断言。
    _check(UITheme.hit_primary() >= 44,
        "触控主操作命中高度不应低于 44，实际 %d" % UITheme.hit_primary())
    _check(UITheme.hit_small() >= 32,
        "触控次要操作命中高度不应低于 32，实际 %d" % UITheme.hit_small())
    var input := UIControls.line_edit("", 220)
    _check(input is LineEdit, "触控输入框工厂应返回 LineEdit")
    _check(input.custom_minimum_size.y >= UITheme.hit_primary(),
        "触控输入框最小高度应不低于主操作命中高度，实际 %d" % int(input.custom_minimum_size.y))
    input.free()

    var page := Control.new()
    page.name = "SafeAreaFixture"
    add_child(page)
    var area := UILayout.safe_area(page)
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
    _check(UILayout.safe_area(page) == area, "触控夹具重复调用应返回同一个 SafeAreaMargin")
    _check(_count_safe_areas(page) == 1, "触控夹具重复调用后应只有一个 SafeAreaMargin")
    await get_tree().process_frame
    var body := page.find_child(UILayout.PAGE_BODY_NAME, true, false) as Control
    _check(body != null and body.size.is_equal_approx(Vector2(ScreenProfile.base_canvas())),
        "安全区边距变化不应改变 PageBody 的设计画布尺寸，实际 %s"
            % (str(body.size) if body != null else "无 PageBody"))
    page.queue_free()
    await get_tree().process_frame


## 手机卡面必须保持桌面 112:146 的卡形，只缩尺寸不改变比例。
func _check_card_ratio() -> void:
    var width := UITheme.stage_metric("card-width")
    var height := UITheme.stage_metric("card-height")
    _check(
        absi(width * 146 - height * 112) <= 60,
        "手机卡面应保持 112:146 比例：%d×%d" % [width, height],
    )


func _check_compact() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    _check_card_ratio()
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
            if str(page_case["path"]).ends_with("game.tscn"):
                var surface := page.find_child(UILayout.PAGE_SURFACE_NAME, true, false) as Control
                _check(surface is TextureRect, "手机档位对局页 PageSurface 应为纹理图")
                if surface is TextureRect:
                    var texture := (surface as TextureRect).texture
                    _check(texture != null and str(texture.resource_path).ends_with("bg_game_phone.png"),
                        "手机档位对局页应使用 bg_game_phone.png")
            for node_name in page_case["deep_names"]:
                var node := page.find_child(str(node_name), true, false)
                _check(node is Control, "%s 的 %s 应为 Control" % [page_case["path"], node_name])
                if node is Control:
                    var rect: Rect2 = (node as Control).get_global_rect()
                    var bounds := Rect2(host.get_global_rect().position, compact)
                    _check(bounds.grow(1.0).encloses(rect),
                        "%s 的 %s 越界：%s 不在 %s 内" % [page_case["path"], node_name, str(rect), str(bounds)])
            if str(page_case["path"]).ends_with("lobby.tscn"):
                _check_lobby_inputs(page, host, compact)
        host.queue_free()
        await get_tree().process_frame
    await _check_game_compact(compact)
    await _check_gallery_hero_compact(compact)


## 手机基准下大厅输入框：命中高度按触控换算，且不越出 960×540 宿主。
## 输入框工厂把触控端最小高度抬到 `hit_primary()`，这里防止大厅面板被撑高后溢出。
func _check_lobby_inputs(page: Node, host: Control, compact: Vector2) -> void:
    var bounds := Rect2(host.get_global_rect().position, compact).grow(1.0)
    var visible := 0
    for child in page.find_children("*", "LineEdit", true, false):
        var input := child as Control
        if input == null or not input.is_visible_in_tree():
            continue
        visible += 1
        _check(bounds.encloses(input.get_global_rect()),
            "大厅输入框越界：%s 不在 %s 内" % [str(input.get_global_rect()), str(bounds)])
        _check(input.size.y >= UITheme.hit_primary(),
            "大厅输入框高度应不低于命中高度：%.1f < %d"
            % [input.size.y, UITheme.hit_primary()])
    _check(visible > 0, "大厅在 960×540 下应有可见输入框")


## 图鉴英雄视图在 iPhone 基准下：隐藏时代与类型筛选，玩法筛选、网格与滚动区不越界。
func _check_gallery_hero_compact(compact: Vector2) -> void:
    var host := Control.new()
    host.name = "CompactGalleryHost"
    host.size = compact
    add_child(host)
    var page := await _instantiate_into("res://pages/gallery/gallery.tscn", host)
    if page != null:
        var mode_hero := page.find_child("ModeHero", true, false) as Button
        _check(mode_hero != null and not mode_hero.disabled, "英雄视图切换应可用")
        if mode_hero != null:
            mode_hero.pressed.emit()
            await get_tree().process_frame
            await get_tree().process_frame
            var bounds := Rect2(host.get_global_rect().position, compact).grow(1.0)
            for node_name in [
                "GalleryHeader",
                "GalleryPlaySelect",
                "GalleryScroll",
            ]:
                var node := page.find_child(str(node_name), true, false) as Control
                _check(
                    node != null and node.is_visible_in_tree(),
                    "手机英雄视图缺少可见的 %s" % node_name,
                )
                if node != null:
                    _check(
                        bounds.encloses(node.get_global_rect()),
                        "%s 在英雄视图越界：%s 不在 %s 内"
                        % [node_name, str(node.get_global_rect()), str(bounds)],
                    )
            var era_select := page.find_child("GalleryEraSelect", true, false) as Control
            _check(
                era_select == null or not era_select.is_visible_in_tree(),
                "英雄视图不应显示时代筛选",
            )
            _check(
                page.find_child("GalleryToc", true, false) == null,
                "手机英雄视图不应再有左侧目录",
            )
            var header := page.find_child("GalleryHeader", true, false) as Control
            var bar := page.find_child("GalleryPlaySelect", true, false) as Control
            _check(
                header != null and bar != null
                    and header.get_global_rect().encloses(bar.get_global_rect()),
                "玩法筛选应位于头部内",
            )
    host.queue_free()
    await get_tree().process_frame


## 对局页有完整八人视图时：积分榜 8 席不滚动、棋盘与手牌不越界、
## 棋盘与商店卡不显示介绍而手牌保留。
func _check_game_compact(compact: Vector2) -> void:
    var saved_view: Dictionary = GameStore.view
    var saved_turn := GameStore.turn
    var saved_phase := GameStore.phase
    var saved_player := GameStore.player_id
    GameStore.player_id = "p0"
    GameStore.turn = 3
    GameStore.phase = Protocol.PHASE_RECRUIT
    GameStore.view = Fixture.game_view()

    var host := Control.new()
    host.name = "CompactGameHost"
    host.size = compact
    add_child(host)
    var page := await _instantiate_into("res://pages/game/game.tscn", host)
    if page != null:
        # 横屏 Home 指示条约 21pt，按内容框换算约 28 画布单位；真机左栏可视高度因此更矮。
        var area := _safe_area(page)
        if area != null:
            area.apply_for_test(Vector4i(0, 0, 0, 28), 0)
            await get_tree().process_frame
            await get_tree().process_frame
        var rail := page.find_child("PlayersRail", true, false) as Control
        if rail != null:
            _check(
                not (rail.get_parent() is ScrollContainer),
                "手机基准左栏座位列不应使用滚动容器",
            )
            var seat_rows: Array = []
            for child in rail.get_children():
                if child.has_meta("player_id"):
                    seat_rows.append(child as Control)
            _check(seat_rows.size() == 8, "手机基准积分榜应有 8 个座位行，实际 %d" % seat_rows.size())
            if seat_rows.size() == 8:
                # 8 个正方形座位行按同一间距排列并落在左栏内，不再均分拉伸行高。
                var rail_rect := rail.get_global_rect()
                var first_rect: Rect2 = seat_rows[0].get_global_rect()
                var min_side := first_rect.size.x
                var max_side := first_rect.size.x
                for row in seat_rows:
                    var rect: Rect2 = row.get_global_rect()
                    _check(
                        rail_rect.grow(1.0).encloses(rect),
                        "手机基准座位行应落在左栏内：行 %s，左栏 %s" % [str(rect), str(rail_rect)],
                    )
                    _check(
                        is_equal_approx(rect.size.x, rect.size.y),
                        "手机基准座位行应为正方形：%s" % str(rect.size),
                    )
                    min_side = minf(min_side, rect.size.x)
                    max_side = maxf(max_side, rect.size.x)
                _check(
                    max_side - min_side <= 1.0,
                    "手机基准 8 个座位行应等边长：最大 %.1f，最小 %.1f"
                    % [max_side, min_side],
                )
                var avatar_box := seat_rows[0].find_child("SeatAvatar", true, false) as Control
                var fallback_side := float(UITheme.stage_metric("avatar-rail"))
                _check(
                    avatar_box != null and avatar_box.size.x > fallback_side,
                    "手机基准头像应随行高放大，实际 %s 不大于兜底 %.0f"
                    % [str(avatar_box.size) if avatar_box != null else "缺失", fallback_side],
                )
                _check(
                    avatar_box != null and is_equal_approx(avatar_box.size.x, avatar_box.size.y),
                    "手机基准头像应为正方形",
                )
        var bounds := Rect2(host.get_global_rect().position, compact).grow(1.0)
        for node_name in ["BoardRow", "HandStrip", "RecruitGoldTile"]:
            var node := page.find_child(str(node_name), true, false) as Control
            _check(node != null, "对局页缺少 %s" % node_name)
            if node != null:
                _check(bounds.encloses(node.get_global_rect()),
                    "%s 越界：%s 不在 %s 内" % [node_name, str(node.get_global_rect()), str(bounds)])
        var hand_strip := page.find_child("HandStrip", true, false) as Control
        var hand_row := page.find_child("HandRow", true, false) as Control
        if hand_strip != null and hand_row != null:
            _check(
                hand_row.get_global_rect().end.y >= hand_strip.get_global_rect().end.y - 1.0,
                "手牌卡应贴住手牌行底部：卡底 %.1f，行底 %.1f"
                % [hand_row.get_global_rect().end.y, hand_strip.get_global_rect().end.y],
            )
            _check(
                hand_strip.get_global_rect().end.y >= compact.y - 28.0 - 8.0,
                "手牌行应贴住页面底部，实际底边 %.1f" % hand_strip.get_global_rect().end.y,
            )
        _check_empty_desc(page.find_child("BoardRow", true, false), "棋盘卡")
        _check_empty_desc(page.find_child("ShopRow", true, false), "商店卡")
        var hand_card := hand_row.get_child(0) if hand_row != null and hand_row.get_child_count() > 0 else null
        _check(
            hand_card != null and hand_card.find_child("CardDesc", true, false) != null,
            "手牌卡应保留介绍",
        )
        # 手机只放大棋盘与商店的随从卡；手牌与英雄卡保持原尺寸。
        var minion_size := Vector2(
            UITheme.stage_metric("minion-card-width"),
            UITheme.stage_metric("minion-card-height"),
        )
        var shop_slot := page.find_child("ShopSlot0", true, false) as Control
        _check(
            shop_slot != null and shop_slot.size.is_equal_approx(minion_size),
            "手机商店卡应为 %s，实际 %s"
            % [str(minion_size), str(shop_slot.size) if shop_slot != null else "缺失"],
        )
        var board_minion := page.find_child("BoardMinion0", true, false) as Control
        _check(
            board_minion != null and board_minion.size.is_equal_approx(minion_size),
            "手机棋盘卡应为 %s，实际 %s"
            % [str(minion_size), str(board_minion.size) if board_minion != null else "缺失"],
        )
        var card_size := Vector2(
            UITheme.stage_metric("card-width"), UITheme.stage_metric("card-height")
        )
        var hand_card_node := page.find_child("HandCard0", true, false) as Control
        _check(
            hand_card_node != null and hand_card_node.size.is_equal_approx(card_size),
            "手机手牌卡应保持 %s，实际 %s"
            % [str(card_size), str(hand_card_node.size) if hand_card_node != null else "缺失"],
        )
        var hero_side := page.find_child("HeroSide", true, false) as Control
        var hero_size := float(UITheme.stage_metric("hero-card-size"))
        _check(
            hero_side != null and is_equal_approx(hero_side.size.x, hero_size),
            "手机英雄卡不应放大：应为 %.0f，实际 %.0f"
            % [hero_size, hero_side.size.x if hero_side != null else -1.0],
        )
        # 舞台整组贴底，富余高度留在顶部，河道不再贴住页面顶边。
        var center_panel := page.find_child("CenterPanel", true, false) as Control
        var recruit_panel := page.find_child("RecruitPanel", true, false) as Control
        _check(
            center_panel != null and recruit_panel != null
                and recruit_panel.get_global_rect().position.y
                    >= center_panel.get_global_rect().position.y + 1.0,
            "手机舞台应把富余高度留在顶部，实际组合顶 %.1f，舞台顶 %.1f"
            % [
                recruit_panel.get_global_rect().position.y if recruit_panel != null else -1.0,
                center_panel.get_global_rect().position.y if center_panel != null else -1.0,
            ],
        )
    host.queue_free()
    GameStore.view = saved_view
    GameStore.turn = saved_turn
    GameStore.phase = saved_phase
    GameStore.player_id = saved_player
    await get_tree().process_frame


## 非 16:9 的逻辑高度：16:10 桌面 1280×800 与 iPad 4:3 1280×960 下
## 六个页面铺满宿主，关键节点不越界；对局页手牌仍贴底。
func _check_tall() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_DESKTOP)
    for size in [Vector2(1280, 800), Vector2(1280, 960)]:
        for page_case in PAGES:
            var host := Control.new()
            host.name = "TallHost"
            host.size = size
            add_child(host)
            var page := await _instantiate_into(str(page_case["path"]), host)
            if page != null:
                _check(
                    page.size.is_equal_approx(size),
                    "%s 在 %s 下应铺满宿主，实际 %s"
                    % [page_case["path"], str(size), str(page.size)],
                )
                var bounds := Rect2(host.get_global_rect().position, size).grow(1.0)
                for node_name in page_case["deep_names"]:
                    var node := page.find_child(str(node_name), true, false)
                    _check(node is Control, "%s 的 %s 应为 Control" % [page_case["path"], node_name])
                    if node is Control:
                        _check(
                            bounds.encloses((node as Control).get_global_rect()),
                            "%s 的 %s 在 %s 下越界：%s"
                            % [page_case["path"], node_name, str(size), str((node as Control).get_global_rect())],
                        )
            host.queue_free()
            await get_tree().process_frame
    await _check_game_tall(Vector2(1280, 800))


## 16:10 高度下对局页：手牌行仍贴底，棋盘、商店与积分榜不越界。
func _check_game_tall(size: Vector2) -> void:
    var saved_view: Dictionary = GameStore.view
    var saved_turn := GameStore.turn
    var saved_phase := GameStore.phase
    var saved_player := GameStore.player_id
    GameStore.player_id = "p0"
    GameStore.turn = 3
    GameStore.phase = Protocol.PHASE_RECRUIT
    GameStore.view = Fixture.game_view()

    var host := Control.new()
    host.name = "TallGameHost"
    host.size = size
    add_child(host)
    var page := await _instantiate_into("res://pages/game/game.tscn", host)
    if page != null:
        var bounds := Rect2(host.get_global_rect().position, size).grow(1.0)
        var frame := page.get_node_or_null(UILayout.CONTENT_FRAME_NAME) as Control
        var bottom := size.y
        if frame != null:
            bottom = frame.get_global_rect().end.y
        var hand_strip := page.find_child("HandStrip", true, false) as Control
        var hand_row := page.find_child("HandRow", true, false) as Control
        _check(hand_strip != null, "对局页缺少 HandStrip")
        if hand_strip != null:
            _check(
                bounds.encloses(hand_strip.get_global_rect()),
                "手牌行在 %s 下越界：%s" % [str(size), str(hand_strip.get_global_rect())],
            )
            _check(
                hand_strip.get_global_rect().end.y >= bottom - 8.0,
                "手牌行应贴住内容框底部，实际底边 %.1f" % hand_strip.get_global_rect().end.y,
            )
        if hand_row != null:
            _check(
                hand_row.get_global_rect().end.y >= bottom - 8.0,
                "手牌卡应贴住内容框底部，实际底边 %.1f" % hand_row.get_global_rect().end.y,
            )
        for node_name in ["BoardRow", "ShopRow", "PlayersPanel"]:
            var node := page.find_child(str(node_name), true, false) as Control
            _check(
                node != null and bounds.encloses(node.get_global_rect()),
                "%s 在 %s 下越界或缺失" % [node_name, str(size)],
            )
    host.queue_free()
    GameStore.view = saved_view
    GameStore.turn = saved_turn
    GameStore.phase = saved_phase
    GameStore.player_id = saved_player
    await get_tree().process_frame


## 行内每张卡都不得渲染介绍。
func _check_empty_desc(row: Node, label: String) -> void:
    if row == null:
        _check(false, "%s所在行缺失" % label)
        return
    var checked := 0
    for card in row.get_children():
        if card is Control and card.find_child("CardDesc", true, false) != null:
            _check(false, "%s 不应显示介绍" % label)
            return
        checked += 1
    _check(checked > 0, "%s所在行没有卡牌可检查" % label)


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
    return page.find_child(UILayout.SAFE_AREA_NAME, true, false) as SafeAreaMargin


func _count_safe_areas(page: Node) -> int:
    return _count_named(page, UILayout.SAFE_AREA_NAME)


func _count_named(node: Node, node_name: String) -> int:
    var count := 0
    for child in node.get_children():
        if str(child.name) == node_name:
            count += 1
        count += _count_named(child, node_name)
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
