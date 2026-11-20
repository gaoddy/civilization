extends "res://tests/smoke_case.gd"
## 商店卡角标与冻结表现：费用在左上、时代在右上，冻结用整卡蒙层 + 冰晶 + 右下「冻」字。


func _ready() -> void:
    var view: Dictionary = FixtureScript.view()
    view["self"]["shop"] = [
        {"index": 0, "cardId": "aolinpike-jidian", "frozen": false},
        {"index": 1, "cardId": "agi", "frozen": true},
        {"index": 2, "cardId": "einstein-fridge", "frozen": false},
        {"index": 3, "cardId": "qishi", "frozen": false},
        {"index": 4, "cardId": "adam-smith", "frozen": false},
        {"index": 5, "cardId": "baiyi", "frozen": false},
    ]
    view["self"]["board"] = []
    view["self"]["affordances"]["shopSlots"] = [
        {"index": 0, "buyable": true, "disabledReason": null},
        {"index": 1, "buyable": false, "disabledReason": "NOT_ENOUGH_GOLD"},
        {"index": 2, "buyable": true, "disabledReason": null},
        {"index": 3, "buyable": true, "disabledReason": null},
        {"index": 4, "buyable": true, "disabledReason": null},
        {"index": 5, "buyable": true, "disabledReason": null},
    ]

    var recruit := GameRecruitView.new()
    add_child(recruit)
    # 独立挂载时没有外层三栏，给足宽度让 6 个商店槽保持同一行。
    recruit.size = Vector2(900, 700)
    var state := GameUiState.new()
    state.view = view
    state.self_state = view["self"]
    state.phase = Protocol.PHASE_RECRUIT
    state.board_slots = int(view.get("boardSlots", 7))
    recruit.refresh(state)
    await get_tree().process_frame
    await get_tree().process_frame

    # 未冻结卡：左上费用、右上时代，没有冻结蒙层。
    var open_card := recruit.find_child("ShopSlot0", true, false)
    _expect(open_card != null, "商店应存在 ShopSlot0")
    if open_card != null:
        _expect(
            (open_card as Control).custom_minimum_size == Vector2(112, 146),
            "商店卡应为 112×146，实际 %s" % (open_card as Control).custom_minimum_size
        )
        var cost := open_card.find_child("CostBadge", true, false)
        _expect(cost != null, "商店卡左上角应有 CostBadge")
        if cost != null:
            _expect(str(cost.text) == str(CardDB.display_cost("aolinpike-jidian")), "CostBadge 文本应为展示费用")
            _expect(str(cost.text) == "2", "CostBadge 应显示该卡展示费用 2，实际：%s" % str(cost.text))
        var era := open_card.find_child("EraBadge", true, false)
        _expect(era != null, "商店卡右上角应有 EraBadge")
        if era != null:
            _expect(
                str(era.text) == UI.enum_text("era", str(CardDB.card("aolinpike-jidian").get("era", ""))),
                "EraBadge 文本应为卡表时代的展示名",
            )
            _expect(
                str(era.text) == TranslationServer.translate("enum.era.classical"),
                "EraBadge 应显示时代文案，实际：%s" % str(era.text),
            )
        _expect(open_card.find_child("FreezeOverlay", true, false) == null, "未冻结卡不应有冻结蒙层")
        _expect(open_card.find_child("FreezeGlyph", true, false) == null, "未冻结卡不应有「冻」字")
        var name_node := open_card.find_child("CardName", true, false)
        _expect(name_node != null, "商店卡应有 CardName")
        # 卡面类型行显示全部玩法，多玩法用「、」连接。
        var expected_type := CardText.playstyle_text(CardDB.card("aolinpike-jidian"))
        var type_node := open_card.find_child("CardType", true, false)
        _expect(
            type_node != null and str(type_node.text) == expected_type,
            "卡面应显示全部玩法「%s」，实际：%s"
            % [expected_type, str(type_node.text) if type_node != null else "缺失"],
        )
        var cover_node := open_card.find_child("Cover", true, false)
        _expect(cover_node is Panel, "卡面封面应为可绘制圆角的面板")
        if cover_node is Panel:
            var expected_cover := UITheme.playstyle_cover(
                CardDB.primary_playstyle(CardDB.card("aolinpike-jidian"))
            )
            var cover_style := (cover_node as Panel).get_theme_stylebox("panel") as StyleBoxFlat
            _expect(
                cover_style != null and cover_style.bg_color == expected_cover,
                "卡面封面色应取首个玩法色",
            )
            _expect(
                cover_style != null
                and cover_style.corner_radius_top_left == UITheme.radius("radius-md"),
                "卡面封面应带卡片圆角",
            )
        var face_node := (open_card as Control).get_child(0) as Control
        _expect(
            face_node != null and face_node.size == Vector2(110, 144),
            "卡面应铺满卡片 1px 描边以内的区域，实际 %s"
            % (face_node.size if face_node != null else Vector2.ZERO),
        )
        if cost != null and era != null and name_node is Control:
            var card_rect: Rect2 = (open_card as Control).get_global_rect()
            var cost_rect: Rect2 = (cost as Control).get_global_rect()
            var era_rect: Rect2 = (era as Control).get_global_rect()
            var name_rect: Rect2 = (name_node as Control).get_global_rect()
            _expect(cost_rect.position.x < card_rect.get_center().x, "费用角标应在卡片左上")
            _expect(cost_rect.position.y < card_rect.get_center().y, "费用角标应在卡片左上")
            _expect(era_rect.position.x > card_rect.get_center().x, "时代角标应在卡片右上")
            _expect(era_rect.position.y < card_rect.get_center().y, "时代角标应在卡片右上")
            _expect(not cost_rect.intersects(era_rect), "费用与时代角标不应相交")
            _expect(not cost_rect.intersects(name_rect), "费用角标不应与卡名相交")
            _expect(not era_rect.intersects(name_rect), "时代角标不应与卡名相交")

    # 冻结卡：整卡蒙层 + 冰晶 + 右下「冻」字，且不再使用「冻」角标。
    var frozen_card := recruit.find_child("ShopSlot1", true, false)
    _expect(frozen_card != null, "商店应存在 ShopSlot1")
    if frozen_card != null:
        var overlay := frozen_card.find_child("FreezeOverlay", true, false)
        _expect(overlay != null, "冻结卡应有 FreezeOverlay 蒙层")
        if overlay is Control:
            _expect((overlay as Control).mouse_filter == Control.MOUSE_FILTER_IGNORE, "冻结蒙层不应拦截鼠标")
        if overlay is Panel:
            var overlay_style := (overlay as Panel).get_theme_stylebox("panel") as StyleBoxFlat
            _expect(
                overlay_style != null
                and overlay_style.bg_color == UITheme.color("color-info"),
                "冻结蒙层底色应取 info 色",
            )
            _expect(
                overlay_style != null
                and overlay_style.corner_radius_top_left == UITheme.radius("radius-md"),
                "冻结蒙层应带卡片圆角",
            )
            _expect(
                is_equal_approx((overlay as CanvasItem).self_modulate.a, UITheme.opacity("freeze-overlay-alpha")),
                "冻结蒙层 self_modulate alpha 应为 freeze-overlay-alpha 令牌值",
            )
        var crystals := frozen_card.find_child("FreezeCrystals", true, false)
        _expect(crystals != null, "冻结卡应有 FreezeCrystals 冰晶纹理")
        if crystals is Control:
            _expect(
                crystals.get_child_count() == CardFaceBase.FREEZE_CRYSTAL_LINES,
                "冰晶线条数应与常量一致，实际：%d" % crystals.get_child_count(),
            )
            if crystals.get_child_count() >= 2:
                var first := crystals.get_child(0) as Control
                var second := crystals.get_child(1) as Control
                _expect(
                    is_equal_approx(
                        second.position.y - first.position.y,
                        float(UITheme.metric("freeze-crystal-gap")),
                    ),
                    "相邻冰晶线 y 间距应为 freeze-crystal-gap 令牌值",
                )
                if first is ColorRect:
                    var line := first as ColorRect
                    _expect(
                        is_equal_approx(line.color.a, CardFaceBase.FREEZE_CRYSTAL_ALPHA),
                        "冰晶线 alpha 应为 FREEZE_CRYSTAL_ALPHA 常量值",
                    )
                    _expect(
                        is_equal_approx(line.modulate.a, 1.0) and is_equal_approx(line.self_modulate.a, 1.0),
                        "冰晶线不应被蒙层 modulate 压暗",
                    )
        var glyph := frozen_card.find_child("FreezeGlyph", true, false)
        _expect(glyph != null, "冻结卡应有 FreezeGlyph")
        if glyph != null:
            _expect(
                str(glyph.text) == TranslationServer.translate("ui.game.card.freeze_glyph"),
                "FreezeGlyph 文本应为冻结字形，实际：%s" % str(glyph.text),
            )
            _expect(
                is_equal_approx((glyph as CanvasItem).modulate.a, 1.0)
                and is_equal_approx((glyph as CanvasItem).self_modulate.a, 1.0),
                "「冻」字不应被蒙层 modulate 压暗",
            )
            var frozen_rect: Rect2 = (frozen_card as Control).get_global_rect()
            var glyph_rect: Rect2 = (glyph as Control).get_global_rect()
            _expect(
                glyph_rect.get_center().x > frozen_rect.get_center().x
                and glyph_rect.get_center().y > frozen_rect.get_center().y,
                "「冻」字应位于卡片右下象限",
            )
            _expect(frozen_rect.encloses(glyph_rect), "「冻」字应落在卡片内")
        var old_badge := false
        var freeze_glyph := TranslationServer.translate("ui.game.card.freeze_glyph")
        for badge in frozen_card.find_children("CardBadge", "Label", true, false):
            if str(badge.text) == freeze_glyph:
                old_badge = true
        _expect(not old_badge, "冻结卡不应再用 text 为「冻」的 CardBadge")

    # 随从卡费用固定为 3：商店里已渲染的随从卡也要显示费用角标。
    var minion_card := recruit.find_child("ShopSlot4", true, false)
    _expect(minion_card != null, "商店应存在 ShopSlot4")
    if minion_card != null:
        var minion_cost := minion_card.find_child("CostBadge", true, false)
        _expect(minion_cost != null, "随从卡应有 CostBadge")
        if minion_cost != null:
            _expect(str(minion_cost.text) == "3", "随从卡费用角标应为 3，实际：%s" % str(minion_cost.text))
        var minion_type := minion_card.find_child("CardType", true, false)
        _expect(
            minion_type != null and str(minion_type.text) == TranslationServer.translate("enum.playstyle.mammal"),
            "随从卡应显示玩法「哺乳纲」",
        )

    # 14 个玩法色互不重复，封面均由玩法色压暗得到。
    _expect(UITheme.PLAYSTYLE_COLOR.size() == 14, "玩法色表应有 14 个键")
    var used_colors := {}
    for playstyle in UITheme.PLAYSTYLE_COLOR:
        var tint := UITheme.playstyle_color(str(playstyle))
        _expect(not used_colors.has(tint.to_html()), "玩法色 %s 不应与其他玩法重复" % playstyle)
        used_colors[tint.to_html()] = true
        _expect(
            UITheme.playstyle_cover(str(playstyle)).v < tint.v,
            "玩法 %s 的封面色应比原色更暗" % playstyle,
        )
    _expect(used_colors.size() == 14, "14 个玩法色应全部可区分")

    # 商店卡不显示介绍：信息区统一退回紧凑高度，名称与数值仍被信息区包住。
    var shop_cases := [
        {"slot": 0, "card_id": "aolinpike-jidian"},
        {"slot": 2, "card_id": "einstein-fridge"},
        {"slot": 3, "card_id": "qishi"},
        {"slot": 4, "card_id": "adam-smith"},
    ]
    for case in shop_cases:
        var card: Control = recruit.find_child("ShopSlot%d" % int(case["slot"]), true, false)
        var card_id := str(case["card_id"])
        _expect(card != null, "商店应存在 %s" % card_id)
        if card == null:
            continue
        _expect(card.find_child("CardDesc", true, false) == null, "%s 不应显示介绍" % card_id)
        var info: Control = card.find_child("CardInfo", true, false)
        var name_label: Control = card.find_child("CardName", true, false)
        var stats: Control = card.find_child("CardStats", true, false)
        _expect(
            info != null and absf(info.size.y - 56.0) <= 1.0,
            "%s 的信息区应为紧凑高度 56，实际 %s"
            % [card_id, info.size.y if info != null else -1.0],
        )
        if info != null and name_label != null and stats != null:
            _expect(
                info.get_global_rect().encloses(name_label.get_global_rect())
                and info.get_global_rect().encloses(stats.get_global_rect()),
                "%s 的信息区应包住名称与数值" % card_id,
            )

    # 介绍仍用于图鉴与手牌：长介绍最多两行并省略。
    var gallery_tile := GalleryCardTile.new("adam-smith")
    add_child(gallery_tile)
    gallery_tile.size = Vector2(112, 146)
    await get_tree().process_frame
    await get_tree().process_frame
    var desc: Label = gallery_tile.find_child("CardDesc", true, false)
    _expect(desc != null and desc.is_visible_in_tree(), "图鉴卡应显示 CardDesc")
    if desc != null:
        _expect(
            str(desc.text) == CardDB.display_text("adam-smith"),
            "图鉴卡介绍应为卡表技能",
        )
        _expect(
            desc.autowrap_mode == TextServer.AUTOWRAP_WORD_SMART,
            "介绍应自动换行",
        )
        _expect(desc.max_lines_visible == 2, "介绍最多两行")
        _expect(
            desc.text_overrun_behavior == TextServer.OVERRUN_TRIM_ELLIPSIS,
            "超长介绍应省略",
        )
        _expect(desc.get_visible_line_count() <= 2, "介绍不应超过两行")
        _expect(desc.get_visible_line_count() == 2, "adam-smith 的长介绍应显示两行")
        var gallery_name: Control = gallery_tile.find_child("CardName", true, false)
        var gallery_info: Control = gallery_tile.find_child("CardInfo", true, false)
        if gallery_name != null and gallery_info != null:
            _expect(
                desc.get_global_rect().end.y <= gallery_name.get_global_rect().position.y + 1.0,
                "图鉴卡介绍应在名称上方",
            )
            _expect(
                gallery_info.get_global_rect().encloses(desc.get_global_rect()),
                "图鉴卡信息区应包住介绍",
            )
    gallery_tile.queue_free()
    await get_tree().process_frame

    # 关联分类行文案：界面直接展示该键，必须是「关联分类」而非旧的「关联玩法」。
    var association_label := str(TranslationServer.translate("ui.game.detail.association"))
    _expect(
        association_label == "关联分类",
        "关联分类行文案应为「关联分类」，实际：%s" % association_label,
    )

    # 详情分类行：科学分类完整路径、全部玩法、关联分类（仅 prey/host）。
    var mammal_detail := _shop_detail_text("jianchihu")
    _expect(
        mammal_detail.contains(TranslationServer.translate("ui.game.detail.taxonomy")),
        "随从详情应显示科学分类：%s" % mammal_detail,
    )
    _expect(
        mammal_detail.contains("动物界 › 脊索动物门 › 哺乳纲 › 食肉目 › 猫科"),
        "随从详情应显示完整分类路径：%s" % mammal_detail,
    )
    _expect(
        mammal_detail.contains(TranslationServer.translate("ui.game.detail.playstyle"))
        and mammal_detail.contains(TranslationServer.translate("enum.playstyle.mammal")),
        "随从详情应显示玩法：%s" % mammal_detail,
    )
    _expect(
        mammal_detail.contains(TranslationServer.translate("ui.game.detail.association"))
        and mammal_detail.contains(TranslationServer.translate("ui.game.detail.prey"))
        and mammal_detail.contains("偶蹄目"),
        "随从详情应显示捕食关联：%s" % mammal_detail,
    )
    _expect(
        not mammal_detail.contains(TranslationServer.translate("ui.game.detail.food_chain")),
        "关联分类已合并捕食/寄生，不应再显示食物链行：%s" % mammal_detail,
    )
    _expect(
        mammal_detail.contains(TranslationServer.translate("enum.card_type.minion")),
        "类型行应显示卡牌类别",
    )

    var tool_detail := _shop_detail_text("airplane")
    _expect(
        tool_detail.contains("造物 › 交通与运载"),
        "造物卡应显示技术分类路径：%s" % tool_detail,
    )
    _expect(
        tool_detail.contains(TranslationServer.translate("ui.game.detail.playstyle"))
        and tool_detail.contains(TranslationServer.translate("enum.playstyle.machine")),
        "机械工具卡应显示玩法：%s" % tool_detail,
    )
    _expect(
        not tool_detail.contains(TranslationServer.translate("ui.game.detail.association")),
        "没有 prey/host 的卡不应显示关联分类：%s" % tool_detail,
    )

    var scholar_detail := _shop_detail_text("alan-turing")
    _expect(
        scholar_detail.contains(TranslationServer.translate("ui.game.detail.playstyle"))
        and scholar_detail.contains(TranslationServer.translate("enum.playstyle.machine")),
        "学者详情应显示机械玩法：%s" % scholar_detail,
    )
    _expect(
        not scholar_detail.contains(TranslationServer.translate("enum.playstyle.neutral")),
        "学者详情不应再显示中立：%s" % scholar_detail,
    )
    _expect(
        not scholar_detail.contains(TranslationServer.translate("ui.game.detail.association")),
        "学者不应显示关联分类行：%s" % scholar_detail,
    )

    var hero_detail := _hero_detail_text("darwin")
    _expect(
        not hero_detail.contains(TranslationServer.translate("ui.game.detail.taxonomy")),
        "英雄详情不应显示科学分类：%s" % hero_detail,
    )
    _expect(
        not hero_detail.contains(TranslationServer.translate("ui.game.detail.era")),
        "英雄详情不应显示时代：%s" % hero_detail,
    )
    _expect(
        hero_detail.contains(TranslationServer.translate("ui.game.detail.playstyle"))
        and hero_detail.contains(TranslationServer.translate("enum.playstyle.mammal")),
        "英雄详情应显示全部玩法：%s" % hero_detail,
    )
    _expect(
        not hero_detail.contains(TranslationServer.translate("ui.game.detail.specialty")),
        "英雄详情不应再显示擅长玩法：%s" % hero_detail,
    )

    await _check_hero_skill_list()
    await _check_room_skill_list()

    var missing_detail := _classification_text({"playstyle": "", "taxon": "does-not-exist"})
    _expect(
        missing_detail.contains(TranslationServer.translate("ui.game.detail.undetermined")),
        "缺失分类应显示未定：%s" % missing_detail,
    )

    await _check_hand_fit()
    await _check_cover_art_inset()
    recruit.queue_free()
    _finish()


## 手牌介绍不被截断：信息区向上扩展，字号缩到能放下全文；介绍不压名称与角标。
func _check_hand_fit() -> void:
    _expect(
        UITheme.panel_style("raised-flush").content_margin_left == 1,
        "卡面样式只应让出 1px 描边",
    )
    var panel := await _hand_probe_card(112, 146)
    var info: Control = panel.find_child("CardInfo", true, false)
    var desc_box: Control = panel.find_child("CardDescBox", true, false)
    var desc: Label = panel.find_child("CardDesc", true, false)
    var name_label: Control = panel.find_child("CardName", true, false)
    var corners: Control = panel.find_child("CardCorners", true, false)
    _expect(info != null and info.size.y > 90.0, "手牌信息区应向封面扩展，实际 %s" % (info.size.y if info else -1.0))
    _expect(desc != null and desc.is_visible_in_tree(), "手牌卡应显示介绍")
    if desc != null and desc_box != null and name_label != null and corners != null:
        _expect(desc.max_lines_visible == -1, "手牌介绍不应限制行数")
        _expect(
            desc.text_overrun_behavior == TextServer.OVERRUN_NO_TRIMMING,
            "手牌介绍不应省略",
        )
        _expect(desc.get_theme_font_size("font_size") >= 8, "手牌介绍字号不应小于 8")
        _expect(desc.get_line_count() > 2, "最长介绍应超过旧的两行限制")
        _expect(
            desc.get_visible_line_count() == desc.get_line_count(),
            "手牌介绍应完整显示：可见 %d 行 / 共 %d 行"
            % [desc.get_visible_line_count(), desc.get_line_count()],
        )
        _expect(
            desc.get_minimum_size().y <= desc_box.size.y + 1.0,
            "介绍高度 %s 应落在承载框 %s 内" % [desc.get_minimum_size().y, desc_box.size.y],
        )
        _expect(
            desc_box.get_global_rect().end.y <= name_label.get_global_rect().position.y + 1.0,
            "介绍应在名称上方",
        )
        _expect(
            not desc_box.get_global_rect().intersects(corners.get_global_rect()),
            "介绍不应压住角标",
        )
        _expect(
            corners.get_global_rect().position.y < desc_box.get_global_rect().position.y,
            "角标应位于介绍上方",
        )
    panel.queue_free()
    await get_tree().process_frame

    # 紧凑卡放不下最长介绍：至少保证不溢出、不压名称，按可用行数截断。
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    var compact := await _hand_probe_card(84, 92)
    var compact_desc: Label = compact.find_child("CardDesc", true, false)
    var compact_box: Control = compact.find_child("CardDescBox", true, false)
    var compact_name: Control = compact.find_child("CardName", true, false)
    if compact_desc != null and compact_box != null and compact_name != null:
        _expect(
            compact_desc.get_visible_line_count() <= maxi(compact_desc.get_line_count(), 1),
            "紧凑手牌介绍行数不应超出文本",
        )
        _expect(
            compact_box.get_global_rect().end.y <= compact_name.get_global_rect().position.y + 1.0,
            "紧凑手牌介绍不应压住名称",
        )
        _expect(
            compact_desc.get_minimum_size().y <= compact_box.size.y + 1.0,
            "紧凑手牌介绍应在承载框内",
        )
    compact.queue_free()
    ScreenProfile.set_device_for_test("")
    await get_tree().process_frame


## 构建一张手牌卡并等待布局；`self` 作为文本测量宿主（非容器）。
func _hand_probe_card(width: int, height: int) -> PanelContainer:
    var panel := PanelContainer.new()
    panel.custom_minimum_size = Vector2(width, height)
    panel.size = Vector2(width, height)
    panel.add_theme_stylebox_override("panel", UITheme.panel_style("raised-flush"))
    add_child(panel)
    panel.add_child(HandCardFace.build({
        "cardId": "superbug",
        "attack": 2,
        "health": 3,
        "golden": false,
    }, self))
    await get_tree().process_frame
    await get_tree().process_frame
    return panel


## 有卡面美术时，方形美术向内收，四角落在卡片圆角以内。
func _check_cover_art_inset() -> void:
    if CardDB.art_path("zhenghe", "card_face") == "":
        return
    var panel := PanelContainer.new()
    panel.custom_minimum_size = Vector2(112, 146)
    panel.size = Vector2(112, 146)
    panel.add_theme_stylebox_override("panel", UITheme.panel_style("raised-flush"))
    add_child(panel)
    panel.add_child(ShopCardFace.build("zhenghe", CardDB.card("zhenghe"), false))
    await get_tree().process_frame
    var cover := panel.find_child("Cover", true, false)
    var art := panel.find_child("CoverArt", true, false) as TextureRect
    _expect(cover is Panel, "有美术的卡面封面仍应是圆角面板")
    _expect(art != null, "有卡面美术时应创建 CoverArt")
    if art != null:
        _expect(
            art.offset_left == UITheme.metric("card-cover-inset")
            and art.offset_top == UITheme.metric("card-cover-inset")
            and art.offset_right == -UITheme.metric("card-cover-inset")
            and art.offset_bottom == -UITheme.metric("card-cover-inset"),
            "卡面美术应向内收 %dpx，实际 %s"
            % [UITheme.metric("card-cover-inset"), str(art.offset_left)],
        )
    panel.queue_free()
    await get_tree().process_frame


## 英雄技能列表：逐条块状展示，无名技能不渲染标题，主动技能带费用与次数。
func _check_hero_skill_list() -> void:
    var detail := GalleryDetail.new()
    add_child(detail)
    detail.show_hero("darwin")
    await get_tree().process_frame
    await get_tree().process_frame

    var blocks := detail.find_children("SkillBlock*", "VBoxContainer", true, false)
    _expect(blocks.size() == 2, "图鉴英雄详情应逐条渲染 2 个技能块，实际：%d" % blocks.size())
    var text := _node_text(detail)
    _expect(
        text.contains(CardDB.skill_info("aura-biology-plus-1-plus-1").get("text", "")),
        "图鉴英雄详情应显示技能完整文本：%s" % text,
    )
    _expect(
        text.contains(TranslationServer.translate("ui.skill.trigger.aura")),
        "图鉴英雄详情应出现光环标签：%s" % text,
    )
    _expect(
        not text.contains(TranslationServer.translate("ui.gallery.skills")),
        "图鉴英雄详情不应再单独显示技能名称行：%s" % text,
    )

    if blocks.size() == 2:
        var unnamed: Control = blocks[0]
        _expect(
            unnamed.find_child("SkillName", true, false) == null,
            "无名称技能不应渲染技能名标题",
        )
        var aura := unnamed.find_child("SkillTrigger", true, false)
        _expect(
            aura != null and str(aura.text) == TranslationServer.translate("ui.skill.trigger.aura"),
            "无名称技能应带光环标签",
        )
        _expect(
            unnamed.find_child("SkillCost", true, false) == null,
            "被动技能不应显示费用与次数",
        )

        var active: Control = blocks[1]
        var active_info := CardDB.skill_info("evolution-discovery")
        var skill_name := active.find_child("SkillName", true, false)
        _expect(
            skill_name != null and str(skill_name.text) == str(active_info.get("name", "")),
            "有名称技能应渲染技能名标题",
        )
        var trigger := active.find_child("SkillTrigger", true, false)
        _expect(
            trigger != null
            and str(trigger.text) == TranslationServer.translate("ui.skill.trigger.on_activate"),
            "主动技能应带主动标签",
        )
        var cost := active.find_child("SkillCost", true, false)
        var expected_cost := "%s · %s" % [
            TranslationServer.translate("ui.skill.cost") % int(active_info.get("cost", -1)),
            TranslationServer.translate("ui.skill.uses") % int(active_info.get("uses", -1)),
        ]
        _expect(
            cost != null and str(cost.text) == expected_cost,
            "主动技能应显示「%s」，实际：%s" % [expected_cost, str(cost.text) if cost != null else "缺失"],
        )
    detail.queue_free()
    await get_tree().process_frame

    # 无技能时显示空态，不渲染空标签或费用行。
    var empty_box := VBoxContainer.new()
    add_child(empty_box)
    UI.skill_list(empty_box, [])
    await get_tree().process_frame
    _expect(
        _node_text(empty_box).contains(TranslationServer.translate("ui.skill.empty")),
        "无技能时应显示空态文案",
    )
    empty_box.queue_free()
    await get_tree().process_frame


## 选英雄页技能浮层：与图鉴共用技能列表，主动技能显示费用与次数。
func _check_room_skill_list() -> void:
    GameStore.player_id = "p0"
    GameStore.room_id = "r_test"
    GameStore.room_code = "ABC123"
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "ready": false, "isHost": true,
         "heroCardId": null, "heroCandidates": ["gauss"]},
    ]
    var instance := await _check_scene("res://scenes/room.tscn", false)
    if instance != null:
        var candidates := instance.find_child("HeroCandidates", true, false)
        _expect(candidates != null and candidates.get_child_count() > 0, "选英雄页应渲染候选英雄")
        if candidates != null and candidates.get_child_count() > 0:
            var event := InputEventMouseButton.new()
            event.button_index = MOUSE_BUTTON_LEFT
            event.pressed = true
            candidates.get_child(0).emit_signal("gui_input", event)
            await get_tree().process_frame
            var panel := instance.find_child("SkillPanel", true, false)
            var text := _node_text(panel)
            var info := CardDB.skill_info("scholar-growth-twice")
            _expect(
                text.contains(TranslationServer.translate("ui.skill.trigger.on_activate")),
                "技能浮层应出现主动标签：%s" % text,
            )
            _expect(
                text.contains(TranslationServer.translate("ui.skill.cost") % int(info.get("cost", -1))),
                "技能浮层应显示费用：%s" % text,
            )
            _expect(
                text.contains(TranslationServer.translate("ui.skill.uses") % int(info.get("uses", -1))),
                "技能浮层应显示每回合次数：%s" % text,
            )
        instance.queue_free()
        await get_tree().process_frame
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.room_players = []


## 商店/手牌详情正文：直接走详情渲染，避免依赖具体页面的悬停时机。
func _shop_detail_text(card_id: String) -> String:
    var layer := GameDetailLayer.new()
    var box := VBoxContainer.new()
    layer._render_shop_detail(box, {"cardId": card_id})
    var text := _node_text(box)
    box.free()
    layer.free()
    return text


func _hero_detail_text(hero_id: String) -> String:
    var layer := GameDetailLayer.new()
    var box := VBoxContainer.new()
    layer._render_hero_detail(box, hero_id)
    var text := _node_text(box)
    box.free()
    layer.free()
    return text


func _classification_text(entry: Dictionary) -> String:
    var layer := GameDetailLayer.new()
    var box := VBoxContainer.new()
    layer._append_classification(box, entry)
    var text := _node_text(box)
    box.free()
    layer.free()
    return text
