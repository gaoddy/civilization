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
        # 卡面类型行固定显示玩法；效果牌与学者都显示「中立」。
        var type_node := open_card.find_child("CardType", true, false)
        _expect(
            type_node != null and str(type_node.text) == TranslationServer.translate("enum.playstyle.neutral"),
            "卡面应显示玩法「中立」，实际：%s" % (str(type_node.text) if type_node != null else "缺失"),
        )
        var cover_node := open_card.find_child("Cover", true, false)
        _expect(
            cover_node is ColorRect
            and (cover_node as ColorRect).color == UITheme.playstyle_cover("neutral"),
            "卡面封面色应取中立玩法色",
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
        if overlay is ColorRect:
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

    # 详情分类行：科学分类完整路径、玩法、关联玩法、食物链，缺失字段不显示。
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
        mammal_detail.contains(TranslationServer.translate("ui.game.detail.playstyle")),
        "随从详情应显示玩法",
    )
    _expect(
        mammal_detail.contains(TranslationServer.translate("ui.game.detail.food_chain"))
        and mammal_detail.contains(TranslationServer.translate("ui.game.detail.prey"))
        and mammal_detail.contains("偶蹄目"),
        "随从详情应显示捕食关系：%s" % mammal_detail,
    )
    _expect(
        not mammal_detail.contains(TranslationServer.translate("ui.game.detail.association")),
        "非中立玩法不应显示关联玩法",
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
        tool_detail.contains(TranslationServer.translate("ui.game.detail.association"))
        and tool_detail.contains(TranslationServer.translate("enum.playstyle.machine")),
        "中立工具卡应显示关联玩法：%s" % tool_detail,
    )
    _expect(
        not tool_detail.contains(TranslationServer.translate("ui.game.detail.food_chain")),
        "无捕食/寄生的卡不应显示食物链",
    )

    var scholar_detail := _shop_detail_text("alan-turing")
    _expect(
        scholar_detail.contains(TranslationServer.translate("enum.playstyle.neutral")),
        "学者详情玩法应为中立",
    )
    _expect(
        not scholar_detail.contains(TranslationServer.translate("ui.game.detail.association")),
        "学者不应显示关联玩法行",
    )

    var hero_detail := _hero_detail_text("darwin")
    _expect(
        hero_detail.contains(TranslationServer.translate("ui.game.detail.taxonomy"))
        and hero_detail.contains("人科"),
        "英雄详情应显示科学分类：%s" % hero_detail,
    )
    _expect(
        hero_detail.contains(TranslationServer.translate("ui.game.detail.playstyle"))
        and hero_detail.contains(TranslationServer.translate("enum.playstyle.neutral")),
        "英雄详情应显示玩法",
    )
    _expect(
        hero_detail.contains(TranslationServer.translate("ui.game.detail.specialty"))
        and hero_detail.contains(TranslationServer.translate("enum.playstyle.mammal")),
        "英雄详情应显示擅长玩法",
    )

    var missing_detail := _classification_text({"playstyle": "", "taxon": "does-not-exist"})
    _expect(
        missing_detail.contains(TranslationServer.translate("ui.game.detail.undetermined")),
        "缺失分类应显示未定：%s" % missing_detail,
    )

    recruit.queue_free()
    _finish()


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
