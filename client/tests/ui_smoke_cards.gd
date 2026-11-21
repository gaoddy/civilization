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
        var era := open_card.find_child("EraGlyph", true, false)
        _expect(era is EraGlyph, "商店卡右上角应有 EraGlyph")
        if era is EraGlyph:
            _expect(
                (era as EraGlyph).era_id == str(CardDB.card("aolinpike-jidian").get("era", "")),
                "时代象形应取卡表 era id",
            )
            _expect(
                (era as EraGlyph).era_id == "classical",
                "时代象形应取该卡的古典时代，实际：%s" % (era as EraGlyph).era_id,
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
        var frame := open_card.find_child("CardFrame", true, false)
        _expect(frame is CardFrame, "卡面应有 CardFrame")
        if frame is CardFrame:
            var level := int(CardDB.card("aolinpike-jidian").get("level", 1))
            _expect((frame as CardFrame).level == level, "卡框星级应取卡表 level")
            _expect((frame as CardFrame).star_count == level, "卡框星数应等于 level")
            _expect(
                (frame as Control).size == (face_node as Control).size,
                "卡框应铺满卡面",
            )
            var corners := open_card.find_child("CardCorners", true, false)
            _expect(
                corners is Control and (frame as Control).get_index() < (corners as Control).get_index(),
                "卡框应绘制在角标之下",
            )
        _expect(open_card.find_child("AttackPlate", true, false) is StatPlate, "卡面应有攻击底牌")
        _expect(open_card.find_child("HealthPlate", true, false) is StatPlate, "卡面应有血量底牌")

    # 英雄卡：无星级、无攻防底牌、无时代象形、无角标与技能文本。
    var hero_face := GalleryCardFace.build_hero("adam-smith", CardDB.hero("adam-smith"))
    add_child(hero_face)
    await get_tree().process_frame
    _expect(hero_face.find_child("CardFrame", true, false) == null, "英雄卡不应有卡框星级")
    _expect(hero_face.find_child("AttackPlate", true, false) == null, "英雄卡不应有攻击底牌")
    _expect(hero_face.find_child("EraGlyph", true, false) == null, "英雄卡不应有时代象形")
    _expect(hero_face.find_child("CardBadge", true, false) == null, "英雄卡不应有角标")
    _expect(hero_face.find_child("CardDesc", true, false) == null, "英雄卡不应显示技能文本")
    hero_face.queue_free()

    # 棋盘卡：有卡框，无时代象形；空身材不出现底牌装饰（文字仍为「—」）。
    var board_face := BoardCardFace.build({
        "cardId": "aolinpike-jidian", "attack": 2, "health": 3,
        "golden": false, "stealth": false,
    })
    add_child(board_face)
    await get_tree().process_frame
    _expect(board_face.find_child("CardFrame", true, false) is CardFrame, "棋盘卡应有卡框")
    _expect(board_face.find_child("EraGlyph", true, false) == null, "棋盘卡不应有时代象形")
    var effect_plate := StatPlate.create(StatPlate.ATTACK, "—", UITheme.font_size("font-title"), "CardAttack")
    _expect(effect_plate.get_child(0).text == "—", "效果牌身材文字应保持「—」")
    effect_plate.free()
    board_face.queue_free()

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

    # 图鉴卡显示玩法、不显示效果文本；信息区为紧凑高度。
    var gallery_tile := GalleryCardTile.new("adam-smith")
    add_child(gallery_tile)
    gallery_tile.size = Vector2(112, 146)
    await get_tree().process_frame
    await get_tree().process_frame
    var gallery_type: Control = gallery_tile.find_child("CardType", true, false)
    var expected_playstyle := CardText.playstyle_text(CardDB.card("adam-smith"))
    _expect(
        gallery_type != null and str(gallery_type.text) == expected_playstyle,
        "图鉴卡应显示玩法「%s」，实际：%s"
        % [expected_playstyle, str(gallery_type.text) if gallery_type != null else "缺失"],
    )
    _expect(gallery_tile.find_child("CardDesc", true, false) == null, "图鉴卡不应显示效果文本")
    var gallery_face := gallery_tile.find_child("CardFace", true, false) as Control
    _expect(
        gallery_face != null
        and absf(gallery_face.size.x - (gallery_tile.size.x - 2.0)) <= 1.0
        and absf(gallery_face.size.y - (gallery_tile.size.y - 2.0)) <= 1.0,
        "图鉴卡面应铺满卡片内容区：卡 %s，卡面 %s"
        % [str(gallery_tile.size), str(gallery_face.size) if gallery_face != null else "缺失"],
    )
    gallery_tile.queue_free()
    await get_tree().process_frame

    # 可选项框内联变体：选中为 2px 强调描边并优先于悬停，悬停为 1px 强描边。
    var selectable := SelectableBox.new()
    selectable.configure(UITheme.panel_style("raised-flush"), SelectableBox.Outline.INLINE)
    selectable.custom_minimum_size = Vector2(112, 146)
    add_child(selectable)
    await get_tree().process_frame
    selectable.set_selected(true)
    var selected_box := selectable.get_theme_stylebox("panel") as StyleBoxFlat
    _expect(
        selected_box != null
        and selected_box.border_width_left == 2
        and selected_box.border_color == UITheme.color("color-accent"),
        "可选项框选中应为 2px 强调描边",
    )
    selectable.set_hovered(true)
    var selected_hover := selectable.get_theme_stylebox("panel") as StyleBoxFlat
    _expect(
        selected_hover != null and selected_hover.border_color == UITheme.color("color-accent"),
        "可选项框选中态应优先于悬停",
    )
    selectable.set_selected(false)
    var hover_box := selectable.get_theme_stylebox("panel") as StyleBoxFlat
    _expect(
        hover_box != null
        and hover_box.border_width_left == 1
        and hover_box.border_color == UITheme.color("color-border-strong"),
        "可选项框悬停应为 1px 强描边",
    )
    selectable.queue_free()
    await get_tree().process_frame

    # 可选项框外扩变体：选中显示框外一圈，且范围大于框本身。
    var outer := SelectableBox.new()
    var outer_base := UITheme.panel_style("raised")
    # 外扩选中框以内容区为基准；内容铺满整卡时使用零内边距底板。
    outer_base.set_content_margin_all(0)
    outer.configure(outer_base, SelectableBox.Outline.OUTER)
    outer.custom_minimum_size = Vector2(112, 146)
    add_child(outer)
    await get_tree().process_frame
    await get_tree().process_frame
    outer.set_selected(true)
    var outer_frame := outer.find_child("SelectedFrame", true, false) as Control
    _expect(
        outer_frame != null and outer_frame.is_visible_in_tree(),
        "外扩可选项框选中应显示选中框",
    )
    _expect(
        outer_frame != null
        and outer_frame.get_global_rect().size.x > outer.get_global_rect().size.x
        and outer_frame.get_global_rect().size.y > outer.get_global_rect().size.y,
        "外扩可选项框的选中框应大于框本身：框 %s，选中框 %s" % [
            str(outer.get_global_rect()), str(outer_frame.get_global_rect()) if outer_frame != null else "缺失",
        ],
    )
    outer.queue_free()
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

    var machine_detail := _shop_detail_text("alan-turing")
    _expect(
        machine_detail.contains(TranslationServer.translate("ui.game.detail.playstyle"))
        and machine_detail.contains(TranslationServer.translate("enum.playstyle.machine")),
        "alan-turing 详情应显示机械玩法：%s" % machine_detail,
    )
    _expect(
        not machine_detail.contains(TranslationServer.translate("enum.playstyle.neutral")),
        "alan-turing 详情不应显示中立：%s" % machine_detail,
    )
    _expect(
        not machine_detail.contains(TranslationServer.translate("ui.game.detail.association")),
        "alan-turing 不应显示关联分类行：%s" % machine_detail,
    )

    _check_hero_detail_art()

    var hero_detail := _hero_detail_text("darwin")
    _expect(
        hero_detail.contains(CardDB.hero_introduction("darwin")),
        "英雄详情应显示介绍：%s" % hero_detail,
    )
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
    await _check_skill_trigger_labels()
    await _check_room_skill_list()

    var missing_detail := _classification_text({"playstyle": "", "taxon": "does-not-exist"})
    _expect(
        missing_detail.contains(TranslationServer.translate("ui.game.detail.undetermined")),
        "缺失分类应显示未定：%s" % missing_detail,
    )

    _check_minion_gained_keywords()

    await _check_hand_fit()
    await _check_cover_art_inset()
    await _check_card_frame()
    await _check_stat_plate()
    await _check_era_glyph()
    await _check_avatar_frame()
    _check_page_background()
    recruit.queue_free()
    _finish()


## 卡框独立可建：记录星级、星数等于星级、不拦截鼠标。
func _check_card_frame() -> void:
    var frame := CardFrame.create(5)
    frame.name = "StandaloneFrame"
    add_child(frame)
    frame.set_anchors_preset(Control.PRESET_TOP_LEFT)
    frame.size = Vector2(112, 146)
    await get_tree().process_frame
    _expect(frame.level == 5, "CardFrame 应记录星级")
    _expect(frame.star_count == 5, "CardFrame 星数应等于星级")
    _expect(frame.mouse_filter == Control.MOUSE_FILTER_IGNORE, "卡框不应拦截鼠标")
    frame.queue_free()


## 攻防底牌：记录类型、尺寸随字号、内含带描边的数字。
func _check_stat_plate() -> void:
    for kind in ["attack", "health"]:
        var plate := StatPlate.create(kind, "4", 14, "Card" + kind.capitalize())
        add_child(plate)
        await get_tree().process_frame
        _expect(plate.kind == kind, "底牌应记录类型")
        _expect(plate.custom_minimum_size == Vector2(18, 18), "14 号字底牌应为 18×18")
        var label := plate.find_child("Card" + kind.capitalize(), true, false)
        _expect(label is Label and (label as Label).text == "4", "底牌应包含数字 Label")
        _expect(
            (label as Label).get_theme_constant("outline_size") >= 1,
            "数字应有描边",
        )
        for sample in ["", "—", " "]:
            var blank := StatPlate.create(kind, sample, 14)
            _expect(not blank.draws_chrome(), "空值「%s」不应画底牌" % sample)
            blank.free()
        var numeric := StatPlate.create(kind, "-1", 14)
        _expect(numeric.draws_chrome(), "数字应画底牌")
        numeric.free()
        plate.queue_free()


## 时代象形：七个时代都能建，记录 id、按尺寸占位、不拦截鼠标。
func _check_era_glyph() -> void:
    for era in ["prehistoric", "primitive", "classical", "feudal", "industrial", "modern", "future"]:
        var glyph := EraGlyph.create(era, 14)
        add_child(glyph)
        await get_tree().process_frame
        _expect(glyph.era_id == era, "时代象形应记录 era id")
        _expect(glyph.custom_minimum_size == Vector2(14, 14), "卡面象形应为 14×14")
        _expect(glyph.mouse_filter == Control.MOUSE_FILTER_IGNORE, "象形不应拦截鼠标")
        glyph.queue_free()


## 页面背景：纹理已导入、节点名固定、不拦截鼠标。
func _check_page_background() -> void:
    _expect(
        ResourceLoader.exists("res://assets/ui/bg/bg_main.png"),
        "页面背景纹理应已导入",
    )
    var background := UIControls.background()
    _expect(background.name == "PageBackground", "背景节点名应为 PageBackground")
    _expect(background.mouse_filter == Control.MOUSE_FILTER_IGNORE, "背景不应拦截鼠标")
    _expect(background is TextureRect, "有纹理时应使用 TextureRect")
    if background is TextureRect:
        var rect := background as TextureRect
        _expect(rect.expand_mode == TextureRect.EXPAND_IGNORE_SIZE, "背景应忽略纹理原始尺寸")
        _expect(
            rect.stretch_mode == TextureRect.STRETCH_KEEP_ASPECT_COVERED,
            "背景应按等比裁切铺满视口",
        )
    background.free()


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
    SkillList.build(empty_box, [])
    await get_tree().process_frame
    _expect(
        _node_text(empty_box).contains(TranslationServer.translate("ui.skill.empty")),
        "无技能时应显示空态文案",
    )
    empty_box.queue_free()
    await get_tree().process_frame


## 触发标签：空触发 + 关键词显示「关键词」；未知触发显示「未知」，不冒充常驻。
## 全部技能 id 无效时 skill_list 显示空态，不渲染空块。
func _check_skill_trigger_labels() -> void:
    var keyword_info := CardDB.skill_info("keyword-armor-1")
    _expect(
        str(keyword_info.get("trigger_label", "")) == TranslationServer.translate("ui.skill.trigger.keyword"),
        "空触发 + 关键词应显示关键词标签，实际：%s" % str(keyword_info.get("trigger_label", "")),
    )
    _expect(
        CardDB.trigger_name("smoke-unknown-trigger") == TranslationServer.translate("ui.skill.trigger.unknown"),
        "未知触发应显示未知标签",
    )

    CardDB.skills["smoke-unknown-trigger"] = {
        "id": "smoke-unknown-trigger",
        "effects": [{"trigger": "smoke-unknown-trigger"}],
    }
    var box := VBoxContainer.new()
    add_child(box)
    SkillList.build(box, ["smoke-unknown-trigger"])
    await get_tree().process_frame
    var text := _node_text(box)
    _expect(
        text.contains(TranslationServer.translate("ui.skill.trigger.unknown")),
        "未知触发应渲染未知标签：%s" % text,
    )
    _expect(
        not text.contains(TranslationServer.translate("ui.skill.trigger.passive")),
        "未知触发不应冒充常驻：%s" % text,
    )
    _expect(
        box.find_children("SkillBlock*", "VBoxContainer", true, false).size() == 1,
        "未知触发应渲染一个技能块",
    )
    box.queue_free()
    CardDB.skills.erase("smoke-unknown-trigger")
    await get_tree().process_frame

    var invalid_box := VBoxContainer.new()
    add_child(invalid_box)
    SkillList.build(invalid_box, ["does-not-exist"])
    await get_tree().process_frame
    _expect(
        _node_text(invalid_box).contains(TranslationServer.translate("ui.skill.empty")),
        "全部技能 id 无效时应显示空态",
    )
    _expect(
        invalid_box.find_children("SkillBlock*", "VBoxContainer", true, false).is_empty(),
        "无效技能 id 不应渲染空块",
    )
    invalid_box.queue_free()
    await get_tree().process_frame


## 选英雄页悬停详情：与图鉴共用技能列表，主动技能显示费用与次数。
func _check_room_skill_list() -> void:
    GameStore.player_id = "p0"
    GameStore.room_id = "r_test"
    GameStore.room_code = "ABC123"
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.room_players = [
        {"playerId": "p0", "displayName": "玩家A", "ready": false, "isHost": true,
         "heroCardId": null, "heroCandidates": ["gauss", "mozi", "luban", "shenkuo"]},
    ]
    var instance := await _check_scene("res://pages/room/room.tscn", false)
    if instance != null:
        var candidates := instance.find_child("HeroCandidates", true, false)
        _expect(candidates != null and candidates.get_child_count() > 0, "选英雄页应渲染候选英雄")
        if candidates != null and candidates.get_child_count() > 0:
            var card: Control = candidates.get_child(0)
            _expect(
                card.custom_minimum_size == Vector2(160, 209),
                "候选英雄卡应为 160×209，实际 %s" % card.custom_minimum_size,
            )
            await _hover(instance, card, card.size * 0.5)
            var panel := instance.find_child("RoomHeroDetail", true, false)
            _expect(panel != null and panel.visible, "悬停候选卡应显示详情卡")
            if panel != null:
                var text := _node_text(panel)
                var info := CardDB.skill_info("activate-friendly-growth-twice")
                _expect(
                    text.contains(CardDB.hero_introduction("gauss")),
                    "悬停详情应显示英雄介绍：%s" % text,
                )
                _expect(
                    text.contains(TranslationServer.translate("ui.skill.trigger.on_activate")),
                    "悬停详情应出现主动标签：%s" % text,
                )
                _expect(
                    text.contains(TranslationServer.translate("ui.skill.cost") % int(info.get("cost", -1))),
                    "悬停详情应显示费用：%s" % text,
                )
                _expect(
                    text.contains(TranslationServer.translate("ui.skill.uses") % int(info.get("uses", -1))),
                    "悬停详情应显示每回合次数：%s" % text,
                )
                _expect(
                    panel.find_children("*", "TextureRect", true, false).is_empty(),
                    "悬停详情不应显示任何图片",
                )
                # 中间候选卡：详情贴着该卡外侧，不再挂候选行外侧。
                var third: Control = candidates.get_child(2)
                await _hover(instance, third, third.size * 0.5)
                var panel_rect: Rect2 = panel.get_global_rect()
                var card_rect: Rect2 = third.get_global_rect()
                _expect(not panel_rect.intersects(card_rect), "详情卡不得压住悬停的候选卡")
                var gap := maxf(
                    card_rect.position.x - panel_rect.end.x,
                    panel_rect.position.x - card_rect.end.x,
                )
                _expect(
                    gap <= UITheme.space("space-md"),
                    "详情卡应贴着悬停卡外侧，实际间隙 %s" % gap,
                )
        instance.queue_free()
        await get_tree().process_frame
    GameStore.room_id = ""
    GameStore.room_code = ""
    GameStore.room_players = []


## 商店/手牌详情正文：直接走详情渲染，避免依赖具体页面的悬停时机。
func _shop_detail_text(card_id: String) -> String:
    var layer := DetailLayer.new()
    var box := VBoxContainer.new()
    layer._render_shop_detail(box, {"cardId": card_id})
    var text := _node_text(box)
    box.free()
    layer.free()
    return text


## 随从获得的关键词：摘要按固定顺序，之后逐条给出规则说明；卡面已有词不重复显示。
func _check_minion_gained_keywords() -> void:
    var gained_text := _minion_detail_text("aristotle", ["armor 2", "taunt"])
    var summary := _minion_gained_summary("aristotle", ["armor 2", "taunt"])
    _expect(
        summary.contains("嘲讽、减伤 2"),
        "获得行应按固定顺序摘要：%s" % summary,
    )
    _expect(
        gained_text.contains("嘲讽：敌方攻击只能在嘲讽随从中选择"),
        "嘲讽应给出规则说明：%s" % gained_text,
    )
    _expect(
        gained_text.contains("减伤 2：受到伤害时，减免 2 点伤害"),
        "减伤层数应代入说明：%s" % gained_text,
    )

    # 卡面自带的关键词不进入获得行；没有获得项时整行不显示。
    _expect(
        _minion_gained_summary("bruce-lee", ["windfury"]) == "",
        "卡面已有连击不应重复进入获得行",
    )

    # 层数增加按完整字符串视为获得：卡面 armor 1，当前 armor 2；基础剧毒不重复。
    var layered := _minion_gained_summary("box-jellyfish", ["poison", "armor 2"])
    _expect(
        layered.contains("减伤 2") and not layered.contains("剧毒"),
        "层数增加应进入获得行，卡面已有剧毒不应重复：%s" % layered,
    )


func _minion_detail_text(card_id: String, keywords: Array) -> String:
    var layer := DetailLayer.new()
    var box := VBoxContainer.new()
    layer._render_minion_detail(box, _minion_fixture(card_id, keywords))
    var text := _node_text(box)
    box.free()
    layer.free()
    return text


func _minion_gained_summary(card_id: String, keywords: Array) -> String:
    var layer := DetailLayer.new()
    var box := VBoxContainer.new()
    layer._render_minion_detail(box, _minion_fixture(card_id, keywords))
    var row := box.find_child("DetailGainedKeywords", true, false)
    var text := "" if row == null else _node_text(row)
    box.free()
    layer.free()
    return text


func _minion_fixture(card_id: String, keywords: Array) -> Dictionary:
    return {
        "cardId": card_id,
        "keywords": keywords,
        "attack": 3,
        "health": 5,
    }


## 对局英雄详情：顶部使用卡面美术，按 112:146 预留高度。
func _check_hero_detail_art() -> void:
    var layer := DetailLayer.new()
    var box := VBoxContainer.new()
    layer._render_hero_detail(box, "gauss")
    var art := box.find_child("HeroDetailArt", true, false) as TextureRect
    _expect(art != null, "英雄详情应有 HeroDetailArt 卡面美术")
    if art != null:
        _expect(
            art.texture != null and art.texture.resource_path.contains("cards/faces"),
            "英雄详情应使用卡面美术，实际：%s"
            % (art.texture.resource_path if art.texture != null else "无纹理"),
        )
        var size := art.custom_minimum_size
        _expect(
            size.x > 0.0 and absf(size.y - size.x * 146.0 / 112.0) <= 1.0,
            "英雄详情美术宽高比应为 112:146，实际 %s" % size,
        )
    box.free()
    layer.free()


func _hero_detail_text(hero_id: String) -> String:
    var layer := DetailLayer.new()
    var box := VBoxContainer.new()
    layer._render_hero_detail(box, hero_id)
    var text := _node_text(box)
    box.free()
    layer.free()
    return text


func _classification_text(entry: Dictionary) -> String:
    var box := VBoxContainer.new()
    DetailKit.new(200).append_classification(box, entry)
    var text := _node_text(box)
    box.free()
    return text


## 头像框：头像按圆角裁切，描边叠在头像之上，无图回退首字，内容叠层在描边之上。
func _check_avatar_frame() -> void:
    var frame := AvatarFrame.new()
    frame.custom_minimum_size = Vector2(84, 84)
    frame.size = Vector2(84, 84)
    add_child(frame)
    frame.configure(UITheme.playstyle_cover("mammal"), UITheme.playstyle_color("mammal"))
    frame.set_glyph("达", UITheme.alpha("color-text", AvatarFrame.GLYPH_ALPHA))
    await get_tree().process_frame

    var clip := frame.find_child("AvatarFrameClip", true, false) as CanvasItem
    var border := frame.find_child("AvatarFrameBorder", true, false) as Control
    _expect(clip != null, "头像框应有裁切层")
    _expect(
        clip != null and clip.clip_children == CanvasItem.CLIP_CHILDREN_AND_DRAW,
        "头像框裁切层应同时绘制封面并裁切子节点",
    )
    var cover: StyleBoxFlat = null
    if clip != null:
        cover = clip.get_theme_stylebox("panel") as StyleBoxFlat
    _expect(
        cover != null and cover.bg_color.a > 0.0,
        "头像框裁切层应绘制不透明封面",
    )
    _expect(
        border != null and clip != null and border.get_index() > clip.get_index(),
        "圆角描边应叠在裁切层之上",
    )
    _expect(
        frame.content() != null and border != null and frame.content().get_index() > border.get_index(),
        "内容叠层应位于描边之上",
    )
    _expect(frame.find_child("AvatarFrameArt", true, false) == null, "无素材时不应挂头像图")
    var glyph := frame.find_child("AvatarFrameGlyph", true, false) as Label
    _expect(glyph != null and glyph.visible, "无素材时应显示首字")

    frame.set_art(CardDB.art_path("darwin", "avatar"))
    await get_tree().process_frame
    var art := frame.find_child("AvatarFrameArt", true, false) as TextureRect
    _expect(art != null and art.visible, "有素材时应挂载头像图")
    _expect(
        art != null and art.stretch_mode == TextureRect.STRETCH_KEEP_ASPECT_COVERED,
        "头像图应按 cover 铺满",
    )
    _expect(art != null and clip != null and art.get_parent() == clip, "头像图应位于裁切层内")
    _expect(glyph != null and not glyph.visible, "有素材时应隐藏首字")

    frame.queue_free()
    await get_tree().process_frame
