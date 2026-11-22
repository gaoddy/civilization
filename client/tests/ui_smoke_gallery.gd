extends "res://tests/smoke_case.gd"
## 卡牌图鉴冒烟：去标题、筛选下拉、9 列网格、详情浮层、手机档位与对局覆盖层卡池。


func _ready() -> void:
    await _check_lobby_page()
    await _check_filters()
    await _check_categories_and_kinds()
    await _check_hero_section()
    await _check_mode_filters()
    await _check_empty_hero_pool()
    await _check_detail_art()
    await _check_empty_catalog()
    await _check_scroll_loading()
    await _check_phone_layout()
    await _check_tablet_layout()
    await _check_detail_drawer()
    await _check_detail()
    await _check_skill_names()
    await _check_lobby_entry()
    await _check_match_overlay()
    _finish()


## 页面骨架：没有标题与左栏，头部是视图分段 + 三个筛选下拉 + 计数 + 返回。
func _check_lobby_page() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    _expect(instance.find_child("GalleryTitle", true, false) == null, "图鉴页不应再显示标题")
    var header = instance.find_child("GalleryHeader", true, false)
    var mode_bar = instance.find_child("GalleryModeBar", true, false)
    var minion = instance.find_child("ModeMinion", true, false)
    var hero = instance.find_child("ModeHero", true, false)
    _expect(
        header != null and mode_bar != null and mode_bar.get_parent() == header,
        "视图切换应与筛选、返回同处头部一行",
    )
    _expect(minion != null and hero != null, "视图切换应为随从 / 英雄两个分段")
    _expect(
        minion != null and minion.button_pressed and minion.text == tr("ui.gallery.mode_minion"),
        "图鉴默认应处于随从视图",
    )
    _expect(instance.find_child("GalleryToc", true, false) == null, "图鉴不应再有左侧分类目录")
    _expect(instance.find_child("GalleryCategoryPanel", true, false) == null, "图鉴不应再有手机分类面板")
    _expect(instance.find_child("GallerySubtitle", true, false) == null, "头部不应有副标题")
    var count = instance.find_child("GalleryCount", true, false)
    _expect(count != null and count.text.contains("432"), "头部应显示结果计数 432")
    for node_name in ["GalleryEraSelect", "GalleryPlaySelect", "GalleryKindSelect"]:
        var select := instance.find_child(node_name, true, false) as Button
        _expect(select != null and select.text.contains("全部"), "筛选下拉缺少初始值：%s" % node_name)
    _expect(
        header != null and header.get_global_rect().end.x <= instance.get_viewport_rect().size.x,
        "图鉴头部不应横向溢出视口",
    )
    var header_height := float(UIControls.compact_button_height())
    _expect(
        is_equal_approx(header_height, 24.0),
        "工具行按钮高度应为 24，实际：%d" % int(header_height),
    )
    _expect(
        header != null and is_equal_approx(header.size.y, header_height),
        "桌面图鉴头部高度应跟随工具行按钮高度（%d），实际：%s"
        % [int(header_height), str(header.size) if header != null else "无"],
    )
    for node_name in [
        "ModeMinion",
        "ModeHero",
        "GalleryEraSelect",
        "GalleryPlaySelect",
        "GalleryKindSelect",
        "GalleryClose",
    ]:
        var control := instance.find_child(node_name, true, false) as Control
        _expect(
            control != null and is_equal_approx(control.size.y, header_height),
            "头部控件应与工具行按钮同高：%s，实际：%s"
            % [node_name, str(control.size) if control != null else "无"],
        )
    _expect(
        _filtered_count(instance) == 432,
        "大厅图鉴应有 432 张，实际：%d" % _filtered_count(instance),
    )
    var rows = instance.find_child("GalleryGridRows", true, false)
    _expect(rows != null and rows.columns == 9, "桌面图鉴网格应为 9 列")
    _expect(rows != null and rows.get_child_count() >= 27, "首屏至少加载一批 27 张卡")
    var scroll = instance.find_child("GalleryScroll", true, false)
    _expect(scroll is ScrollContainer, "图鉴网格应有滚动容器")
    _expect(instance.find_child("GalleryPager", true, false) == null, "图鉴不应再有分页条")
    var status = instance.find_child("GalleryLoadStatus", true, false)
    _expect(
        status != null and status.text.contains("已显示") and status.text.contains("继续下滑加载"),
        "网格底部应显示已加载张数与加载提示，实际：%s" % (status.text if status != null else "无"),
    )
    var initial_layer := instance.find_child("GalleryDetailLayer", true, false) as Control
    _expect(
        initial_layer != null and not initial_layer.is_visible_in_tree(),
        "进入图鉴不应自动打开详情浮层",
    )
    var first_tile: GalleryCardTile = null
    if rows != null and rows.get_child_count() > 0:
        first_tile = rows.get_child(0) as GalleryCardTile
    _expect(
        first_tile != null and first_tile.selected,
        "进入图鉴应预选中首张卡并保留选中描边",
    )
    await _unmount(instance)


## 时代与玩法下拉：计数、禁用与清除。
func _check_filters() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return

    await _pick(instance, "GalleryEraSelect", "Chapter1")
    _expect(
        _filtered_count(instance) == 86,
        "史前章应有 86 张，实际：%d" % _filtered_count(instance),
    )

    await _pick(instance, "GalleryEraSelect", "Chapter7")
    _expect(
        _filtered_count(instance) == 47,
        "未来章应有 47 张，实际：%d" % _filtered_count(instance),
    )
    # 未来章下没有鱼纲卡时该分类应禁用。
    var select := instance.find_child("GalleryPlaySelect", true, false) as Button
    await _push_click(instance, select.get_global_rect().get_center())
    await get_tree().process_frame
    var fish = instance.find_child("TaxonFish", true, false)
    _expect(fish != null and fish.disabled, "未来章下没有鱼纲卡时该分类应禁用")
    var mammal = instance.find_child("TaxonMammal", true, false)
    _expect(mammal != null, "玩法下拉缺少哺乳纲")
    if mammal != null:
        await _push_click(instance, mammal.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 15,
        "未来 + 哺乳纲应有 15 张，实际：%d" % _filtered_count(instance),
    )

    await _pick(instance, "GalleryEraSelect", "ChapterAll")
    _expect(
        _filtered_count(instance) == 122,
        "清除时代后应保留哺乳纲的 122 张，实际：%d" % _filtered_count(instance),
    )
    await _pick(instance, "GalleryPlaySelect", "TaxonAll")
    _expect(
        _filtered_count(instance) == 432,
        "清除筛选后应恢复 432 张，实际：%d" % _filtered_count(instance),
    )
    await _unmount(instance)


## 13 个玩法与随从 / 效果二级筛选；多玩法卡按全部归属计数。
func _check_categories_and_kinds() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    await _pick(instance, "GalleryPlaySelect", "TaxonMammal")
    _expect(
        _filtered_count(instance) == 122,
        "哺乳纲应有 122 张，实际：%d" % _filtered_count(instance),
    )

    await _pick(instance, "GalleryKindSelect", "KindMinion")
    _expect(
        _filtered_count(instance) == 97,
        "哺乳纲 + 随从应有 97 张，实际：%d" % _filtered_count(instance),
    )
    await _pick(instance, "GalleryKindSelect", "KindEffect")
    _expect(
        _filtered_count(instance) == 25,
        "哺乳纲 + 效果应有 25 张，实际：%d" % _filtered_count(instance),
    )

    # 切换顶层分类时二级筛选回到「全部」。
    await _pick(instance, "GalleryPlaySelect", "TaxonMammal")
    _expect(
        _filtered_count(instance) == 122,
        "哺乳纲顶层应有 122 张，实际：%d" % _filtered_count(instance),
    )
    var kind_select := instance.find_child("GalleryKindSelect", true, false) as Button
    _expect(
        kind_select != null and kind_select.text.contains(tr("ui.gallery.kind_all")),
        "切换顶层分类后二级筛选应回到全部，实际：%s" % (kind_select.text if kind_select != null else "无"),
    )

    await _pick(instance, "GalleryPlaySelect", "TaxonAll")
    _expect(
        _filtered_count(instance) == 432,
        "清除分类筛选后应恢复 432 张，实际：%d" % _filtered_count(instance),
    )
    await _unmount(instance)


## 随从 / 英雄视图：英雄视图隐藏时代与类型，玩法下拉改用英雄分类。
func _check_hero_section() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var mode_hero := instance.find_child("ModeHero", true, false) as Button
    _expect(mode_hero != null, "头部缺少英雄视图分段")
    if mode_hero != null:
        await _push_click(instance, mode_hero.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 32,
        "英雄视图应有 32 张，实际：%d" % _filtered_count(instance),
    )
    var era_select := instance.find_child("GalleryEraSelect", true, false) as Control
    var kind_select := instance.find_child("GalleryKindSelect", true, false) as Control
    _expect(era_select == null or not era_select.is_visible_in_tree(), "英雄视图不应显示时代筛选")
    _expect(kind_select == null or not kind_select.is_visible_in_tree(), "英雄视图不应显示类型筛选")
    var play_select := instance.find_child("GalleryPlaySelect", true, false) as Control
    _expect(
        play_select != null and play_select.is_visible_in_tree(),
        "英雄视图应显示玩法筛选",
    )

    await _pick(instance, "GalleryPlaySelect", "HeroCategoryMammal")
    _expect(
        _filtered_count(instance) == 20,
        "英雄哺乳纲应有 20 张，实际：%d" % _filtered_count(instance),
    )
    await _pick(instance, "GalleryPlaySelect", "HeroCategoryAll")
    _expect(
        _filtered_count(instance) == 32,
        "英雄分类清空后应恢复 32 张，实际：%d" % _filtered_count(instance),
    )

    var rows = instance.find_child("GalleryGridRows", true, false)
    if rows != null and rows.get_child_count() > 0:
        var first_tile: Control = rows.get_child(0)
        _expect(first_tile.find_child("CardBadge", true, false) == null, "英雄卡面不应有角标")
        _expect(first_tile.find_child("CardDesc", true, false) == null, "英雄卡面不应显示技能文本")
        _expect(first_tile.find_child("CostBadge", true, false) == null, "英雄卡面不应有费用角标")
        _expect(
            first_tile.find_child("CardAttack", true, false) == null
                and first_tile.find_child("CardHealth", true, false) == null,
            "英雄卡面不应有攻击与生命",
        )
        await _push_click(instance, first_tile.get_global_rect().get_center())
        var layer := instance.find_child("GalleryDetailLayer", true, false) as Control
        _expect(layer != null and layer.is_visible_in_tree(), "点击英雄卡应打开详情浮层")
        var detail_name := instance.find_child("GalleryDetailName", true, false)
        _expect(detail_name != null and detail_name.text != "", "英雄详情应显示卡名")
        var intro := instance.find_child("GalleryDetailIntroduction", true, false)
        _expect(intro != null and intro.text != "", "英雄详情应显示介绍")
        var text := _node_text(instance.find_child("GalleryDetail", true, false))
        _expect(
            not text.contains(TranslationServer.translate("ui.game.detail.type")),
            "英雄详情不应显示类型行：%s" % text,
        )
        _expect(
            not text.contains(TranslationServer.translate("ui.game.detail.taxonomy")),
            "英雄详情不应显示科学分类：%s" % text,
        )
        _expect(
            not text.contains(TranslationServer.translate("ui.game.detail.era")),
            "英雄详情不应显示时代：%s" % text,
        )

    # 切回随从视图恢复普通卡池、时代与类型筛选。
    var mode_minion := instance.find_child("ModeMinion", true, false) as Button
    if mode_minion != null:
        await _push_click(instance, mode_minion.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 432,
        "切回随从视图应恢复 432 张，实际：%d" % _filtered_count(instance),
    )
    _expect(era_select != null and era_select.is_visible_in_tree(), "切回随从视图应恢复时代筛选")
    await _unmount(instance)


## 切换视图保留各视图筛选：随从视图选中的时代在英雄视图往返后仍然生效。
func _check_mode_filters() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    await _pick(instance, "GalleryEraSelect", "Chapter1")
    _expect(
        _filtered_count(instance) == 86,
        "史前章应有 86 张，实际：%d" % _filtered_count(instance),
    )
    var mode_hero := instance.find_child("ModeHero", true, false) as Button
    var header := instance.find_child("GalleryHeader", true, false) as Control
    var header_height := header.size.y if header != null else 0.0
    if mode_hero != null:
        await _push_click(instance, mode_hero.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 32,
        "英雄视图不受时代筛选影响，实际：%d" % _filtered_count(instance),
    )
    _expect(
        header != null and is_equal_approx(header.size.y, header_height),
        "切换视图后头部高度应保持不变，实际：%s" % (str(header.size) if header != null else "无"),
    )
    var mode_minion := instance.find_child("ModeMinion", true, false) as Button
    if mode_minion != null:
        await _push_click(instance, mode_minion.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 86,
        "切回随从视图应保留史前筛选，实际：%d" % _filtered_count(instance),
    )
    await _unmount(instance)


## 本局没有可用英雄时，英雄分段禁用且保持随从视图。
func _check_empty_hero_pool() -> void:
    var gallery := CardGallery.new()
    gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(gallery)
    gallery.set_catalog(CardDB.cards.keys(), tr("ui.gallery.back_lobby"), [])
    await get_tree().process_frame
    var hero := gallery.find_child("ModeHero", true, false) as Button
    _expect(hero != null and hero.disabled, "没有可用英雄时应禁用英雄分段")
    _expect(
        hero != null and not hero.button_pressed,
        "没有可用英雄时应保持随从视图",
    )
    _expect(gallery.call("_filtered_ids").size() == 432, "没有可用英雄时随从视图仍为 432 张")
    gallery.queue_free()
    await get_tree().process_frame


## 空图鉴：没有卡片时只显示空态与 0 张计数，详情保持收起。
func _check_empty_catalog() -> void:
    var gallery := CardGallery.new()
    gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(gallery)
    gallery.set_catalog([], tr("ui.gallery.back_lobby"))
    await get_tree().process_frame
    _expect(gallery.call("_filtered_ids").size() == 0, "空图鉴不应有可展示卡牌")
    var empty := gallery.find_child("GalleryEmpty", true, false)
    _expect(empty != null and empty.is_visible_in_tree(), "空图鉴应显示空态")
    var count := gallery.find_child("GalleryCount", true, false) as Label
    _expect(count != null and count.text.contains("0"), "空图鉴计数应为 0")
    var layer := gallery.find_child("GalleryDetailLayer", true, false) as Control
    _expect(layer != null and not layer.is_visible_in_tree(), "空图鉴不应打开详情浮层")
    gallery.queue_free()
    await get_tree().process_frame


func _check_scroll_loading() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var rows = instance.find_child("GalleryGridRows", true, false)
    var scroll = instance.find_child("GalleryScroll", true, false) as ScrollContainer
    if rows == null or scroll == null:
        _expect(false, "图鉴缺少网格或滚动容器")
        await _unmount(instance)
        return
    var before := rows.get_child_count()
    _expect(before >= 27, "首屏应至少加载一批，实际：%d" % before)
    await get_tree().process_frame
    await get_tree().process_frame
    scroll.scroll_vertical = int(scroll.get_v_scroll_bar().max_value)
    await get_tree().process_frame
    await get_tree().process_frame
    await get_tree().process_frame
    _expect(
        rows.get_child_count() > before,
        "滚到底部应加载下一批，实际：%d → %d" % [before, rows.get_child_count()],
    )
    var status = instance.find_child("GalleryLoadStatus", true, false)
    _expect(
        status != null and status.text.begins_with("已显示"),
        "加载后应更新底部状态，实际：%s" % (status.text if status != null else "无"),
    )
    await _unmount(instance)


## 手机 960×540：6 列网格，头部筛选不越界，详情浮层从底部打开且不出屏。
func _check_phone_layout() -> void:
    ScreenProfile.set_device_for_test("phone")
    var host := Control.new()
    host.name = "PhoneGalleryHost"
    host.size = Vector2(960, 540)
    add_child(host)
    var page = load("res://pages/gallery/gallery.tscn").instantiate()
    host.add_child(page)
    await get_tree().process_frame
    await get_tree().process_frame
    await get_tree().process_frame

    var bounds := Rect2(host.get_global_rect().position, host.size).grow(1.0)
    for node_name in ["GalleryHeader", "GalleryEraSelect", "GalleryPlaySelect", "GalleryScroll"]:
        var node := page.find_child(str(node_name), true, false) as Control
        _expect(node != null and node.is_visible_in_tree(), "手机图鉴缺少可见的 %s" % node_name)
        if node != null:
            _expect(
                bounds.encloses(node.get_global_rect()),
                "%s 在手机基准下越界：%s 不在 %s 内"
                % [node_name, str(node.get_global_rect()), str(bounds)],
            )
    _expect(page.find_child("GalleryToc", true, false) == null, "手机图鉴不应再有左侧目录")

    var rows := page.find_child("GalleryGridRows", true, false) as GridContainer
    _expect(rows != null and rows.columns == 6, "手机网格应为 6 列")
    _expect(rows != null and rows.get_child_count() >= 12, "手机首屏至少加载一批 12 张卡")

    var layer := page.find_child("GalleryDetailLayer", true, false) as Control
    _expect(layer != null and not layer.is_visible_in_tree(), "手机进入图鉴不应打开详情")
    var tile := rows.get_child(0) as Control
    if tile != null:
        await _push_click(page, tile.get_global_rect().get_center())
    _expect(layer != null and layer.is_visible_in_tree(), "手机点击卡牌应打开详情浮层")
    var drawer := page.find_child("GalleryDrawer", true, false) as Control
    _expect(drawer != null and bounds.encloses(drawer.get_global_rect()), "手机详情抽屉不应越界")
    var header := page.find_child("GalleryHeader", true, false) as Control
    if drawer != null and header != null:
        var drawer_rect := drawer.get_global_rect()
        _expect(
            is_equal_approx(drawer_rect.position.y, header.get_global_rect().end.y),
            "手机详情抽屉应从头部下沿开始，实际：%s" % str(drawer_rect),
        )
        _expect(
            is_equal_approx(drawer_rect.end.y, host.get_global_rect().end.y),
            "手机详情抽屉应贴到页面底边，实际：%s" % str(drawer_rect),
        )
        _expect(
            is_equal_approx(drawer_rect.size.x, float(UITheme.stage_metric("gallery-drawer-width"))),
            "手机详情抽屉宽度应为 gallery-drawer-width，实际：%s" % str(drawer_rect),
        )
    await _push_key(page, KEY_ESCAPE)
    _expect(layer != null and not layer.is_visible_in_tree(), "Esc 应只收起详情浮层")

    host.queue_free()
    await get_tree().process_frame
    ScreenProfile.set_device_for_test("desktop")


## 平板 1280×720（触控）：头部取触控高度，详情抽屉贴右整高、右侧留 24。
func _check_tablet_layout() -> void:
    ScreenProfile.set_device_for_test("tablet")
    var host := Control.new()
    host.name = "TabletGalleryHost"
    host.size = Vector2(1280, 720)
    add_child(host)
    var page = load("res://pages/gallery/gallery.tscn").instantiate()
    host.add_child(page)
    for _i in range(3):
        await get_tree().process_frame

    var header := page.find_child("GalleryHeader", true, false) as Control
    var toolbar_height := float(UIControls.compact_button_height())
    _expect(
        is_equal_approx(toolbar_height, float(UITheme.metric("toolbar-button-height"))),
        "触控工具行按钮高度应与视觉一致，实际：%d" % int(toolbar_height),
    )
    var touch_style := UITheme.compact_button_style("default", "normal")
    _expect(
        is_zero_approx(touch_style.expand_margin_top)
            and is_zero_approx(touch_style.expand_margin_bottom),
        "触控工具行按钮背景不应外扩，实际：%s/%s"
        % [str(touch_style.expand_margin_top), str(touch_style.expand_margin_bottom)],
    )
    _expect(
        header != null and is_equal_approx(header.size.y, toolbar_height),
        "平板头部高度应与工具行按钮同高，实际：%s" % (str(header.size) if header != null else "无"),
    )
    var rows := page.find_child("GalleryGridRows", true, false) as GridContainer
    _expect(rows != null and rows.columns == 9, "平板图鉴网格应为 9 列")
    if rows != null and rows.get_child_count() > 0:
        var tile := rows.get_child(0) as Control
        await _push_click(page, tile.get_global_rect().get_center())
    var drawer := page.find_child("GalleryDrawer", true, false) as Control
    if header != null and drawer != null:
        var drawer_rect := drawer.get_global_rect()
        var host_rect := host.get_global_rect()
        _expect(
            is_equal_approx(drawer_rect.position.y, header.get_global_rect().end.y),
            "平板详情抽屉应从头部下沿开始，实际：%s" % str(drawer_rect),
        )
        _expect(
            is_equal_approx(drawer_rect.end.y, host_rect.end.y),
            "平板详情抽屉应贴到页面底边，实际：%s" % str(drawer_rect),
        )
        _expect(
            is_equal_approx(drawer_rect.end.x, host_rect.end.x - UITheme.space("space-2xl")),
            "平板详情抽屉右侧应留外边距 24，实际：%s" % str(drawer_rect),
        )
        _expect(
            is_equal_approx(drawer_rect.size.x, float(UITheme.stage_metric("gallery-drawer-width"))),
            "平板详情抽屉宽度应为 gallery-drawer-width，实际：%s" % str(drawer_rect),
        )

    var minion := page.find_child("ModeMinion", true, false) as Button
    var before_height := minion.size.y if minion != null else 0.0
    var era_select := page.find_child("GalleryEraSelect", true, false) as Button
    await _tap(page, era_select.get_global_rect().get_center())
    await get_tree().process_frame
    var chapter1 := page.find_child("Chapter1", true, false) as Button
    if chapter1 != null:
        await _tap(page, chapter1.get_global_rect().get_center())
    await get_tree().process_frame
    _expect(
        minion != null and is_equal_approx(minion.size.y, before_height),
        "触控选择筛选项后分段高度不应变化，实际：%s → %s"
        % [str(before_height), str(minion.size) if minion != null else "无"],
    )
    if minion != null:
        _expect(
            is_equal_approx(minion.size.y, toolbar_height),
            "触控分段高度应等于工具行高度，实际：%s" % str(minion.size),
        )
        for state in ["normal", "hover", "pressed", "hover_pressed", "focus", "disabled"]:
            var style := minion.get_theme_stylebox(state)
            _expect(
                style != null
                    and is_zero_approx(style.expand_margin_top)
                    and is_zero_approx(style.expand_margin_bottom),
                "触控选择筛选项后分段 %s 背景不应外扩，实际：%s"
                % [state, str(style.expand_margin_top) if style != null else "无"],
            )

    host.queue_free()
    await get_tree().process_frame
    ScreenProfile.set_device_for_test("desktop")


## 详情浮层：点卡打开、点遮罩与 Esc 关闭，选中描边同步。
func _check_detail_drawer() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var rows = instance.find_child("GalleryGridRows", true, false)
    if rows == null or rows.get_child_count() < 2:
        _expect(false, "图鉴网格不足两张卡")
        await _unmount(instance)
        return
    var tile := rows.get_child(1) as GalleryCardTile
    var card_id := str(tile.card_id)
    await _push_click(instance, tile.get_global_rect().get_center())
    var layer := instance.find_child("GalleryDetailLayer", true, false) as Control
    _expect(layer != null and layer.is_visible_in_tree(), "点击卡牌应打开详情浮层")
    _expect(tile.selected, "打开详情后卡面应保持选中描边")
    var detail_name = instance.find_child("GalleryDetailName", true, false)
    _expect(
        detail_name != null and detail_name.text == CardDB.display_name(card_id),
        "详情应切换为 %s" % CardDB.display_name(card_id),
    )
    var header := instance.find_child("GalleryHeader", true, false) as Control
    var drawer := instance.find_child("GalleryDrawer", true, false) as Control
    if header != null and drawer != null:
        var drawer_rect := drawer.get_global_rect()
        _expect(
            is_equal_approx(drawer_rect.position.y, header.get_global_rect().end.y),
            "详情抽屉应从头部下沿开始，实际：%s" % str(drawer_rect),
        )
        _expect(
            is_equal_approx(drawer_rect.end.y, layer.get_global_rect().end.y),
            "详情抽屉应贴到页面底边，实际：%s" % str(drawer_rect),
        )
        _expect(
            is_equal_approx(drawer_rect.size.x, float(UITheme.stage_metric("gallery-drawer-width"))),
            "详情抽屉宽度应为 gallery-drawer-width，实际：%s" % str(drawer_rect),
        )
    # 点遮罩关闭，网格仍保留选中。
    await _push_click(instance, Vector2(12, 360))
    _expect(layer != null and not layer.is_visible_in_tree(), "点遮罩应收起详情浮层")
    _expect(tile.selected, "收起浮层后应保留选中描边")
    # 重新打开后按 Esc 先收浮层。
    await _push_click(instance, tile.get_global_rect().get_center())
    _expect(layer != null and layer.is_visible_in_tree(), "再次点卡应重新打开详情浮层")
    await _push_key(instance, KEY_ESCAPE)
    _expect(layer != null and not layer.is_visible_in_tree(), "Esc 应先收起详情浮层")
    _expect(instance.is_inside_tree(), "收起详情不应离开图鉴页")
    await _unmount(instance)


## 详情预览：英雄与随从都取卡面美术，无美术显示占位，不再现场拼装卡面。
func _check_detail_art() -> void:
    var detail := GalleryDetail.new()
    add_child(detail)
    detail.show_card("zhenghe")
    await get_tree().process_frame
    var art := detail.find_child("GalleryDetailArt", true, false) as TextureRect
    _expect(art != null and art.texture != null, "有卡面美术的卡应显示卡图")
    _expect(
        detail.find_child("GalleryDetailArtPlaceholder", true, false) == null,
        "有卡面美术时不应显示占位",
    )
    var card_face_path := ""
    if art != null and art.texture != null:
        card_face_path = art.texture.resource_path

    detail.show_hero("zhenghe")
    await get_tree().process_frame
    var hero_art := detail.find_child("GalleryDetailArt", true, false) as TextureRect
    _expect(hero_art != null and hero_art.texture != null, "英雄详情应显示卡图")
    _expect(
        hero_art != null and hero_art.texture.resource_path.contains("cards/faces"),
        "英雄详情预览应使用卡面而不是立绘",
    )
    _expect(
        hero_art != null and hero_art.texture.resource_path == card_face_path,
        "英雄与随从详情应使用同一张卡面",
    )
    var frame := detail.find_child("GalleryDetailArtFrame", true, false) as Control
    _expect(
        frame != null and frame.custom_minimum_size == Vector2(200, 261),
        "详情预览框应按卡面比例显示为 200×261，实际：%s"
        % (frame.custom_minimum_size if frame != null else "无"),
    )

    detail.show_card("alan-turing")
    await get_tree().process_frame
    _expect(
        detail.find_child("GalleryDetailArt", true, false) == null,
        "无卡面美术时不应构建卡图",
    )
    _expect(
        detail.find_child("GalleryDetailArtPlaceholder", true, false) != null,
        "无卡面美术时应显示占位",
    )
    detail.queue_free()
    await get_tree().process_frame


func _check_detail() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var rows = instance.find_child("GalleryGridRows", true, false)
    if rows == null or rows.get_child_count() < 2:
        _expect(false, "图鉴网格不足两张卡")
        await _unmount(instance)
        return
    var tile := rows.get_child(1) as GalleryCardTile
    var card_id := str(tile.card_id)
    # 触屏拖拽靠 ScrollContainer 收到同一次按下起手，卡面不能吞掉按下事件。
    var scroll := instance.find_child("GalleryScroll", true, false) as ScrollContainer
    var press_reached := [false]
    if scroll != null:
        scroll.gui_input.connect(
            func(event: InputEvent) -> void:
                if event is InputEventMouseButton and (event as InputEventMouseButton).pressed:
                    press_reached[0] = true
        )
    await _push_click(instance, tile.get_global_rect().get_center())
    _expect(scroll != null and press_reached[0], "卡面按下应继续冒泡到滚动区，触屏拖拽才能滚动")
    var detail_name = instance.find_child("GalleryDetailName", true, false)
    _expect(
        detail_name != null and detail_name.text == CardDB.display_name(card_id),
        "点击第二张卡后详情应切换为 %s" % CardDB.display_name(card_id),
    )
    var other := rows.get_child(2) as Control
    if other != null:
        await _real_drag(instance, tile.get_global_rect().get_center(), other.get_global_rect().get_center())
        _expect(
            detail_name != null and detail_name.text == CardDB.display_name(card_id),
            "从当前卡拖到另一张卡松手不应切换详情",
        )
    _expect(
        instance.find_child("GalleryDetailEraGlyph", true, false) == null,
        "普通卡详情卡名旁不应有时代象形",
    )
    var detail_text := _node_text(instance.find_child("GalleryDetail", true, false))
    _expect(detail_text.contains(tr("ui.game.detail.type")), "详情应包含类型行")
    _expect(detail_text.contains(tr("ui.game.detail.taxonomy")), "详情应包含分类行")
    for probe_id in ["adam-smith", "aolinpike-jidian"]:
        var probe := GalleryDetail.new()
        add_child(probe)
        probe.show_card(probe_id)
        await get_tree().process_frame
        _expect(
            probe.find_child("GalleryDetailEraGlyph", true, false) == null,
            "详情不应有时代象形：%s" % probe_id,
        )
        probe.queue_free()
    await _unmount(instance)


## 缺技能展示名时，详情不得把内部技能 id 当成名称泄露出来。
func _check_skill_names() -> void:
    var missing := ""
    var card_id := ""
    for id in CardDB.cards:
        for skill_id in CardDB.card(str(id)).get("skills", []):
            if CardDB.skill_name(str(skill_id)) == "":
                missing = str(skill_id)
                card_id = str(id)
                break
        if missing != "":
            break
    if missing == "":
        return
    var detail := GalleryDetail.new()
    add_child(detail)
    detail.show_card(card_id)
    await get_tree().process_frame
    var text := _node_text(detail)
    _expect(not text.contains(missing), "详情不应显示技能内部 id：%s" % missing)
    detail.queue_free()
    await get_tree().process_frame


func _check_lobby_entry() -> void:
    GameStore.player_id = "p0"
    GameStore.view = FixtureScript.view()
    var instance := await _check_scene("res://pages/lobby/lobby.tscn", false)
    if instance == null:
        return
    var button = instance.find_child("GalleryButton", true, false)
    _expect(button != null, "大厅缺少卡牌图鉴入口")
    if button != null:
        _expect(button.text == tr("ui.gallery.entry"), "大厅图鉴入口文案不正确")
    await _unmount(instance)


func _check_match_overlay() -> void:
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_RECRUIT
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    GameStore.view = FixtureScript.view()
    _clear_combat()
    var instance := await _check_scene("res://pages/game/game.tscn", false)
    if instance == null:
        return

    var button = instance.find_child("GalleryButton", true, false)
    _expect(button != null, "对局右栏缺少图鉴入口")
    if button == null:
        await _unmount(instance)
        return
    await _push_click(instance, button.get_global_rect().get_center())

    var overlay = instance.find_child("GalleryOverlay", true, false)
    _expect(overlay != null and overlay.is_visible_in_tree(), "对局图鉴覆盖层应打开")
    if overlay == null:
        await _unmount(instance)
        return

    var gallery := _card_gallery(instance.find_child("GalleryHeader", true, false))
    _expect(gallery != null, "对局图鉴缺少 CardGallery")
    if gallery != null:
        var ids: Array = gallery.get("_ids")
        _expect(ids.size() == 177, "本局卡池应为 177 张，实际：%d" % ids.size())
        _expect(ids.has("alan-turing"), "机械玩法卡应在本局卡池")
        _expect(ids.has("airplane"), "机械玩法工具卡应在本局卡池")
        _expect(not ids.has("bruce-lee"), "未启用玩法的哺乳纲卡不应在本局卡池")
        _expect(not ids.has("ammonite-token"), "本局图鉴不应包含非池卡")
        _expect(not ids.has("aristotle"), "未启用玩法的爬行纲卡不应在本局卡池")

    _expect(
        gallery != null and gallery.call("_filtered_ids").size() == 177,
        "本局图鉴应筛出 177 张",
    )
    _expect(instance.is_inside_tree(), "打开图鉴不应切换场景")
    _expect(instance.find_child("ShopSlot0", true, false) != null, "打开图鉴不应丢失对局内容")

    var close = instance.find_child("GalleryClose", true, false)
    _expect(close != null, "图鉴缺少返回对局按钮")
    if close != null:
        await _push_click(instance, close.get_global_rect().get_center())
    _expect(not overlay.is_visible_in_tree(), "点击返回对局应收起图鉴")
    _expect(instance.find_child("ShopSlot0", true, false) != null, "收起图鉴后对局仍在")

    await _push_click(instance, button.get_global_rect().get_center())
    _expect(overlay.is_visible_in_tree(), "再次点击图鉴入口应重新打开")
    # 打开详情后，Esc 只收详情；再按一次才收图鉴。
    var rows := instance.find_child("GalleryGridRows", true, false)
    if rows != null and rows.get_child_count() > 0:
        var tile := rows.get_child(0) as Control
        await _push_click(instance, tile.get_global_rect().get_center())
        var layer := instance.find_child("GalleryDetailLayer", true, false) as Control
        _expect(layer != null and layer.is_visible_in_tree(), "对局图鉴点击卡牌应打开详情浮层")
        await _push_key(instance, KEY_ESCAPE)
        _expect(layer != null and not layer.is_visible_in_tree(), "Esc 应先收起详情浮层")
        _expect(overlay.is_visible_in_tree(), "收起详情后图鉴覆盖层应保留")
    await _push_key(instance, KEY_ESCAPE)
    _expect(not overlay.is_visible_in_tree(), "Esc 应收起图鉴")
    await _unmount(instance)


func _mount_gallery() -> Node:
    GameStore.player_id = "p0"
    return await _check_scene("res://pages/gallery/gallery.tscn", false)


## 画布坐标处的触摸按下/松开事件，模拟真机触控。
func _tap(instance: Node, canvas_point: Vector2) -> void:
    for pressed in [true, false]:
        var event := InputEventScreenTouch.new()
        event.index = 0
        event.pressed = pressed
        event.position = canvas_point
        instance.get_viewport().push_input(event, true)
        await get_tree().process_frame


## 打开筛选下拉并点选条目；面板未出现时记为失败并返回。
func _pick(instance: Node, select_name: String, item_name: String) -> void:
    var select := instance.find_child(select_name, true, false) as Button
    if select == null:
        _expect(false, "缺少筛选触发器 %s" % select_name)
        return
    await _push_click(instance, select.get_global_rect().get_center())
    await get_tree().process_frame
    var item := instance.find_child(item_name, true, false) as Button
    if item == null:
        _expect(false, "筛选面板缺少 %s" % item_name)
        return
    await _push_click(instance, item.get_global_rect().get_center())
    await get_tree().process_frame


func _card_gallery(node: Node) -> CardGallery:
    var current := node
    while current != null:
        if current is CardGallery:
            return current as CardGallery
        current = current.get_parent()
    return null


func _filtered_count(instance: Node) -> int:
    var gallery := instance.find_child("CardGallery", true, false) as CardGallery
    return gallery.call("_filtered_ids").size() if gallery != null else 0


func _unmount(instance: Node) -> void:
    instance.queue_free()
    await get_tree().process_frame
