extends "res://tests/smoke_case.gd"
## 卡牌图鉴冒烟：大厅全量页、章节/分类筛选、分页、常驻详情与对局覆盖层卡池。


func _ready() -> void:
    await _check_lobby_page()
    await _check_filters()
    await _check_categories_and_kinds()
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
    var count = instance.find_child("GalleryCount", true, false)
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 432,
        "大厅图鉴应显示 432 张，实际：%s" % (count.text if count != null else "无"),
    )
    var all_button = instance.find_child("ChapterAll", true, false)
    _expect(
        all_button != null and all_button.text.contains("432"),
        "章节栏「全部」应显示 432，实际：%s" % (all_button.text if all_button != null else "无"),
    )
    var rows = instance.find_child("GalleryGridRows", true, false)
    _expect(rows != null and rows.get_child_count() == 40, "首屏应只构建 40 张卡")
    var label = instance.find_child("GalleryPageLabel", true, false)
    _expect(
        label != null and label.text == tr("ui.gallery.page") % [1, 11],
        "432 张应按 40 张一页显示 1/11，实际：%s" % (label.text if label != null else "无"),
    )
    var detail_name = instance.find_child("GalleryDetailName", true, false)
    _expect(detail_name != null and detail_name.text != "", "首屏应常驻选中首张卡")

    _expect(instance.find_child("Taxonneutral", true, false) != null, "分类目录缺少中立")
    _expect(instance.find_child("Taxonmammal", true, false) != null, "分类目录缺少哺乳纲")
    _expect(instance.find_child("Taxonmachine", true, false) != null, "分类目录缺少机械")
    _expect(instance.find_child("Taxoneffect", true, false) == null, "效果牌不应作为顶层分类")
    _expect(instance.find_child("Taxonscience", true, false) == null, "科学分类不应作为顶层分类")

    var toc = instance.find_child("GalleryToc", true, false)
    var detail = instance.find_child("GalleryDetail", true, false)
    var body = instance.find_child("GalleryBody", true, false)
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
    var count = instance.find_child("GalleryCount", true, false)
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 63,
        "史前章应显示 63 张，实际：%s" % (count.text if count != null else "无"),
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

    count = instance.find_child("GalleryCount", true, false)
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 11,
        "未来 + 哺乳纲应显示 11 张，实际：%s" % (count.text if count != null else "无"),
    )

    var all_taxon = instance.find_child("TaxonAll", true, false)
    _expect(all_taxon != null, "分类目录缺少「全部」")
    if all_taxon != null:
        await _push_click(instance, all_taxon.get_global_rect().get_center())
    var all_chapter = instance.find_child("ChapterAll", true, false)
    _expect(all_chapter != null, "章节栏缺少「全部」")
    if all_chapter != null:
        await _push_click(instance, all_chapter.get_global_rect().get_center())
    count = instance.find_child("GalleryCount", true, false)
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 432,
        "清除筛选后应恢复 432 张",
    )
    await _unmount(instance)


## 玩法与「中立」顶层分类，以及选中后展开的随从 / 效果二级筛选。
func _check_categories_and_kinds() -> void:
    var instance := await _mount_gallery()
    if instance == null:
        return
    var count = instance.find_child("GalleryCount", true, false)
    var neutral = instance.find_child("Taxonneutral", true, false)
    _expect(neutral != null, "分类目录缺少中立")
    if neutral != null:
        await _push_click(instance, neutral.get_global_rect().get_center())
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 178,
        "中立应显示 178 张，实际：%s" % (count.text if count != null else "无"),
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
        count != null and count.text == tr("ui.gallery.count") % 78,
        "中立 + 随从应显示 78 张，实际：%s" % (count.text if count != null else "无"),
    )
    # 二级筛选重建后旧节点已释放，必须重新取当前节点。
    kind_effect = instance.find_child("KindEffect", true, false)
    if kind_effect != null:
        await _push_click(instance, kind_effect.get_global_rect().get_center())
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 100,
        "中立 + 效果应显示 100 张，实际：%s" % (count.text if count != null else "无"),
    )

    var mammal_taxon = instance.find_child("Taxonmammal", true, false)
    _expect(mammal_taxon != null, "分类目录缺少哺乳纲")
    if mammal_taxon != null:
        await _push_click(instance, mammal_taxon.get_global_rect().get_center())
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 100,
        "哺乳纲顶层应显示 100 张，实际：%s" % (count.text if count != null else "无"),
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
        count != null and count.text == tr("ui.gallery.count") % 432,
        "清除分类筛选后应恢复 432 张",
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


## 空图鉴：没有章节、没有卡片，只显示空态与详情占位。
func _check_empty_catalog() -> void:
    var gallery := CardGallery.new()
    gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(gallery)
    gallery.set_catalog(
        [],
        tr("ui.gallery.title"),
        tr("ui.gallery.back_lobby"),
        tr("ui.gallery.all_cards"),
    )
    await get_tree().process_frame
    var count = gallery.find_child("GalleryCount", true, false)
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 0,
        "空图鉴应显示 0 张",
    )
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
        label != null and label.text == tr("ui.gallery.page") % [2, 11],
        "翻页后应显示 2/11，实际：%s" % (label.text if label != null else "无"),
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

    var gallery := _card_gallery(instance.find_child("GalleryCount", true, false))
    _expect(gallery != null, "对局图鉴缺少 CardGallery")
    if gallery != null:
        var ids: Array = gallery.get("_ids")
        _expect(ids.size() == 198, "本局卡池应为 198 张，实际：%d" % ids.size())
        _expect(ids.has("alan-turing"), "中立且关联机械的卡应在本局卡池")
        _expect(ids.has("airplane"), "中立且无玩法关联的卡应在本局卡池")
        _expect(not ids.has("bruce-lee"), "未启用玩法的哺乳纲卡不应在本局卡池")
        _expect(not ids.has("ammonite-token"), "本局图鉴不应包含 Token")
        _expect(not ids.has("aristotle"), "关联玩法未启用的中立卡不应在本局卡池")

    var count = instance.find_child("GalleryCount", true, false)
    _expect(
        count != null and count.text == tr("ui.gallery.count") % 198,
        "本局图鉴计数应为 198 张，实际：%s" % (count.text if count != null else "无"),
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


func _unmount(instance: Node) -> void:
    instance.queue_free()
    await get_tree().process_frame
