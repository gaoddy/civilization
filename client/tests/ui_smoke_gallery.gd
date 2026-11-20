extends "res://tests/smoke_case.gd"
## 卡牌图鉴冒烟：大厅全量页、章节/分类筛选、分页、常驻详情与对局覆盖层卡池。


func _ready() -> void:
    await _check_lobby_page()
    await _check_filters()
    await _check_categories_and_kinds()
    await _check_hero_section()
    await _check_mode_filters()
    await _check_empty_hero_pool()
    await _check_detail_art()
    await _check_empty_catalog()
    await _check_pagination()
    await _check_detail()
    await _check_skill_names()
    await _check_lobby_entry()
    await _check_match_overlay()
    _finish()


func _check_lobby_page() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    _expect(instance.find_child("GalleryTitle", true, false) != null, "图鉴页缺少标题")
    var header = instance.find_child("GalleryHeader", true, false)
    var chapters = instance.find_child("GalleryChapters", true, false)
    _expect(
        header != null and chapters != null and chapters.get_parent() == header,
        "时代章节栏应与标题、返回按钮同处头部一行",
    )
    _expect(instance.find_child("GallerySubtitle", true, false) == null, "头部不应再有副标题胶囊")
    _expect(instance.find_child("GalleryCount", true, false) == null, "头部不应再有计数")
    var same_row := true
    if chapters != null:
        for child in chapters.get_children():
            if child is Button and not is_equal_approx(child.position.y, chapters.get_child(0).position.y):
                same_row = false
    _expect(same_row, "时代章节按钮应保持在头部同一行")
    _expect(
        header != null and header.get_global_rect().end.x <= instance.get_viewport_rect().size.x,
        "图鉴头部不应横向溢出视口",
    )
    _expect(
        _filtered_count(instance) == 432,
        "大厅图鉴应有 432 张，实际：%d" % _filtered_count(instance),
    )
    var all_button = instance.find_child("ChapterAll", true, false)
    _expect(
        all_button != null and all_button.text.contains("432") and all_button.text.contains("\n"),
        "章节按钮应为名称在上、计数在下，实际：%s" % (all_button.text if all_button != null else "无"),
    )
    var rows = instance.find_child("GalleryGridRows", true, false)
    _expect(rows != null and rows.columns == 6, "图鉴网格应为 6 列")
    _expect(rows != null and rows.get_child_count() == 18, "首屏应只构建 18 张卡")
    _expect(instance.find_child("GalleryScroll", true, false) == null, "图鉴网格不应再有滚动容器")
    if rows != null and rows.get_child_count() > 0:
        var last_tile := rows.get_child(rows.get_child_count() - 1) as Control
        _expect(
            last_tile != null
                and last_tile.get_global_rect().end.y <= instance.get_viewport_rect().size.y,
            "18 张应在一屏内放下，不出现纵向滚动",
        )
    var label = instance.find_child("GalleryPageLabel", true, false)
    _expect(
        label != null and label.text == tr("ui.gallery.page") % [1, 24],
        "432 张应按 18 张一页显示 1/24，实际：%s" % (label.text if label != null else "无"),
    )
    var detail_name = instance.find_child("GalleryDetailName", true, false)
    _expect(detail_name != null and detail_name.text != "", "首屏应常驻选中首张卡")

    _expect(instance.find_child("Taxonneutral", true, false) != null, "分类目录缺少中立")
    _expect(instance.find_child("Taxonmammal", true, false) != null, "分类目录缺少哺乳纲")
    _expect(instance.find_child("Taxonmachine", true, false) != null, "分类目录缺少机械")
    _expect(instance.find_child("Taxoneffect", true, false) == null, "效果牌不应作为顶层分类")
    _expect(instance.find_child("Taxonscience", true, false) == null, "科学分类不应作为顶层分类")
    _expect(instance.find_child("Taxonhero", true, false) == null, "英雄不应作为左侧分类")

    var mode_bar = instance.find_child("GalleryModeBar", true, false)
    var mode_toggle = instance.find_child("ModeToggle", true, false)
    _expect(
        mode_bar != null and mode_toggle != null,
        "标题右侧缺少随从 / 英雄视图切换",
    )
    _expect(
        mode_toggle != null and mode_toggle.text == tr("ui.gallery.mode_minion"),
        "图鉴默认应显示随从视图，实际：%s" % (mode_toggle.text if mode_toggle != null else "无"),
    )
    _expect(
        instance.find_child("ModeMinion", true, false) == null
            and instance.find_child("ModeHero", true, false) == null,
        "视图切换应只显示当前视图的一个按钮",
    )

    var toc = instance.find_child("GalleryToc", true, false)
    var detail = instance.find_child("GalleryDetail", true, false)
    var body = instance.find_child("GalleryBody", true, false)
    _expect(
        toc != null and toc.get_child_count() > 0 and str(toc.get_child(0).name) == "TaxonAll",
        "分类列表不应再有标题，首项应为「全部」",
    )
    _expect(
        toc != null and toc.size.x >= UITheme.metric("gallery-toc-width"),
        "分类目录应按令牌宽度占位",
    )
    _expect(
        detail != null and is_equal_approx(detail.size.x, UITheme.metric("gallery-detail-width")),
        "常驻详情应按令牌宽度占位，实际：%s" % (detail.size.x if detail != null else "无"),
    )
    _expect(
        body != null and body.get_global_rect().end.x <= instance.get_viewport_rect().size.x,
        "图鉴主体不应横向溢出视口",
    )
    await _unmount(instance)


func _check_filters() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return

    var chapter = instance.find_child("Chapter1", true, false)
    _expect(chapter != null, "章节栏缺少史前章")
    if chapter != null:
        await _push_click(instance, chapter.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 63,
        "史前章应有 63 张，实际：%d" % _filtered_count(instance),
    )

    chapter = instance.find_child("Chapter7", true, false)
    _expect(chapter != null, "章节栏缺少未来章")
    if chapter != null:
        await _push_click(instance, chapter.get_global_rect().get_center())
    var fish = instance.find_child("Taxonfish", true, false)
    _expect(fish != null, "分类目录缺少鱼纲")
    _expect(fish != null and fish.disabled, "未来章下没有鱼纲卡时该分类应禁用")
    var mammal = instance.find_child("Taxonmammal", true, false)
    _expect(mammal != null, "分类目录缺少哺乳纲")
    if mammal != null:
        await _push_click(instance, mammal.get_global_rect().get_center())

    _expect(
        _filtered_count(instance) == 11,
        "未来 + 哺乳纲应有 11 张，实际：%d" % _filtered_count(instance),
    )

    var all_taxon = instance.find_child("TaxonAll", true, false)
    _expect(all_taxon != null, "分类目录缺少「全部」")
    if all_taxon != null:
        await _push_click(instance, all_taxon.get_global_rect().get_center())
    var all_chapter = instance.find_child("ChapterAll", true, false)
    _expect(all_chapter != null, "章节栏缺少「全部」")
    if all_chapter != null:
        await _push_click(instance, all_chapter.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 432,
        "清除筛选后应恢复 432 张，实际：%d" % _filtered_count(instance),
    )
    await _unmount(instance)


## 玩法与「中立」顶层分类，以及选中后展开的随从 / 效果二级筛选。
func _check_categories_and_kinds() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var neutral = instance.find_child("Taxonneutral", true, false)
    _expect(neutral != null, "分类目录缺少中立")
    if neutral != null:
        await _push_click(instance, neutral.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 178,
        "中立应有 178 张，实际：%d" % _filtered_count(instance),
    )

    var kind_minion = instance.find_child("KindMinion", true, false)
    var kind_effect = instance.find_child("KindEffect", true, false)
    _expect(
        instance.find_child("KindAll", true, false) != null
            and kind_minion != null
            and kind_effect != null,
        "选中顶层分类后应展开随从 / 效果二级目录",
    )
    if kind_minion != null:
        await _push_click(instance, kind_minion.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 78,
        "中立 + 随从应有 78 张，实际：%d" % _filtered_count(instance),
    )
    # 二级筛选重建后旧节点已释放，必须重新取当前节点。
    kind_effect = instance.find_child("KindEffect", true, false)
    if kind_effect != null:
        await _push_click(instance, kind_effect.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 100,
        "中立 + 效果应有 100 张，实际：%d" % _filtered_count(instance),
    )

    var mammal_taxon = instance.find_child("Taxonmammal", true, false)
    _expect(mammal_taxon != null, "分类目录缺少哺乳纲")
    if mammal_taxon != null:
        await _push_click(instance, mammal_taxon.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 100,
        "哺乳纲顶层应有 100 张，实际：%d" % _filtered_count(instance),
    )
    var kind_all = instance.find_child("KindAll", true, false)
    _expect(
        kind_all != null and kind_all.button_pressed,
        "切换顶层分类后二级筛选应回到全部",
    )

    var all_taxon = instance.find_child("TaxonAll", true, false)
    if all_taxon != null:
        await _push_click(instance, all_taxon.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 432,
        "清除分类筛选后应恢复 432 张，实际：%d" % _filtered_count(instance),
    )
    await _unmount(instance)


## 详情预览：有美术显示卡图，无美术显示占位，不再现场拼装卡面。
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

    detail.show_card("adam-smith")
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


## 随从 / 英雄视图切换：英雄视图隐藏左侧目录，英雄分类顶部居中，详情走英雄口径。
func _check_hero_section() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var mode_toggle = instance.find_child("ModeToggle", true, false)
    _expect(mode_toggle != null, "标题右侧缺少视图切换")
    if mode_toggle != null:
        await _push_click(instance, mode_toggle.get_global_rect().get_center())
    _expect(
        mode_toggle != null and mode_toggle.text == tr("ui.gallery.mode_hero"),
        "切到英雄视图后按钮应显示英雄，实际：%s"
        % (mode_toggle.text if mode_toggle != null else "无"),
    )
    _expect(
        _filtered_count(instance) == 32,
        "英雄视图应有 32 张，实际：%d" % _filtered_count(instance),
    )
    var toc = instance.find_child("GalleryToc", true, false)
    _expect(
        toc != null and not toc.is_visible_in_tree(),
        "英雄视图不应显示左侧分类目录",
    )
    var chapters = instance.find_child("GalleryChapters", true, false)
    _expect(
        chapters != null and not chapters.is_visible_in_tree(),
        "英雄视图不应显示时代章节",
    )
    var hero_bar = instance.find_child("GalleryHeroCategories", true, false)
    _expect(
        hero_bar != null and hero_bar.is_visible_in_tree(),
        "英雄视图应显示顶部居中的英雄分类栏",
    )
    var kind_all = instance.find_child("KindAll", true, false)
    _expect(
        kind_all == null or not kind_all.is_visible_in_tree(),
        "英雄视图不应展开随从/效果二级筛选",
    )
    var hero_all = instance.find_child("HeroCategoryAll", true, false)
    _expect(
        hero_all != null and hero_all.text.contains("32"),
        "英雄分类「全部」应计 32 张，实际：%s" % (hero_all.text if hero_all != null else "无"),
    )
    var header = instance.find_child("GalleryHeader", true, false) as Control
    var body = instance.find_child("GalleryBody", true, false) as Control
    var view_size: Vector2 = instance.get_viewport_rect().size
    _expect(
        header != null and header.get_global_rect().end.x <= view_size.x,
        "英雄视图头部不应横向溢出视口",
    )
    _expect(
        body != null and body.get_global_rect().end.x <= view_size.x,
        "英雄视图主体不应横向溢出视口",
    )
    var hero_rows = instance.find_child("GalleryGridRows", true, false) as Control
    _expect(
        hero_rows != null and hero_rows.get_global_rect().end.y <= view_size.y,
        "英雄视图网格不应纵向溢出视口",
    )
    var mammal = instance.find_child("HeroCategoryMammal", true, false)
    _expect(
        mammal != null and mammal.text.contains("7"),
        "英雄分类哺乳纲应计 7 张，实际：%s" % (mammal.text if mammal != null else "无"),
    )
    var fungus = instance.find_child("HeroCategoryFungus", true, false)
    _expect(
        fungus != null and fungus.disabled,
        "没有英雄的玩法分类应显示并禁用",
    )
    if mammal != null:
        await _push_click(instance, mammal.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 7,
        "英雄哺乳纲应有 7 张，实际：%d" % _filtered_count(instance),
    )
    hero_all = instance.find_child("HeroCategoryAll", true, false)
    if hero_all != null:
        await _push_click(instance, hero_all.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 32,
        "英雄分类清空后应恢复 32 张，实际：%d" % _filtered_count(instance),
    )
    var rows = instance.find_child("GalleryGridRows", true, false)
    if rows != null and rows.get_child_count() > 0:
        var first_tile: Control = rows.get_child(0)
        _expect(
            first_tile.find_child("CardBadge", true, false) != null,
            "英雄卡面应有「英雄」角标",
        )
        _expect(
            first_tile.find_child("CostBadge", true, false) == null,
            "英雄卡面不应有费用角标",
        )
        _expect(
            first_tile.find_child("CardAttack", true, false) == null
                and first_tile.find_child("CardHealth", true, false) == null,
            "英雄卡面不应有攻击与生命",
        )
    var detail = instance.find_child("GalleryDetail", true, false)
    var detail_name = instance.find_child("GalleryDetailName", true, false)
    _expect(detail_name != null and detail_name.text != "", "英雄视图应常驻选中首张英雄")
    if detail != null:
        var text := _node_text(detail)
        _expect(
            text.contains(TranslationServer.translate("ui.game.detail.hero")),
            "英雄详情应标为英雄：%s" % text,
        )
        _expect(
            not text.contains(TranslationServer.translate("ui.game.detail.taxonomy")),
            "英雄详情不应显示科学分类：%s" % text,
        )
        _expect(
            not text.contains(TranslationServer.translate("ui.game.detail.era")),
            "英雄详情不应显示时代：%s" % text,
        )
    # 切回随从视图恢复普通卡池、左侧目录与时代章节。
    if mode_toggle != null:
        await _push_click(instance, mode_toggle.get_global_rect().get_center())
    _expect(
        mode_toggle != null and mode_toggle.text == tr("ui.gallery.mode_minion"),
        "切回随从视图后按钮应显示随从，实际：%s"
        % (mode_toggle.text if mode_toggle != null else "无"),
    )
    _expect(
        _filtered_count(instance) == 432,
        "切回随从视图应恢复 432 张，实际：%d" % _filtered_count(instance),
    )
    toc = instance.find_child("GalleryToc", true, false)
    _expect(
        toc != null and toc.is_visible_in_tree(),
        "切回随从视图应恢复左侧分类目录",
    )
    chapters = instance.find_child("GalleryChapters", true, false)
    _expect(
        chapters != null and chapters.is_visible_in_tree(),
        "切回随从视图应恢复时代章节",
    )
    await _unmount(instance)


## 切换视图保留各视图筛选：随从视图选中的时代在英雄视图往返后仍然生效。
func _check_mode_filters() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var chapter = instance.find_child("Chapter1", true, false)
    if chapter != null:
        await _push_click(instance, chapter.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 63,
        "史前章应有 63 张，实际：%d" % _filtered_count(instance),
    )
    var mode_toggle = instance.find_child("ModeToggle", true, false)
    if mode_toggle != null:
        await _push_click(instance, mode_toggle.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 32,
        "英雄视图不受时代筛选影响，实际：%d" % _filtered_count(instance),
    )
    if mode_toggle != null:
        await _push_click(instance, mode_toggle.get_global_rect().get_center())
    _expect(
        _filtered_count(instance) == 63,
        "切回随从视图应保留史前筛选，实际：%d" % _filtered_count(instance),
    )
    await _unmount(instance)


## 本局没有可用英雄时，英雄视图切换禁用且保持随从视图。
func _check_empty_hero_pool() -> void:
    var gallery := CardGallery.new()
    gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(gallery)
    gallery.set_catalog(
        CardDB.cards.keys(),
        tr("ui.gallery.title"),
        tr("ui.gallery.back_lobby"),
        [],
    )
    await get_tree().process_frame
    var mode_toggle = gallery.find_child("ModeToggle", true, false)
    _expect(
        mode_toggle != null and mode_toggle.disabled,
        "没有可用英雄时应禁用视图切换",
    )
    _expect(
        mode_toggle != null and mode_toggle.text == tr("ui.gallery.mode_minion"),
        "没有可用英雄时应保持随从视图",
    )
    _expect(gallery.call("_filtered_ids").size() == 432, "没有可用英雄时随从视图仍为 432 张")
    gallery.queue_free()
    await get_tree().process_frame


## 空图鉴：没有章节、没有卡片，只显示空态与详情占位。
func _check_empty_catalog() -> void:
    var gallery := CardGallery.new()
    gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(gallery)
    gallery.set_catalog(
        [],
        tr("ui.gallery.title"),
        tr("ui.gallery.back_lobby"),
    )
    await get_tree().process_frame
    _expect(gallery.call("_filtered_ids").size() == 0, "空图鉴不应有可展示卡牌")
    var empty := gallery.find_child("GalleryEmpty", true, false)
    _expect(empty != null and empty.is_visible_in_tree(), "空图鉴应显示空态")
    _expect(gallery.find_child("GalleryDetailName", true, false) == null, "空图鉴不应有详情卡名")
    gallery.queue_free()
    await get_tree().process_frame


func _check_pagination() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var next = instance.find_child("GalleryNext", true, false)
    _expect(next != null and not next.disabled, "首屏「下一页」应可用")
    if next != null:
        await _push_click(instance, next.get_global_rect().get_center())
    var label = instance.find_child("GalleryPageLabel", true, false)
    _expect(
        label != null and label.text == tr("ui.gallery.page") % [2, 24],
        "翻页后应显示 2/24，实际：%s" % (label.text if label != null else "无"),
    )
    var prev = instance.find_child("GalleryPrev", true, false)
    _expect(prev != null and not prev.disabled, "第二页「上一页」应可用")
    await _unmount(instance)


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
    await _push_click(instance, tile.get_global_rect().get_center())
    var detail_name = instance.find_child("GalleryDetailName", true, false)
    _expect(
        detail_name != null and detail_name.text == CardDB.display_name(card_id),
        "点击第二张卡后详情应切换为 %s" % CardDB.display_name(card_id),
    )
    var detail_text := _node_text(instance.find_child("GalleryDetail", true, false))
    _expect(
        detail_text.contains(tr("ui.game.detail.type")),
        "详情应包含类型行",
    )
    _expect(
        detail_text.contains(tr("ui.game.detail.taxonomy")),
        "详情应包含分类行",
    )
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
    var instance := await _check_scene("res://scenes/lobby.tscn", false)
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
    var instance := await _check_scene("res://scenes/game.tscn", false)
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
        _expect(ids.size() == 198, "本局卡池应为 198 张，实际：%d" % ids.size())
        _expect(ids.has("alan-turing"), "中立且关联机械的卡应在本局卡池")
        _expect(ids.has("airplane"), "中立且无玩法关联的卡应在本局卡池")
        _expect(not ids.has("bruce-lee"), "未启用玩法的哺乳纲卡不应在本局卡池")
        _expect(not ids.has("ammonite-token"), "本局图鉴不应包含 Token")
        _expect(not ids.has("aristotle"), "关联玩法未启用的中立卡不应在本局卡池")

    _expect(
        gallery != null and gallery.call("_filtered_ids").size() == 198,
        "本局图鉴应筛出 198 张",
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
    var key := InputEventKey.new()
    key.keycode = KEY_ESCAPE
    key.physical_keycode = KEY_ESCAPE
    key.pressed = true
    instance.get_viewport().push_input(key, true)
    await get_tree().process_frame
    _expect(not overlay.is_visible_in_tree(), "Esc 应收起图鉴")
    await _unmount(instance)


func _mount_gallery() -> Node:
    GameStore.player_id = "p0"
    return await _check_scene("res://scenes/gallery.tscn", false)


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
