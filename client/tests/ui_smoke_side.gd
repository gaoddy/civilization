extends "res://tests/smoke_case.gd"
## Task 9 右栏冒烟：顶栏与事件流已删除，右栏改为身份 / 回合倒计时 / 英雄三段。


func _ready() -> void:
    await _check_recruit_identity()
    await _check_countdown_flip()
    await _check_combat_identity()
    await _check_eliminated_identity()
    await _check_ghost_identity()
    _finish()


func _base_view(phase: String) -> Dictionary:
    GameStore.player_id = "p0"
    GameStore.phase = phase
    GameStore.turn = 3
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var view: Dictionary = FixtureScript.game_view()
    view["phase"] = phase
    return view


func _mount(view: Dictionary, combat: Dictionary = {}) -> Node:
    GameStore.view = view
    if combat.is_empty():
        _clear_combat()
    else:
        _set_combat(combat)
    var instance := await _check_scene("res://scenes/game.tscn", false)
    await get_tree().process_frame
    return instance


func _unmount(instance: Node) -> void:
    instance.queue_free()
    await get_tree().process_frame
    GameStore.view = {}
    _clear_combat()
    GameStore.turn = 1
    GameStore.phase = ""


func _check_recruit_identity() -> void:
    var view := _base_view(Protocol.PHASE_RECRUIT)
    var instance := await _mount(view)
    if instance == null:
        return

    _expect(instance.find_child("Header", true, false) == null, "顶栏节点不应存在")
    _expect(instance.find_child("EraPill", true, false) == null, "顶栏时代胶囊不应存在")
    _expect(instance.find_child("LogText", true, false) == null, "事件流文本节点不应存在")

    var status = instance.find_child("SideStatus", true, false)
    _expect(status != null, "右栏中部应存在回合与倒计时块")
    if status != null:
        _expect(
            _node_text(status).contains(TranslationServer.translate("ui.game.side_panel.turn") % 3),
            "右栏中部应显示回合数",
        )

    var identity = instance.find_child("SideIdentity", true, false)
    _expect(identity != null, "右栏顶部应存在身份块")
    if identity != null:
        var identity_text := _node_text(identity)
        _expect(
            identity_text.contains(TranslationServer.translate("ui.game.side_panel.seller")),
            "招募阶段右栏顶部应显示售货员名字",
        )
        _expect(
            not identity_text.contains(TranslationServer.translate("ui.game.detail.shop_open")),
            "营业信息不应常驻显示",
        )
        var seller: Control = identity.find_child("SellerTile", true, false)
        _expect(seller != null, "招募阶段应显示售货员大图")
        if seller != null:
            _expect(seller.size.x >= 150 and seller.size.y >= 150, "售货员应使用大图")
            await _hover(instance, seller, seller.size / 2)
            var detail_panel = instance.find_child("MinionDetailPanel", true, false)
            _expect(
                detail_panel != null and detail_panel.is_visible_in_tree(),
                "悬停售货员应显示详情面板",
            )
            if detail_panel != null:
                var detail_text := _node_text(detail_panel)
                _expect(
                    detail_text.contains(TranslationServer.translate("ui.game.detail.shop_open")),
                    "售货员详情应显示营业状态",
                )
                _expect(
                    detail_text.contains(TranslationServer.translate("ui.game.side_panel.reroll")),
                    "售货员详情应显示可用商店操作",
                )

        # 商店操作按钮在售货员下方，与身份块同栏。
        var reroll: Control = identity.find_child("RerollButton", true, false)
        _expect(reroll != null, "商店操作应位于右栏身份块内")
        if reroll != null and seller != null:
            _expect(
                reroll.get_global_rect().position.y >= seller.get_global_rect().end.y,
                "刷新按钮应位于售货员下方",
            )
        var side_hero = instance.find_child("SideHero", true, false)
        var panel: Control = instance.find_child("SidePanel", true, false)
        var status_rect: Rect2 = status.get_global_rect() if status != null else Rect2()
        var hero_rect: Rect2 = side_hero.get_global_rect() if side_hero != null else Rect2()
        if panel != null and side_hero != null and status != null:
            _expect(
                is_equal_approx(identity.get_global_rect().size.y, hero_rect.size.y),
                "身份块与英雄块应等高：%s / %s" % [identity.get_global_rect().size.y, hero_rect.size.y],
            )
            _expect(
                absf(status_rect.get_center().y - panel.get_global_rect().get_center().y) < 1.0,
                "回合与倒计时块应位于右栏垂直中心",
            )
        var portrait: Control = side_hero.find_child("HeroPortrait", true, false) if side_hero != null else null
        _expect(portrait != null, "我方英雄应使用大图")
        if portrait != null:
            _expect(portrait.size.x >= 150 and portrait.size.y >= 150, "我方英雄大图尺寸应足够")
            var portrait_text := _node_text(portrait)
            _expect(portrait_text.contains("达尔文"), "我方英雄大图应显示名字")
            _expect(
                not portrait_text.contains(TranslationServer.translate("enum.tribe.scholar")),
                "英雄部落等信息不应常驻",
            )
            await _hover(instance, portrait, portrait.size / 2)
            var hero_detail = instance.find_child("MinionDetailPanel", true, false)
            _expect(
                hero_detail != null and hero_detail.is_visible_in_tree(),
                "悬停我方英雄应显示详情面板",
            )
            if hero_detail != null:
                var hero_text := _node_text(hero_detail)
                _expect(
                    hero_text.contains(TranslationServer.translate("ui.game.detail.tribe")),
                    "我方英雄详情应显示部落",
                )
                _expect(
                    hero_text.contains(TranslationServer.translate("ui.game.detail.hero_power")),
                    "我方英雄详情应显示技能说明",
                )

    await _unmount(instance)


## 招募倒计时悬停翻成结束招募；移出恢复倒计时，点击发 EndRecruit。
func _check_countdown_flip() -> void:
    var view := _base_view(Protocol.PHASE_RECRUIT)
    var instance := await _mount(view)
    if instance == null:
        return

    var flip: Control = instance.find_child("CountdownFlip", true, false)
    var timer: Label = instance.find_child("TimerLabel", true, false)
    var end_button: Button = instance.find_child("EndButton", true, false)
    _expect(flip != null and timer != null and end_button != null, "右栏中部应有倒计时翻转区域")
    if flip == null or timer == null or end_button == null:
        await _unmount(instance)
        return

    var captured: Array = []
    instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
    var center := flip.get_global_rect().get_center()
    await _push_motion(instance, center)
    _expect(not timer.visible and end_button.visible, "悬停倒计时应翻成结束招募")
    _expect(
        end_button.text == TranslationServer.translate("ui.game.side_panel.end_recruit"),
        "翻转后应显示结束招募",
    )
    await _push_click(instance, center)
    _expect(
        captured.size() == 1
            and str(captured[0].get("kind", "")) == Protocol.INTENT_END_RECRUIT,
        "点击翻转后的倒计时应发出结束招募意图，实际 %s" % [captured],
    )

    await _push_motion(instance, Vector2(-40, -40))
    _expect(timer.visible and not end_button.visible, "移出倒计时区域应恢复剩余时间")
    await _unmount(instance)


func _check_combat_identity() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["opponentPlayerId"] = "p2"
    var instance := await _mount(view, FixtureScript.combat_result())
    if instance == null:
        return

    var identity = instance.find_child("SideIdentity", true, false)
    _expect(identity != null, "战斗阶段右栏顶部应存在身份块")
    if identity != null:
        var text := _node_text(identity)
        _expect(text.contains("机器人C"), "战斗阶段应显示对手英雄名")
        _expect(text.contains("HP"), "战斗阶段应显示对手 HP 行")
        _expect(identity.find_child("HpBar", true, false) != null, "对手身份应显示血量条")
    var status = instance.find_child("SideStatus", true, false)
    _expect(status != null, "战斗阶段右栏中部应存在回合与倒计时块")
    if status != null:
        _expect(
            _node_text(status).contains(TranslationServer.translate("ui.game.side_panel.combat")),
            "战斗阶段状态行显示战斗中",
        )
        _expect(
            not _node_text(status).contains(
                TranslationServer.translate("ui.game.side_panel.remaining_seconds").split("%")[0]
            ),
            "战斗阶段不显示招募倒计时",
        )

    await _unmount(instance)


func _check_eliminated_identity() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["players"][0]["alive"] = false
    # 同时具备对手与幽灵标记：淘汰判定必须优先于对手/幽灵分支。
    view["opponentPlayerId"] = "p2"
    view["players"][2]["alive"] = false
    var combat := FixtureScript.combat_result()
    combat["mirror"] = true
    var instance := await _mount(view, combat)
    if instance == null:
        return

    var identity = instance.find_child("SideIdentity", true, false)
    _expect(identity != null, "已淘汰观战者应有身份块")
    if identity != null:
        _expect(
            _node_text(identity).contains(TranslationServer.translate("ui.game.side_panel.eliminated")),
            "已淘汰观战者应显示淘汰观战提示",
        )

    await _unmount(instance)


func _check_ghost_identity() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["opponentPlayerId"] = "p2"
    view["players"][2]["alive"] = false
    var combat := FixtureScript.combat_result()
    combat["mirror"] = true
    var instance := await _mount(view, combat)
    if instance == null:
        return

    var identity = instance.find_child("SideIdentity", true, false)
    _expect(identity != null, "幽灵战应有身份块")
    if identity != null:
        _expect(
            _node_text(identity).contains(TranslationServer.translate("ui.game.side_panel.ghost")),
            "幽灵战身份块应标注幽灵",
        )
        _expect(
            identity.find_child("HpBar", true, false) == null,
            "幽灵战身份块不应显示血量条",
        )

    await _unmount(instance)
