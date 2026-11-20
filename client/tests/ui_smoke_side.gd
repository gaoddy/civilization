extends "res://tests/smoke_case.gd"
## Task 9 右栏冒烟：顶栏与事件流已删除，右栏改为身份 / 回合倒计时 / 英雄三段。


class IconProbe extends Control:
    var icon_name := ""

    func _draw() -> void:
        Icons.draw(self, icon_name, Rect2(0, 0, 20, 20), Color.WHITE)


func _ready() -> void:
    await _check_shared_layer()
    await _check_icon_button()
    await _check_hero_card()
    await _check_portrait()
    await _check_top_bar()
    await _check_recruit_river()
    await _check_combat_river()
    await _check_hero_row()
    await _check_hand_row()
    await _check_shop_view()
    await _check_freeze_toggle()
    await _check_recruit_view()
    await _check_opponent_view()
    await _check_recruit_stage()
    await _check_countdown_flip()
    await _check_combat_stage()
    await _check_eliminated_stage()
    await _check_ghost_stage()
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


## 共享层：新 token、样式、图标入口、技能表与手牌拖拽门控。
func _check_shared_layer() -> void:
    _expect(UITheme.metric("hero-row-height") == 84, "英雄行高应为 84")
    _expect(UITheme.metric("hero-card-width") == 64, "英雄卡宽应为 64")
    _expect(UITheme.metric("hero-card-height") == 84, "英雄卡高应为 84")
    _expect(UITheme.metric("shop-icon-size") == 40, "商店图标按钮应为 40")
    _expect(UITheme.metric("river-height") == 32, "河道高应为 32")
    _expect(UITheme.metric("top-bar-height") == 28, "顶部栏高应为 28")
    _expect(UITheme.metric("recruit-shop-gap") == 18, "商店到河道的间距应为 18")
    _expect(UITheme.metric("opponent-hp-width") == 360, "对手血条宽应为 360")
    _expect(UITheme.metric("portrait-size") == 64, "头像尺寸应为 64")
    _expect(UITheme.font_size("font-micro") == 8, "微型字号应为 8")
    _expect(UITheme.panel_style("bare").content_margin_left == 0, "bare 样式内容边距应为 0")
    _expect(UITheme.panel_style("accent-bare").content_margin_left == 0, "accent-bare 样式内容边距应为 0")

    _expect(CardDB.skill("division-of-labor").get("cost") == 1, "技能表应包含分工协作费用")
    _expect(CardDB.skill_cost("division-of-labor") == 1, "技能费用应可读取")
    _expect(CardDB.skill_cost("missing-skill") == -1, "缺失技能费用应为 -1")

    var state := GameUiState.new()
    _expect(not state.can_drag(true), "默认状态下不应允许拖拽手牌")
    state.phase = Protocol.PHASE_RECRUIT
    state.connected = true
    _expect(state.can_drag(true), "招募阶段已连接时应允许拖拽")
    _expect(not state.can_drag(false), "不可出的牌不应允许拖拽")
    for field in ["stale", "expired", "target_pending"]:
        state.set(field, true)
        _expect(not state.can_drag(true), "%s 为真时不应允许拖拽" % field)
        state.set(field, false)

    for icon_name in ["upgrade", "reroll", "freeze", "hero_power", "missing-icon"]:
        var probe := IconProbe.new()
        probe.icon_name = icon_name
        probe.size = Vector2(20, 20)
        add_child(probe)
        probe.queue_redraw()
        await get_tree().process_frame
        probe.queue_free()
    await get_tree().process_frame


## 图标按钮：尺寸、费用角标、次数角标、激活与禁用表现。
func _check_icon_button() -> void:
    var button := GameIconButton.new("upgrade", "Upgrade")
    add_child(button)
    await get_tree().process_frame
    _expect(button.custom_minimum_size == Vector2(40, 40), "图标按钮应为 40×40")
    var cost_badge: Label = button.find_child("UpgradeCostBadge", true, false)
    _expect(cost_badge != null, "图标按钮应有费用角标")
    if cost_badge != null:
        _expect(not cost_badge.visible, "默认无费用时不显示费用角标")
        button.set_cost(4)
        _expect(cost_badge.visible and cost_badge.text == "4", "费用角标应显示 4")
        _expect(
            cost_badge.get_global_rect().get_center().x > button.get_global_rect().get_center().x,
            "费用角标应位于按钮右上侧",
        )
        button.set_cost(-1)
        _expect(not cost_badge.visible, "费用为 -1 时应隐藏费用角标")
    button.queue_free()
    await get_tree().process_frame

    var power := GameIconButton.new("hero_power", "HeroPower")
    power.uses_visible = true
    add_child(power)
    await get_tree().process_frame
    var uses_badge: Label = power.find_child("HeroPowerUsesBadge", true, false)
    _expect(uses_badge != null, "英雄技能按钮应有次数角标")
    if uses_badge != null:
        power.set_uses(1)
        _expect(not uses_badge.visible, "剩余 1 次时不显示次数角标")
        power.set_uses(0)
        _expect(uses_badge.visible and uses_badge.text == "0", "剩余 0 次时应显示 0")
        power.set_uses(2)
        _expect(uses_badge.visible and uses_badge.text == "2", "剩余 2 次时应显示 2")
        _expect(
            uses_badge.get_global_rect().get_center().x < power.get_global_rect().get_center().x,
            "次数角标应位于按钮左上侧",
        )
    power.set_active(true)
    var normal := power.get_theme_stylebox("normal") as StyleBoxFlat
    _expect(
        normal != null and normal.border_color.is_equal_approx(UITheme.color("color-info")),
        "激活态应使用 info 描边",
    )
    power.disabled = true
    await get_tree().process_frame
    _expect(power.modulate.a <= 0.5, "禁用态应整枚压暗")
    power.queue_free()
    await get_tree().process_frame


## 英雄卡：尺寸、名字行、时代与生命两格及其左右顺序。
func _check_hero_card() -> void:
    var card := GameHeroCard.new()
    add_child(card)
    await get_tree().process_frame
    _expect(
        card.custom_minimum_size == Vector2(64, 84),
        "英雄卡最小尺寸应为 64×84，实际 %s" % card.custom_minimum_size,
    )
    card.refresh("adam-smith", 35, 40, 5)
    await get_tree().process_frame
    var name_label: Label = card.find_child("HeroCardName", true, false)
    var era_label: Label = card.find_child("HeroEra", true, false)
    var hp_label: Label = card.find_child("HeroHp", true, false)
    _expect(name_label != null, "英雄卡应有名字行")
    _expect(era_label != null and hp_label != null, "英雄卡应有时代与生命两格")
    _expect(card.find_child("HeroAttack", true, false) == null, "英雄卡不应再显示攻击力")
    if name_label != null and era_label != null and hp_label != null:
        _expect(name_label.text == CardDB.display_name("adam-smith"), "英雄卡应显示英雄名")
        _expect(hp_label.text == "35/40", "英雄卡生命应为 35/40")
        _expect(era_label.text == CardDB.era_name(5), "英雄卡时代应为工业")
        var stats_node: Control = card.find_child("HeroCardStats", true, false)
        _expect(
            stats_node != null
                and name_label.get_global_rect().end.y <= stats_node.get_global_rect().position.y + 1,
            "名字行应在数值行上方",
        )
        _expect(
            era_label.get_global_rect().get_center().x
                < hp_label.get_global_rect().get_center().x,
            "数值顺序应为时代、生命",
        )
    card.queue_free()
    await get_tree().process_frame


## 无图头像：尺寸、首字与名字条。
func _check_portrait() -> void:
    var portrait := GamePortrait.new()
    add_child(portrait)
    await get_tree().process_frame
    _expect(portrait.custom_minimum_size == Vector2(64, 64), "头像最小尺寸应为 64×64")
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), true)
    await get_tree().process_frame
    _expect(_node_text(portrait).contains("售货员"), "头像应显示名字")
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), false)
    await get_tree().process_frame
    _expect(portrait.find_child("PortraitName", true, false) == null, "不显示名字时不应创建名字条")
    var seller_art := "res://assets/characters/avatars/seller_ava.png"
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), true, seller_art)
    await get_tree().process_frame
    _expect(portrait.find_child("PortraitArt", true, false) != null, "有贴图时应挂载头像纹理")
    _expect(
        portrait.find_child("PortraitArt", true, false).get_index() == 0,
        "头像纹理应置于最底层以免遮挡名字条"
    )
    _expect(not portrait.find_child("PortraitGlyph", true, false).visible, "有贴图时应隐藏首字")
    _expect(_node_text(portrait).contains("售货员"), "有贴图时名字条仍应显示")
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), true, seller_art)
    await get_tree().process_frame
    _expect(
        portrait.find_children("PortraitArt", "", true, false).size() == 1,
        "重复 setup 不应残留多张头像纹理"
    )
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), true)
    await get_tree().process_frame
    _expect(portrait.find_child("PortraitArt", true, false) == null, "贴图路径为空时应回退无图")
    _expect(portrait.find_child("PortraitGlyph", true, false).visible, "回退无图时应恢复首字")
    portrait.queue_free()
    await get_tree().process_frame


## 组件级快照：只填视图与门控，不依赖页面。
func _state_for(phase: String, view: Dictionary) -> GameUiState:
    var state := GameUiState.new()
    state.view = view
    state.self_state = view.get("self", {})
    state.phase = phase
    state.recruited = bool(state.self_state.get("recruited", false))
    state.connected = true
    state.can_play = true
    state.hp_cap = maxi(int(view.get("hpCap", 40)), 1)
    state.board_slots = maxi(int(view.get("boardSlots", 7)), 1)
    return state


## 固定尺寸的承载容器：组件在这里按页面真实宽度排版。
func _host(size: Vector2) -> Control:
    var host := Control.new()
    host.size = size
    host.custom_minimum_size = size
    add_child(host)
    await get_tree().process_frame
    return host


## 顶部栏：状态文本、三个入口与连接态可用性。
func _check_top_bar() -> void:
    var bar := GameTopBar.new()
    add_child(bar)
    await get_tree().process_frame
    var signals: Array = []
    bar.gallery_pressed.connect(func() -> void: signals.append("gallery"))
    bar.hint_pressed.connect(func() -> void: signals.append("hint"))
    bar.leave_game_pressed.connect(func() -> void: signals.append("leave"))
    bar.set_status("正在重连", UITheme.color("color-text-dim"), "详情")
    _expect(bar.status.text == "正在重连", "顶部栏应显示状态文本")
    _expect(bar.status.tooltip_text == "详情", "顶部栏状态应带悬停详情")
    var gallery: Button = bar.find_child("GalleryButton", true, false)
    var hint: Button = bar.find_child("HintButton", true, false)
    var leave: Button = bar.find_child("LeaveGameButton", true, false)
    _expect(gallery != null and hint != null and leave != null, "顶部栏应有图鉴、提示与退出对局")
    if gallery != null and hint != null and leave != null:
        gallery.pressed.emit()
        hint.pressed.emit()
        leave.pressed.emit()
        _expect(signals == ["gallery", "hint", "leave"], "三个入口应各自发出信号")
        bar.set_connected(false)
        _expect(hint.disabled and leave.disabled, "断线时提示与退出对局应禁用")
        _expect(not gallery.disabled, "断线时图鉴仍可用")
    bar.queue_free()
    await get_tree().process_frame


## 招募河道：中央回合数、右端倒计时、悬停翻转与结束招募。
func _check_recruit_river() -> void:
    GameStore.deadline = "2030-01-01T00:00:00Z"
    var host: Control = await _host(Vector2(1148, 32))
    var river := GameRiver.new(GameRiver.Mode.RECRUIT)
    river.set_anchors_preset(Control.PRESET_FULL_RECT)
    host.add_child(river)
    await get_tree().process_frame
    var view := FixtureScript.game_view()
    river.refresh(_state_for(Protocol.PHASE_RECRUIT, view))
    await get_tree().process_frame

    var turn: Label = river.find_child("RecruitTurnLabel", true, false)
    var flip: Control = river.find_child("RecruitCountdownFlip", true, false)
    var timer: Label = river.find_child("RecruitTimerLabel", true, false)
    var end_button: Button = river.find_child("RecruitEndButton", true, false)
    _expect(turn != null and flip != null and timer != null and end_button != null, "招募河道节点应齐全")
    if turn == null or flip == null or timer == null or end_button == null:
        river.queue_free()
        await get_tree().process_frame
        return
    _expect(turn.text == TranslationServer.translate("ui.game.side_panel.turn") % 3, "河道中央应显示回合数")
    _expect(
        absf(turn.get_global_rect().get_center().x - river.get_global_rect().get_center().x) < 1.0,
        "回合数应位于河道中央",
    )
    _expect(
        absf(flip.get_global_rect().end.x - river.get_global_rect().end.x) < 4.0,
        "倒计时区域应贴河道右端",
    )
    _expect(river.seconds_left() > 0, "未来截止时间应返回正秒数")
    _expect(not timer.visible or timer.text != "", "有截止时间时倒计时应显示文字")

    var ends: Array = []
    river.end_recruit_pressed.connect(func() -> void: ends.append(true))
    flip.mouse_entered.emit()
    _expect(not timer.visible and end_button.visible, "悬停倒计时应翻成结束招募")
    var click := InputEventMouseButton.new()
    click.button_index = MOUSE_BUTTON_LEFT
    click.pressed = true
    flip.gui_input.emit(click)
    _expect(ends.size() == 1, "点击翻转区域应发出一次结束招募")
    flip.mouse_exited.emit()
    _expect(timer.visible and not end_button.visible, "移出后应恢复倒计时")

    GameStore.deadline = "2000-01-01T00:00:00Z"
    var expired := _state_for(Protocol.PHASE_RECRUIT, view)
    expired.expired = true
    river.refresh(expired)
    _expect(river.seconds_left() == 0, "已过期截止时间应返回 0 秒")
    _expect(
        timer.text == TranslationServer.translate("ui.game.side_panel.waiting_settle"),
        "归零后应显示等待结算",
    )
    _expect(end_button.disabled, "过期后结束招募应禁用")
    host.queue_free()
    await get_tree().process_frame


## 战斗河道：始终显示回合数，结果追加在后，右端只剩「战斗中」。
func _check_combat_river() -> void:
    var host: Control = await _host(Vector2(1148, 32))
    var river := GameRiver.new(GameRiver.Mode.COMBAT)
    river.set_anchors_preset(Control.PRESET_FULL_RECT)
    host.add_child(river)
    await get_tree().process_frame
    river.refresh(_state_for(Protocol.PHASE_COMBAT, FixtureScript.game_view()))
    await get_tree().process_frame
    var banner: Label = river.find_child("CombatBanner", true, false)
    var pill: Label = river.find_child("CombatStatusPill", true, false)
    _expect(banner != null and banner.visible, "战斗河道应始终显示横幅")
    _expect(pill != null, "战斗河道右端应有战斗中胶囊")
    _expect(river.find_child("RecruitCountdownFlip", true, false) == null, "战斗河道不应创建倒计时")
    if banner != null and pill != null:
        _expect(banner.text == TranslationServer.translate("ui.game.side_panel.turn") % 3, "横幅应先显示回合数")
        _expect(pill.text == TranslationServer.translate("ui.game.side_panel.combat"), "右端应显示战斗中文案")
        river.set_combat_detail("胜利", UITheme.color("color-success"))
        _expect(banner.text == "%s · 胜利" % (TranslationServer.translate("ui.game.side_panel.turn") % 3), "结果应追加在回合数之后")
        river.set_combat_detail("", UITheme.color("color-success"))
        _expect(banner.text == TranslationServer.translate("ui.game.side_panel.turn") % 3, "清空结果后应只剩回合数")
    host.queue_free()
    await get_tree().process_frame


## 英雄行：英雄卡、金币、技能角标、战斗隐藏与淘汰替换。
func _check_hero_row() -> void:
    GameStore.player_id = "p0"
    var host: Control = await _host(Vector2(1148, 84))
    var row := GameHeroRow.new()
    row.set_anchors_preset(Control.PRESET_TOP_WIDE)
    host.add_child(row)
    await get_tree().process_frame
    var view := FixtureScript.game_view()
    var state := _state_for(Protocol.PHASE_RECRUIT, view)
    row.refresh(state)
    await get_tree().process_frame

    var card: Control = row.find_child("HeroCard", true, false)
    var gold: Label = row.find_child("GoldTile", true, false)
    var power: Control = row.find_child("HeroPowerButton", true, false)
    _expect(card != null and card.custom_minimum_size == Vector2(64, 84), "英雄行应含 64×84 英雄卡")
    _expect(gold != null and gold.text.contains("6"), "金币格应显示当前金币")
    _expect(
        gold != null and gold.text.contains(TranslationServer.translate("ui.game.side_panel.gold")),
        "金币格应带金币文案",
    )
    _expect(power != null and power.custom_minimum_size == Vector2(40, 40), "英雄技能应为 40×40 图标按钮")
    var uses_badge: Label = row.find_child("HeroPowerUsesBadge", true, false)
    _expect(uses_badge != null and not uses_badge.visible, "剩余 1 次时不显示次数角标")

    var empty_power := _state_for(Protocol.PHASE_RECRUIT, view)
    empty_power.self_state["heroPower"]["usesLeft"] = 0
    empty_power.self_state["affordances"]["canUseHeroPower"] = false
    row.refresh(empty_power)
    await get_tree().process_frame
    _expect(uses_badge != null and uses_badge.visible and uses_badge.text == "0", "0 次时应显示次数角标")
    _expect(power.disabled, "不可用时技能按钮应禁用")

    row.set_combat_mode(true)
    _expect(not power.visible, "战斗阶段应隐藏技能按钮")
    _expect(card.visible, "战斗阶段英雄卡应保留")

    var dead_view := FixtureScript.game_view()
    dead_view["players"][0]["alive"] = false
    row.refresh(_state_for(Protocol.PHASE_COMBAT, dead_view))
    await get_tree().process_frame
    var eliminated: Label = row.find_child("EliminatedLabel", true, false)
    _expect(eliminated != null and eliminated.visible, "本人淘汰时应显示观战提示")
    _expect(not card.visible, "本人淘汰时应隐藏英雄卡")
    host.queue_free()
    await get_tree().process_frame


## 手牌行：总体居中、十张单行、末张补齐整卡宽与拖拽门控。
func _check_hand_row() -> void:
    var host: Control = await _host(Vector2(1148, 146))
    var row := GameHandRow.new()
    row.set_anchors_preset(Control.PRESET_FULL_RECT)
    host.add_child(row)
    await get_tree().process_frame

    var view := FixtureScript.game_view()
    view["self"]["hand"] = []
    for index in 10:
        var card := FixtureScript.hand("archimedes", false)
        card["index"] = index
        card["instanceId"] = str(3000 + index)
        view["self"]["hand"].append(card)
    row.refresh(_state_for(Protocol.PHASE_RECRUIT, view))
    await get_tree().process_frame
    await get_tree().process_frame

    var hand_row: HBoxContainer = row.find_child("HandRow", true, false)
    _expect(hand_row != null and hand_row.get_child_count() == 10, "十张手牌应单行排列")
    if hand_row != null:
        _expect(
            absf(hand_row.get_combined_minimum_size().x - 832.0) < 1.0,
            "十张手牌总宽应为 832，实际 %s" % hand_row.get_combined_minimum_size().x,
        )
        _expect(
            absf(hand_row.get_global_rect().get_center().x - row.get_global_rect().get_center().x) < 1.0,
            "手牌行应在手牌区内居中",
        )
    var last_slot: Control = row.find_child("HandSlot9", true, false)
    _expect(last_slot != null and last_slot.custom_minimum_size.x == 112, "最后一张槽位应补齐整卡宽度")
    var first_card: HandCard = row.find_child("HandCard0", true, false)
    _expect(first_card != null and first_card.drag_allowed, "招募阶段已连接时手牌可拖拽")

    row.refresh(_state_for(Protocol.PHASE_COMBAT, view))
    await get_tree().process_frame
    var combat_card: HandCard = row.find_child("HandCard0", true, false)
    _expect(combat_card != null and not combat_card.drag_allowed, "战斗阶段手牌不可拖拽")

    var empty_view := FixtureScript.game_view()
    empty_view["self"]["hand"] = []
    row.refresh(_state_for(Protocol.PHASE_RECRUIT, empty_view))
    await get_tree().process_frame
    var empty_row: HBoxContainer = row.find_child("HandRow", true, false)
    _expect(empty_row != null and empty_row.get_child_count() == 1, "空手牌应只有一条空态提示")
    host.queue_free()
    await get_tree().process_frame


## 商店视图：操作行围绕售货员、六个卡槽、费用角标与冻结激活态。
func _check_shop_view() -> void:
    var host: Control = await _host(Vector2(1148, 216))
    var shop := GameShopView.new()
    shop.set_anchors_preset(Control.PRESET_TOP_WIDE)
    host.add_child(shop)
    await get_tree().process_frame
    shop.refresh(_state_for(Protocol.PHASE_RECRUIT, FixtureScript.view()))
    await get_tree().process_frame
    await get_tree().process_frame

    var actions: Control = shop.find_child("ShopActions", true, false)
    var upgrade: Button = shop.find_child("UpgradeButton", true, false)
    var seller: Control = shop.find_child("SellerTile", true, false)
    var reroll: Button = shop.find_child("RerollButton", true, false)
    var freeze: Button = shop.find_child("FreezeButton", true, false)
    _expect(actions != null and actions.get_global_rect().size.y == 64, "商店操作行高应为 64")
    _expect(upgrade != null and upgrade.custom_minimum_size == Vector2(40, 40), "升级按钮应为 40×40")
    _expect(seller != null and seller.custom_minimum_size == Vector2(64, 64), "售货员应为 64×64")
    _expect(seller != null and seller.find_child("PortraitArt", true, false) != null, "售货员应显示立绘贴图")
    _expect(reroll != null and reroll.custom_minimum_size == Vector2(40, 40), "刷新按钮应为 40×40")
    _expect(freeze != null and freeze.custom_minimum_size == Vector2(40, 40), "冻结按钮应为 40×40")
    if upgrade != null and seller != null and reroll != null:
        _expect(
            upgrade.get_global_rect().get_center().x < seller.get_global_rect().get_center().x,
            "升级时代应在售货员左侧",
        )
        _expect(
            reroll.get_global_rect().get_center().x > seller.get_global_rect().get_center().x,
            "刷新与冻结应在售货员右侧",
        )
    var panel: Control = shop.find_child("ShopPanel", true, false)
    _expect(panel is SellDrop, "商店面板应为出售落点")
    _expect(panel != null and panel.get_global_rect().size.y == 146, "商店面板高应为 146")
    var shop_row: Control = shop.find_child("ShopRow", true, false)
    var slot_count := 0
    if shop_row != null:
        for child in shop_row.get_children():
            if str(child.name).begins_with("ShopSlot"):
                slot_count += 1
    _expect(slot_count == 4, "商店应只渲染 4 个有卡槽位，实际 %d" % slot_count)
    _expect(shop.find_child("ShopSlot2", true, false) == null, "空商店槽不应渲染占位")
    _expect(shop.find_child("ShopSlot4", true, false) == null, "空商店槽不应渲染占位")
    var upgrade_badge: Label = shop.find_child("UpgradeCostBadge", true, false)
    var reroll_badge: Label = shop.find_child("RerollCostBadge", true, false)
    _expect(upgrade_badge != null and upgrade_badge.visible and upgrade_badge.text == "5", "升级费用角标应显示 5")
    _expect(reroll_badge != null and reroll_badge.visible and reroll_badge.text == "1", "刷新费用角标应显示 1")
    _expect(shop.find_child("FreezeCostBadge", true, false) == null, "冻结不应有费用角标")
    if freeze != null:
        var box := freeze.get_theme_stylebox("normal") as StyleBoxFlat
        _expect(
            box != null and box.border_color.is_equal_approx(UITheme.color("color-info")),
            "有卡牌被冻结时冻结按钮应高亮",
        )
        _expect(
            freeze.tooltip_text == TranslationServer.translate("ui.game.side_panel.unfreeze"),
            "有卡牌被冻结时冻结按钮 tooltip 应为解冻",
        )
    var slot0: Control = shop.find_child("ShopSlot0", true, false)
    var slot1: Control = shop.find_child("ShopSlot1", true, false)
    _expect(slot0 is ShopCard and slot1 is ShopCard, "商店槽位应由 ShopCard 承载")
    if slot0 is ShopCard and slot1 is ShopCard:
        _expect(int(slot0.payload.get("shopSlot", -1)) == 0, "商店卡拖拽数据应带槽位 0")
        _expect(slot0.drag_allowed, "可购买商店卡应允许拖拽")
        _expect(not slot1.drag_allowed, "不可购买商店卡不应允许拖拽")
    _expect(not shop.has_signal("buy"), "商店视图不应再提供点击购买信号")
    host.queue_free()
    await get_tree().process_frame


## 冻结按钮按商店冻结状态切换意图与 tooltip。
func _check_freeze_toggle() -> void:
    var view := _base_view(Protocol.PHASE_RECRUIT)
    var instance := await _mount(view)
    if instance == null:
        return
    var freeze: Button = instance.find_child("FreezeButton", true, false)
    _expect(freeze != null, "冻结按钮应存在")
    if freeze == null:
        await _unmount(instance)
        return
    _expect(
        freeze.tooltip_text == TranslationServer.translate("ui.game.side_panel.unfreeze"),
        "存在冻结槽位时 tooltip 应为解冻",
    )
    var captured: Array = []
    instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
    await _push_click(instance, freeze.get_global_rect().get_center())
    _expect(
        captured.size() == 1
            and str(captured[0].get("kind", "")) == Protocol.INTENT_UNFREEZE_SHOP
            and captured[0].get("slot") == null,
        "存在冻结槽位时应发出整店解冻意图，实际 %s" % [captured],
    )
    await _unmount(instance)

    var thawed := _base_view(Protocol.PHASE_RECRUIT)
    for slot in thawed["self"]["shop"]:
        slot["frozen"] = false
    instance = await _mount(thawed)
    if instance == null:
        return
    freeze = instance.find_child("FreezeButton", true, false)
    _expect(freeze != null, "冻结按钮应存在")
    if freeze == null:
        await _unmount(instance)
        return
    _expect(
        freeze.tooltip_text == TranslationServer.translate("ui.game.side_panel.freeze"),
        "无冻结槽位时 tooltip 应为冻结",
    )
    captured = []
    instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
    await _push_click(instance, freeze.get_global_rect().get_center())
    _expect(
        captured.size() == 1
            and str(captured[0].get("kind", "")) == Protocol.INTENT_FREEZE_SHOP
            and captured[0].get("slot") == null,
        "无冻结槽位时应发出整店冻结意图，实际 %s" % [captured],
    )
    await _unmount(instance)


## 招募组合：商店、河道、棋盘的顺序与高度。
func _check_recruit_view() -> void:
    var host: Control = await _host(Vector2(1148, 418))
    var view := GameRecruitView.new()
    view.set_anchors_preset(Control.PRESET_TOP_WIDE)
    host.add_child(view)
    await get_tree().process_frame
    var state := _state_for(Protocol.PHASE_RECRUIT, FixtureScript.view())
    view.refresh(state)
    await get_tree().process_frame
    await get_tree().process_frame

    var names: Array = []
    for child in view.get_children():
        names.append(str(child.name))
    _expect(
        names == ["ShopView", "RecruitShopGap", "RecruitRiver", "RecruitRiverGap", "BoardPanel"],
        "招募组合子节点顺序不符：%s" % [names],
    )
    _expect(
        absf(view.get_combined_minimum_size().y - 418.0) < 1.0,
        "招募组合总高应为 418，实际 %s" % view.get_combined_minimum_size().y,
    )
    var shop: Control = view.find_child("ShopView", true, false)
    var river: Control = view.find_child("RecruitRiver", true, false)
    var board: Control = view.find_child("BoardPanel", true, false)
    _expect(shop != null and shop.get_global_rect().size.y == 216, "商店块高应为 216")
    _expect(river != null and river.get_global_rect().size.y == 32, "河道高应为 32")
    _expect(board != null and board.get_global_rect().size.y == 146, "棋盘块高应为 146")
    var turn: Label = view.find_child("RecruitTurnLabel", true, false)
    _expect(
        turn != null and turn.text == TranslationServer.translate("ui.game.side_panel.turn") % 3,
        "招募河道应显示回合数",
    )
    var board_scroll: Control = view.find_child("BoardScroll", true, false)
    _expect(board_scroll != null and board_scroll.get_global_rect().size.y == 146, "棋盘滚动区高应为 146")
    var board_row: Control = view.find_child("BoardRow", true, false)
    _expect(board_row != null and board_row.get_child_count() == 1, "棋盘只应渲染实际随从")
    host.queue_free()
    await get_tree().process_frame


## 对手视图：普通对手显示头像、名字与血量条；幽灵战换身份块并去掉血量条；
## 对手为空时显示等待配对。
func _check_opponent_view() -> void:
    var host: Control = await _host(Vector2(1148, 64))
    var opponent := GameOpponentView.new()
    host.add_child(opponent)
    await get_tree().process_frame

    var view := FixtureScript.game_view()
    view["opponentPlayerId"] = "p2"
    GameStore.turn = 3
    _clear_combat()
    opponent.refresh(_state_for(Protocol.PHASE_COMBAT, view))
    await get_tree().process_frame
    var normal: Control = opponent.find_child("OpponentIdentity", true, false)
    var portrait: Control = opponent.find_child("OpponentPortrait", true, false)
    _expect(normal != null, "普通对手应显示身份块")
    _expect(
        portrait != null and portrait.custom_minimum_size == Vector2(64, 64),
        "对手头像应为 64×64",
    )
    _expect(
        _node_text(opponent).contains("机器人C"),
        "对手视图应显示展示名，实际「%s」" % _node_text(opponent),
    )
    _expect(opponent.find_child("HpBar", true, false) != null, "普通对手应有血量条")

    var ghost_view := FixtureScript.game_view()
    ghost_view["opponentPlayerId"] = "p2"
    GameStore.phase = Protocol.PHASE_COMBAT
    _set_combat({"mirror": true})
    opponent.refresh(_state_for(Protocol.PHASE_COMBAT, ghost_view))
    await get_tree().process_frame
    _expect(opponent.find_child("GhostIdentity", true, false) != null, "幽灵战应显示幽灵身份块")
    _expect(opponent.find_child("GhostBadge", true, false) != null, "幽灵战应有幽灵角标")
    _expect(opponent.find_child("HpBar", true, false) == null, "幽灵战不应显示血量条")
    _clear_combat()

    var empty_view := FixtureScript.game_view()
    empty_view["opponentPlayerId"] = null
    opponent.refresh(_state_for(Protocol.PHASE_COMBAT, empty_view))
    await get_tree().process_frame
    _expect(opponent.find_child("OpponentPortrait", true, false) == null, "无对手时不应显示头像")
    _expect(
        _node_text(opponent).contains(TranslationServer.translate("ui.game.side_panel.waiting_match")),
        "无对手时应显示等待配对",
    )
    host.queue_free()
    await get_tree().process_frame


## 招募阶段页面结构：无右栏；中央舞台自上而下为招募、战斗（隐藏）、英雄、手牌。
func _check_recruit_stage() -> void:
    var view := _base_view(Protocol.PHASE_RECRUIT)
    var instance := await _mount(view)
    if instance == null:
        return

    for node_name in ["SidePanel", "SideIdentity", "SideStatus", "SideHero", "HandPanel"]:
        _expect(
            instance.find_child(node_name, true, false) == null,
            "不应存在右栏与旧手牌节点 %s" % node_name,
        )
    _expect(instance.find_child("Header", true, false) == null, "顶栏节点不应存在")
    _expect(instance.find_child("LogText", true, false) == null, "事件流文本节点不应存在")
    var top_bar: Control = instance.find_child("TopBar", true, false)
    _expect(
        top_bar != null and top_bar.find_child("StatusLabel", true, false) != null,
        "顶部工具条应含状态文本",
    )

    var center: Control = instance.find_child("CenterPanel", true, false)
    _expect(center != null, "中央舞台应存在")
    if center != null:
        var names: Array = []
        for child in center.get_children():
            names.append(str(child.name))
        _expect(
            names == ["RecruitPanel", "CombatPanel", "HeroRow", "HandStrip"],
            "中央舞台顺序不符：%s" % [names],
        )

    var recruit: Control = instance.find_child("RecruitPanel", true, false)
    var combat: Control = instance.find_child("CombatPanel", true, false)
    _expect(recruit != null and recruit.visible, "招募阶段应显示招募组合")
    _expect(combat != null and not combat.visible, "招募阶段不应显示战斗组合")

    # 英雄行 84，英雄卡 64×84；名字在时代上方，时代在左、生命在右。
    var hero_row: Control = instance.find_child("HeroRow", true, false)
    var hero_card: Control = instance.find_child("HeroCard", true, false)
    _expect(
        hero_row != null and is_equal_approx(hero_row.size.y, 84.0),
        "英雄行高应为 84，实际 %.1f" % (0.0 if hero_row == null else hero_row.size.y),
    )
    _expect(
        hero_card != null and hero_card.size == Vector2(64, 84),
        "英雄卡应为 64×84，实际 %s" % ("null" if hero_card == null else str(hero_card.size)),
    )
    if hero_card != null:
        var name_label: Label = hero_card.find_child("HeroCardName", true, false)
        var era: Label = hero_card.find_child("HeroEra", true, false)
        var hp: Label = hero_card.find_child("HeroHp", true, false)
        _expect(
            name_label != null and era != null and hp != null,
            "英雄卡应含名字、时代与生命",
        )
        _expect(
            hero_card.find_child("HeroAttack", true, false) == null,
            "英雄卡不应再显示攻击力",
        )
        if name_label != null and era != null and hp != null:
            _expect(name_label.text.contains("达尔文"), "英雄卡应显示英雄名")
            _expect(
                name_label.get_global_rect().end.y <= era.get_global_rect().position.y,
                "英雄名应在时代上方单独一行",
            )
            _expect(
                era.get_global_rect().get_center().x < hp.get_global_rect().get_center().x,
                "时代应在左、生命在右",
            )
            _expect(hp.text == "35/40", "生命应显示当前/上限，实际「%s」" % hp.text)

    var power: Control = instance.find_child("HeroPowerButton", true, false)
    _expect(power != null and power.size == Vector2(40, 40), "英雄技能图标应为 40×40")
    if power != null:
        _expect(power.find_child("HeroPowerCostBadge", true, false) != null, "技能应有费用角标")
        _expect(power.find_child("HeroPowerUsesBadge", true, false) != null, "技能应有次数角标")
    _expect(instance.find_child("GoldTile", true, false) != null, "英雄行应显示金币")

    # 商店操作行：升时代在售货员左侧，刷新与冻结在右侧，均为 40×40 图标按钮。
    var seller: Control = instance.find_child("SellerTile", true, false)
    var upgrade: Control = instance.find_child("UpgradeButton", true, false)
    var reroll: Control = instance.find_child("RerollButton", true, false)
    var freeze: Control = instance.find_child("FreezeButton", true, false)
    _expect(seller != null and seller.size == Vector2(64, 64), "售货员应为 64×64")
    for button in [upgrade, reroll, freeze]:
        _expect(button != null and button.size == Vector2(40, 40), "商店操作应为 40×40 图标")
    if seller != null and upgrade != null and reroll != null and freeze != null:
        _expect(
            upgrade.get_global_rect().end.x <= seller.get_global_rect().position.x
                and seller.get_global_rect().end.x <= reroll.get_global_rect().position.x
                and reroll.get_global_rect().end.x <= freeze.get_global_rect().position.x,
            "商店操作顺序应为升时代—售货员—刷新—冻结",
        )

    # 河道在商店与棋盘之间，回合数居中、倒计时贴右端。
    var shop_panel: Control = instance.find_child("ShopPanel", true, false)
    var river: Control = instance.find_child("RecruitRiver", true, false)
    var board_row: Control = instance.find_child("BoardRow", true, false)
    _expect(shop_panel != null and river != null and board_row != null, "商店、河道与棋盘应存在")
    if shop_panel != null and river != null and board_row != null:
        _expect(
            shop_panel.get_global_rect().end.y <= river.get_global_rect().position.y
                and river.get_global_rect().end.y <= board_row.get_global_rect().position.y,
            "河道应位于商店与棋盘之间",
        )
        var turn: Label = river.find_child("RecruitTurnLabel", true, false)
        var flip: Control = river.find_child("RecruitCountdownFlip", true, false)
        _expect(
            turn != null
                and absf(turn.get_global_rect().get_center().x - river.get_global_rect().get_center().x) < 1.0,
            "回合数应位于河道中央",
        )
        _expect(
            flip != null and absf(flip.get_global_rect().end.x - river.get_global_rect().end.x) < 4.0,
            "倒计时区域应贴河道右端",
        )

    # 手牌贴底：英雄行下方，卡底边距页面底部 8（712）。
    var hand: Control = instance.find_child("HandStrip", true, false)
    _expect(hand != null and hero_row != null, "手牌行与英雄行应存在")
    if hand != null and hero_row != null:
        _expect(
            hand.get_global_rect().position.y >= hero_row.get_global_rect().end.y,
            "手牌应在英雄行下方",
        )
        _expect(
            absf(hand.get_global_rect().end.y - 712.0) < 1.0,
            "手牌底边应距页面底部 8，实际 %.1f" % hand.get_global_rect().end.y,
        )

    await _unmount(instance)


## 招募倒计时悬停翻成结束招募；移出恢复倒计时，点击发 EndRecruit。
func _check_countdown_flip() -> void:
    var view := _base_view(Protocol.PHASE_RECRUIT)
    var instance := await _mount(view)
    if instance == null:
        return

    var flip: Control = instance.find_child("RecruitCountdownFlip", true, false)
    var timer: Label = instance.find_child("RecruitTimerLabel", true, false)
    var end_button: Button = instance.find_child("RecruitEndButton", true, false)
    _expect(flip != null and timer != null and end_button != null, "河道右端应有倒计时翻转区域")
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


## 战斗阶段页面结构：战斗组合替换招募组合，顶部恢复对手身份；英雄卡保留、技能图标隐藏。
func _check_combat_stage() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["opponentPlayerId"] = "p2"
    var instance := await _mount(view, FixtureScript.combat_result())
    if instance == null:
        return

    var recruit: Control = instance.find_child("RecruitPanel", true, false)
    var combat: Control = instance.find_child("CombatPanel", true, false)
    _expect(recruit != null and not recruit.visible, "战斗阶段不应显示招募组合")
    _expect(combat != null and combat.visible, "战斗阶段应显示战斗组合")

    var opponent: Control = instance.find_child("OpponentView", true, false)
    _expect(opponent != null and opponent.is_visible_in_tree(), "战斗阶段顶部应显示对手身份块")
    if opponent != null:
        var text := _node_text(opponent)
        _expect(text.contains("机器人C"), "战斗阶段应显示对手英雄名")
        _expect(opponent.find_child("HpBar", true, false) != null, "对手身份应显示血量条")

    var pill: Label = instance.find_child("CombatStatusPill", true, false)
    _expect(
        pill != null and pill.is_visible_in_tree()
            and pill.text == TranslationServer.translate("ui.game.side_panel.combat"),
        "战斗河道右端应显示战斗中",
    )

    var hero_card: Control = instance.find_child("HeroCard", true, false)
    var power: Control = instance.find_child("HeroPowerButton", true, false)
    _expect(hero_card != null and hero_card.is_visible_in_tree(), "战斗阶段英雄卡应保留")
    _expect(
        power == null or not power.is_visible_in_tree(),
        "战斗阶段应隐藏英雄技能图标",
    )

    var hand: Control = instance.find_child("HandStrip", true, false)
    _expect(hand != null and hand.is_visible_in_tree(), "战斗阶段手牌仍应可见")
    var hand_card: Control = instance.find_child("HandCard0", true, false)
    _expect(
        hand_card != null and not bool(hand_card.get("drag_allowed")),
        "战斗阶段手牌不可拖动",
    )

    await _unmount(instance)


## 本人淘汰只影响英雄行：显示观战提示并隐藏英雄卡，不拦截对手展示。
func _check_eliminated_stage() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["players"][0]["alive"] = false
    view["opponentPlayerId"] = "p2"
    view["players"][2]["alive"] = false
    var combat := FixtureScript.combat_result()
    combat["mirror"] = true
    var instance := await _mount(view, combat)
    if instance == null:
        return

    var hero_row: Control = instance.find_child("HeroRow", true, false)
    var eliminated: Label = instance.find_child("EliminatedLabel", true, false)
    var hero_card: Control = instance.find_child("HeroCard", true, false)
    _expect(
        eliminated != null and eliminated.is_visible_in_tree()
            and eliminated.text == TranslationServer.translate("ui.game.side_panel.eliminated"),
        "已淘汰时英雄行应显示观战提示",
    )
    _expect(hero_card != null and not hero_card.is_visible_in_tree(), "已淘汰时应隐藏英雄卡")
    _expect(
        hero_row != null and instance.find_child("OpponentView", true, false).is_visible_in_tree(),
        "本人淘汰不应拦截对手展示",
    )

    await _unmount(instance)


## 幽灵战：对手身份块带幽灵角标，不显示血量条。
func _check_ghost_stage() -> void:
    var view := _base_view(Protocol.PHASE_COMBAT)
    view["opponentPlayerId"] = "p2"
    view["players"][2]["alive"] = false
    var combat := FixtureScript.combat_result()
    combat["mirror"] = true
    var instance := await _mount(view, combat)
    if instance == null:
        return

    var opponent: Control = instance.find_child("OpponentView", true, false)
    _expect(opponent != null, "幽灵战应有对手身份块")
    if opponent != null:
        _expect(opponent.find_child("GhostIdentity", true, false) != null, "幽灵战应换成幽灵身份块")
        _expect(opponent.find_child("GhostBadge", true, false) != null, "幽灵战应标注幽灵")
        _expect(
            opponent.find_child("HpBar", true, false) == null,
            "幽灵战不应显示血量条",
        )

    await _unmount(instance)
