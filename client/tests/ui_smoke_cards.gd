extends "res://tests/smoke_case.gd"
## 商店卡角标与冻结表现：费用在左上、时代在右上，冻结用整卡蒙层 + 冰晶 + 右下「冻」字。


func _ready() -> void:
    var view: Dictionary = FixtureScript.view()
    view["self"]["shop"] = [
        {"index": 0, "cardId": "aolinpike-jidian", "frozen": false},
        {"index": 1, "cardId": "agi", "frozen": true},
        {"index": 2, "cardId": "tufu", "frozen": false},
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
                crystals.get_child_count() == GameCardFace.FREEZE_CRYSTAL_LINES,
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
                        is_equal_approx(line.color.a, GameCardFace.FREEZE_CRYSTAL_ALPHA),
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

    # 卡面介绍：位于名称上方，最多两行；无介绍时信息区退回紧凑高度。
    var intro_cases := [
        {"slot": 0, "card_id": "aolinpike-jidian"},
        {"slot": 4, "card_id": "adam-smith"},
        {"slot": 3, "card_id": "qishi"},
        {"slot": 5, "card_id": "baiyi"},
    ]
    for case in intro_cases:
        var card: Control = recruit.find_child("ShopSlot%d" % int(case["slot"]), true, false)
        var card_id := str(case["card_id"])
        _expect(card != null, "商店应存在 %s" % card_id)
        if card == null:
            continue
        var desc: Label = card.find_child("CardDesc", true, false)
        var name_label: Label = card.find_child("CardName", true, false)
        var info: Control = card.find_child("CardInfo", true, false)
        var stats: Control = card.find_child("CardStats", true, false)
        _expect(desc != null and desc.is_visible_in_tree(), "%s 应显示 CardDesc" % card_id)
        _expect(
            desc != null and str(desc.text) == CardDB.display_text(card_id),
            "%s 的介绍文本应为卡表技能" % card_id,
        )
        if desc != null:
            _expect(
                desc.autowrap_mode == TextServer.AUTOWRAP_WORD_SMART,
                "%s 的介绍应自动换行" % card_id,
            )
            _expect(desc.max_lines_visible == 2, "%s 的介绍最多两行" % card_id)
            _expect(
                desc.text_overrun_behavior == TextServer.OVERRUN_TRIM_ELLIPSIS,
                "%s 的介绍超长应省略" % card_id,
            )
            _expect(desc.get_visible_line_count() <= 2, "%s 的介绍不应超过两行" % card_id)
            if card_id in ["adam-smith", "baiyi"]:
                _expect(desc.get_visible_line_count() == 2, "%s 的长介绍应显示两行" % card_id)
        _expect(stats != null, "%s 应有 CardStats" % card_id)
        if desc != null and name_label != null and info != null and stats != null:
            var desc_rect: Rect2 = desc.get_global_rect()
            var name_rect: Rect2 = name_label.get_global_rect()
            var info_rect: Rect2 = info.get_global_rect()
            var stats_rect: Rect2 = stats.get_global_rect()
            _expect(desc_rect.end.y <= name_rect.position.y + 1.0, "%s 介绍应在名称上方" % card_id)
            _expect(info_rect.encloses(desc_rect), "%s 的信息区应包住介绍" % card_id)
            _expect(
                info_rect.encloses(name_rect) and info_rect.encloses(stats_rect),
                "%s 的信息区应包住名称与数值" % card_id,
            )

    # 无介绍的随从（tufu）退回两行信息区，名称与数值仍与其他卡对齐。
    var plain_card: Control = recruit.find_child("ShopSlot2", true, false)
    _expect(plain_card != null, "商店应存在 ShopSlot2")
    if plain_card != null:
        var plain_desc: Label = plain_card.find_child("CardDesc", true, false)
        _expect(
            plain_desc == null or not plain_desc.is_visible_in_tree(),
            "无介绍卡不应显示 CardDesc",
        )
        var plain_info: Control = plain_card.find_child("CardInfo", true, false)
        _expect(
            plain_info != null and absf(plain_info.size.y - 56.0) <= 1.0,
            "无介绍卡信息区高度应为 56，实际 %s" % (plain_info.size.y if plain_info != null else -1.0),
        )
        var qishi_card: Control = recruit.find_child("ShopSlot3", true, false)
        if qishi_card != null:
            var qishi_name: Label = qishi_card.find_child("CardName", true, false)
            var plain_name: Label = plain_card.find_child("CardName", true, false)
            var qishi_attack: Label = qishi_card.find_child("CardAttack", true, false)
            var plain_attack: Label = plain_card.find_child("CardAttack", true, false)
            _expect(
                qishi_name != null and plain_name != null
                and absf(qishi_name.get_global_rect().position.y - plain_name.get_global_rect().position.y) <= 1.0,
                "一行介绍与无介绍的名称基线应一致",
            )
            _expect(
                qishi_attack != null and plain_attack != null
                and absf(qishi_attack.get_global_rect().position.y - plain_attack.get_global_rect().position.y) <= 1.0,
                "一行介绍与无介绍的数值基线应一致",
            )

    recruit.queue_free()
    _finish()
