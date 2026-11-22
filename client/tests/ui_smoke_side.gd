extends "res://tests/smoke_case.gd"
## Task 9 右栏冒烟：顶栏与事件流已删除，右栏改为身份 / 回合倒计时 / 英雄三段。


class IconProbe extends Control:
    var icon_name := ""

    func _draw() -> void:
        Icons.draw(self, icon_name, Rect2(0, 0, 20, 20), Color.WHITE)


func _ready() -> void:
    await _check_shared_layer()
    await _check_popup_layer()
    await _check_icon_button()
    await _check_hero_card()
    await _check_portrait()
    await _check_tool_buttons()
    await _check_status_modal()
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
    await _check_countdown_touch_single_intent()
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
    var instance := await _check_scene("res://pages/game/game.tscn", false)
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
    _expect(UITheme.metric("hero-card-size") == 84, "英雄卡边长应为 84")
    _expect(UITheme.metric("shop-icon-size") == 40, "商店图标按钮应为 40")
    _expect(UITheme.metric("river-height") == 44, "河道高应为 44")
    _expect(UITheme.metric("pill-button-height") == 26, "胶囊按钮高应为 26")
    _expect(UITheme.metric("recruit-shop-gap") == 6, "商店到河道的间距应为 6")
    _expect(UITheme.metric("opponent-hp-width") == 84, "对手血条宽应为 84")
    _expect(UITheme.metric("river-gold-width") == 72, "河道金币宽应为 72")
    _expect(UITheme.metric("river-gold-height") == 24, "河道金币高应为 24")
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


## 弹出层外壳：内容替换、居中、锚点夹紧与底边贴边。
func _check_popup_layer() -> void:
    var host := Control.new()
    host.size = Vector2(400, 300)
    add_child(host)

    var centered := PopupLayer.new()
    host.add_child(centered)
    await get_tree().process_frame
    _expect(not centered.is_layer_open(), "弹出层初始应隐藏")
    var panel := Control.new()
    panel.custom_minimum_size = Vector2(160, 100)
    centered.set_content(panel)
    centered.open_layer()
    await get_tree().process_frame
    _expect(centered.is_layer_open(), "open 后弹层应显示")
    _expect(panel.get_parent() == centered.content_host, "内容应挂在 PopupBody")
    centered.close_layer()
    _expect(not centered.is_layer_open(), "close 后弹层应隐藏")

    var anchored := PopupLayer.new(PopupLayer.Placement.ANCHORED, 0.62, UITheme.color("color-bg"))
    host.add_child(anchored)
    var anchored_panel := Control.new()
    anchored_panel.custom_minimum_size = Vector2(120, 80)
    anchored.set_content(anchored_panel)
    anchored.open_layer_at(Vector2(900, 900))
    await get_tree().process_frame
    _expect(
        anchored_panel.position.x >= anchored.edge_margin
        and anchored_panel.position.y >= anchored.edge_margin,
        "锚点弹层不应越过边缘间距"
    )
    _expect(
        anchored_panel.position.x <= host.size.x - anchored_panel.size.x,
        "锚点弹层应夹紧在图层内"
    )

    var edge := PopupLayer.new(PopupLayer.Placement.EDGE_BOTTOM, 0.55)
    host.add_child(edge)
    var edge_panel := Control.new()
    edge_panel.custom_minimum_size = Vector2(200, 60)
    edge.set_content(edge_panel)
    edge.open_layer()
    await get_tree().process_frame
    _expect(
        is_equal_approx(edge_panel.position.y, host.size.y - edge_panel.size.y - edge.edge_margin),
        "底边弹层应贴底"
    )

    host.queue_free()
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


## 英雄卡：方形边长、头像铺满、只保留右下角当前生命。
func _check_hero_card() -> void:
    var card := GameHeroCard.new()
    add_child(card)
    await get_tree().process_frame
    _expect(
        card.custom_minimum_size == Vector2(84, 84),
        "英雄卡最小尺寸应为 84×84，实际 %s" % card.custom_minimum_size,
    )
    card.refresh("adam-smith", 35, 40)
    await get_tree().process_frame
    _expect(card.find_child("HeroCardName", true, false) == null, "英雄卡不应再显示名字行")
    _expect(card.find_child("HeroEra", true, false) == null, "英雄卡不应再显示时代")
    _expect(card.find_child("HeroAttack", true, false) == null, "英雄卡不应再显示攻击力")
    var hp_label: Label = card.find_child("HeroHp", true, false)
    var hp_plate: Control = card.find_child("HeroHpPlate", true, false)
    _expect(hp_label != null and hp_plate != null, "英雄卡应有右下角当前生命")
    var cover_art: TextureRect = card.find_child("AvatarFrameArt", true, false)
    var cover_mark: Label = card.find_child("AvatarFrameGlyph", true, false)
    _expect(cover_art != null and cover_art.visible, "有头像素材的英雄卡应显示封面图")
    _expect(
        cover_art != null and cover_art.stretch_mode == TextureRect.STRETCH_KEEP_ASPECT_COVERED,
        "英雄卡封面应裁切铺满整卡",
    )
    _expect(cover_mark != null and not cover_mark.visible, "有封面图时应隐藏首字")
    var clip := card.find_child("AvatarFrameClip", true, false) as CanvasItem
    var border := card.find_child("AvatarFrameBorder", true, false) as Control
    _expect(clip != null, "英雄卡应有头像裁切层")
    _expect(
        clip != null and clip.clip_children == CanvasItem.CLIP_CHILDREN_AND_DRAW,
        "英雄卡应同时绘制封面并裁切头像",
    )
    _expect(cover_art != null and clip != null and cover_art.get_parent() == clip, "头像图应位于裁切层内")
    _expect(
        border != null and clip != null and border.get_index() > clip.get_index(),
        "圆角描边应叠在头像之上",
    )
    if hp_label != null and hp_plate != null:
        _expect(hp_label.text == "35", "英雄卡生命只显示当前值，实际「%s」" % hp_label.text)
        var card_rect := card.get_global_rect()
        var plate_rect := hp_plate.get_global_rect()
        _expect(
            plate_rect.get_center().x > card_rect.get_center().x
                and plate_rect.get_center().y > card_rect.get_center().y,
            "生命牌应贴右下角：卡 %s，牌 %s" % [str(card_rect), str(plate_rect)],
        )
        _expect(
            card_rect.encloses(plate_rect),
            "生命牌应完全位于卡内：卡 %s，牌 %s" % [str(card_rect), str(plate_rect)],
        )
    _expect(card.tooltip_text == CardDB.display_name("adam-smith"), "英雄名应由 tooltip 承担")
    card.refresh("", 0, 0)
    await get_tree().process_frame
    _expect(card.find_child("AvatarFrameArt", true, false) == null, "无英雄时应移除封面图")
    _expect(
        card.find_child("AvatarFrameGlyph", true, false).visible,
        "无英雄时应显示首字",
    )
    _expect(
        not (card.find_child("HeroHpPlate", true, false) as Control).visible,
        "无英雄时应隐藏生命牌",
    )
    card.queue_free()
    await get_tree().process_frame


## 圆角头像块：尺寸、裁切、描边层次、首字与名字条。
func _check_portrait() -> void:
    var portrait := GamePortrait.new()
    add_child(portrait)
    await get_tree().process_frame
    _expect(portrait.custom_minimum_size == Vector2(64, 64), "头像最小尺寸应为 64×64")
    var clip := portrait.find_child("AvatarFrameClip", true, false) as Panel
    var border := portrait.find_child("AvatarFrameBorder", true, false) as Panel
    _expect(
        clip != null and clip.clip_children == CanvasItem.CLIP_CHILDREN_AND_DRAW,
        "头像块应同时绘制封面并裁切子节点"
    )
    var cover: StyleBoxFlat = null
    if clip != null:
        cover = clip.get_theme_stylebox("panel") as StyleBoxFlat
    _expect(
        cover != null and cover.corner_radius_top_left == UITheme.radius("radius-md"),
        "头像块圆角应取 radius-md，实际 %s"
        % (cover.corner_radius_top_left if cover != null else "缺失")
    )
    _expect(
        border != null and clip != null and border.get_index() > clip.get_index(),
        "圆角描边应叠在头像之上"
    )
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), true)
    await get_tree().process_frame
    _expect(_node_text(portrait).contains("售货员"), "头像应显示名字")
    var name_label := portrait.find_child("PortraitName", true, false) as Label
    _expect(
        name_label != null and name_label.get_parent() == portrait.clipped_content(),
        "名字条应挂在裁切内叠层，随头像收进圆角"
    )
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), false)
    await get_tree().process_frame
    _expect(portrait.find_child("PortraitName", true, false) == null, "不显示名字时不应创建名字条")
    var seller_art := "res://assets/characters/avatars/seller_ava.png"
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), true, seller_art)
    await get_tree().process_frame
    var art := portrait.find_child("AvatarFrameArt", true, false) as TextureRect
    _expect(art != null, "有贴图时应挂载头像纹理")
    _expect(
        art != null and clip != null and art.get_parent() == clip and art.get_index() == 0,
        "头像纹理应位于裁切层最底层，避免遮挡名字条"
    )
    _expect(not portrait.find_child("AvatarFrameGlyph", true, false).visible, "有贴图时应隐藏首字")
    _expect(_node_text(portrait).contains("售货员"), "有贴图时名字条仍应显示")
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), true, seller_art)
    await get_tree().process_frame
    _expect(
        portrait.find_children("AvatarFrameArt", "", true, false).size() == 1,
        "重复 setup 不应残留多张头像纹理"
    )
    portrait.setup("售货员", "售", UITheme.playstyle_color("neutral"), true)
    await get_tree().process_frame
    _expect(portrait.find_child("AvatarFrameArt", true, false) == null, "贴图路径为空时应回退无图")
    _expect(portrait.find_child("AvatarFrameGlyph", true, false).visible, "回退无图时应恢复首字")
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


## 右上角工具组：三枚胶囊按钮各自发出信号，断线时只禁用提示与退出对局。
func _check_tool_buttons() -> void:
    var tools := GameToolButtons.new()
    add_child(tools)
    await get_tree().process_frame
    var signals: Array = []
    tools.gallery_pressed.connect(func() -> void: signals.append("gallery"))
    tools.hint_pressed.connect(func() -> void: signals.append("hint"))
    tools.leave_game_pressed.connect(func() -> void: signals.append("leave"))
    var gallery: Button = tools.find_child("GalleryButton", true, false)
    var hint: Button = tools.find_child("HintButton", true, false)
    var leave: Button = tools.find_child("LeaveGameButton", true, false)
    _expect(gallery != null and hint != null and leave != null, "工具组应有图鉴、提示与退出对局")
    if gallery != null and hint != null and leave != null:
        _expect(
            gallery is PillButton and hint is PillButton and leave is PillButton,
            "三个入口应各自是胶囊按钮",
        )
        gallery.pressed.emit()
        hint.pressed.emit()
        leave.pressed.emit()
        _expect(signals == ["gallery", "hint", "leave"], "三个入口应各自发出信号")
        tools.set_connected(false)
        _expect(hint.disabled and leave.disabled, "断线时提示与退出对局应禁用")
        _expect(not gallery.disabled, "断线时图鉴仍可用")
    tools.queue_free()
    await get_tree().process_frame


## 状态模态：居中面板取令牌宽度，面板与内容容器放行输入，只有「知道了」接收点击。
func _check_status_modal() -> void:
    var host: Control = await _host(Vector2(1280, 720))
    var modal := GameStatusModal.new()
    host.add_child(modal)
    await get_tree().process_frame
    await get_tree().process_frame
    _expect(not modal.visible, "状态模态默认隐藏")

    modal.show_status("未连接 · 正在重连", "danger", "详情")
    await get_tree().process_frame
    _expect(modal.visible and modal.is_layer_open(), "show_status 后模态应显示")
    var message: Label = modal.find_child("StatusMessage", true, false)
    var detail: Label = modal.find_child("StatusDetail", true, false)
    var panel: Control = modal.find_child("StatusPanel", true, false)
    var body: Control = modal.find_child("StatusBody", true, false)
    var actions: Control = modal.find_child("StatusActions", true, false)
    var confirm: Control = modal.find_child("StatusConfirm", true, false)
    _expect(message != null and message.text == "未连接 · 正在重连", "模态应显示主文案")
    _expect(detail != null and detail.visible and detail.text == "详情", "模态应显示次要说明")
    _expect(
        modal.mouse_filter == Control.MOUSE_FILTER_IGNORE
            and modal.scrim.mouse_filter == Control.MOUSE_FILTER_IGNORE,
        "模态与遮罩应放行输入",
    )
    _expect(
        panel != null
            and panel.mouse_filter == Control.MOUSE_FILTER_IGNORE
            and body != null
            and body.mouse_filter == Control.MOUSE_FILTER_IGNORE
            and actions != null
            and actions.mouse_filter == Control.MOUSE_FILTER_IGNORE,
        "面板与内容容器应放行输入",
    )
    _expect(
        confirm != null and confirm.mouse_filter == Control.MOUSE_FILTER_STOP,
        "「知道了」按钮应接收点击",
    )
    if panel != null:
        var width := UITheme.stage_metric("status-dialog-width")
        _expect(
            absf(panel.size.x - float(width)) <= 2.0,
            "面板宽度应取 status-dialog-width，实际 %s" % panel.size.x,
        )

    var acked := [false]
    modal.acknowledged.connect(func() -> void: acked[0] = true)
    if confirm != null:
        confirm.pressed.emit()
        await get_tree().process_frame
        _expect(acked[0] and not modal.visible, "点「知道了」应发出信号并收起")
    modal.hide_status()
    _expect(not modal.visible, "hide_status 应保持隐藏")
    host.queue_free()
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
    var gold: Label = river.find_child("RecruitGoldTile", true, false)
    _expect(turn != null and flip != null and timer != null and end_button != null, "招募河道节点应齐全")
    _expect(gold != null and gold.text.contains("6"), "招募河道左侧应显示金币")
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


## 战斗河道：始终只显示回合数，右端只剩「战斗中」。
func _check_combat_river() -> void:
    var host: Control = await _host(Vector2(1148, 32))
    var river := GameRiver.new(GameRiver.Mode.COMBAT)
    river.set_anchors_preset(Control.PRESET_FULL_RECT)
    host.add_child(river)
    await get_tree().process_frame
    river.refresh(_state_for(Protocol.PHASE_COMBAT, FixtureScript.game_view()))
    await get_tree().process_frame
    var turn_label: Label = river.find_child("CombatTurnLabel", true, false)
    var pill: Label = river.find_child("CombatStatusPill", true, false)
    var gold: Label = river.find_child("CombatGoldTile", true, false)
    _expect(turn_label != null and turn_label.visible, "战斗河道应始终显示回合数")
    _expect(gold != null and gold.text.contains("6"), "战斗河道左侧应显示金币")
    _expect(pill != null, "战斗河道右端应有战斗中胶囊")
    _expect(river.find_child("RecruitCountdownFlip", true, false) == null, "战斗河道不应创建倒计时")
    if turn_label != null and pill != null:
        _expect(
            turn_label.text == TranslationServer.translate("ui.game.side_panel.turn") % 3,
            "战斗河道应只显示回合数，实际「%s」" % turn_label.text,
        )
        _expect(pill.text == TranslationServer.translate("ui.game.side_panel.combat"), "右端应显示战斗中文案")
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
    var meta: Control = row.find_child("HeroMeta", true, false)
    var power: Control = row.find_child("HeroPowerButton0", true, false)
    _expect(card != null and card.custom_minimum_size == Vector2(84, 84), "英雄行应含 84×84 英雄卡")
    _expect(row.find_child("GoldTile", true, false) == null, "英雄行不应再显示金币")
    _expect(meta != null and meta.custom_minimum_size.y == 24, "英雄 meta 行应保持 24 高")
    _expect(power != null and power.custom_minimum_size == Vector2(40, 40), "英雄技能应为 40×40 图标按钮")
    var uses_badge: Label = row.find_child("HeroPower0UsesBadge", true, false)
    _expect(uses_badge != null and not uses_badge.visible, "剩余 1 次时不显示次数角标")

    var passive: Control = row.find_child("HeroPassive0", true, false)
    _expect(passive != null, "英雄行应显示被动技能标记")
    if passive != null:
        _expect(passive.custom_minimum_size == Vector2(24, 24), "被动技能应为 24×24 标记")
        var passive_text := TranslationServer.translate("skill.aura-biology-plus-1-plus-1.text")
        _expect(
            passive.tooltip_text.contains(passive_text),
            "被动技能 tooltip 应包含效果文本",
        )
        _expect(passive.modulate == Color.WHITE, "被动技能不应置灰")

    var empty_power := _state_for(Protocol.PHASE_RECRUIT, view)
    empty_power.self_state["heroPowers"][0]["usesLeft"] = 0
    empty_power.self_state["heroPowers"][0]["canUse"] = false
    row.refresh(empty_power)
    await get_tree().process_frame
    _expect(uses_badge != null and uses_badge.visible and uses_badge.text == "0", "0 次时应显示次数角标")
    _expect(power.disabled, "不可用时技能按钮应禁用")
    _expect(passive != null and passive.modulate == Color.WHITE, "主动技能禁用时被动技能仍不置灰")

    row.set_combat_mode(true)
    _expect(not power.visible, "战斗阶段应隐藏技能按钮")
    _expect(card.visible, "战斗阶段英雄卡应保留")
    _expect(passive != null and passive.is_visible_in_tree(), "战斗阶段被动技能标记应保留")

    var dead_view := FixtureScript.game_view()
    dead_view["players"][0]["alive"] = false
    row.refresh(_state_for(Protocol.PHASE_COMBAT, dead_view))
    await get_tree().process_frame
    var eliminated: Label = row.find_child("EliminatedLabel", true, false)
    _expect(eliminated != null and eliminated.visible, "本人淘汰时应显示观战提示")
    _expect(not card.visible, "本人淘汰时应隐藏英雄卡")
    _expect(passive == null or not passive.is_visible_in_tree(), "本人淘汰时应隐藏被动技能标记")

    # 多个主动技能：按视图顺序各出一个按钮。
    row.set_combat_mode(false)
    var multi_view := FixtureScript.game_view()
    multi_view["self"]["heroCardId"] = "einstein"
    multi_view["self"]["heroPowers"] = [
        {
            "skillId": "mass-energy-equivalence",
            "usesLeft": 1,
            "requiresTarget": false,
            "targetCandidates": [],
            "canUse": true,
            "disabledReason": null,
        },
        {
            "skillId": "light-speed-strike",
            "usesLeft": 1,
            "requiresTarget": true,
            "targetCandidates": [{"zone": "board", "id": "mi_1"}],
            "canUse": true,
            "disabledReason": null,
        },
    ]
    var events: Array[String] = []
    row.hero_power_pressed.connect(func(skill_id: String) -> void: events.append("pressed:" + skill_id))
    row.hero_power_down.connect(func(skill_id: String) -> void: events.append("down:" + skill_id))
    row.refresh(_state_for(Protocol.PHASE_RECRUIT, multi_view))
    await get_tree().process_frame
    var first_power: BaseButton = row.find_child("HeroPowerButton0", true, false)
    var second_power: BaseButton = row.find_child("HeroPowerButton1", true, false)
    _expect(second_power != null, "两个主动技能应各有一个按钮")
    _expect(row.find_child("HeroPassive0", true, false) == null, "无被动技能时不应保留被动标记")
    if first_power != null and second_power != null:
        first_power.pressed.emit()
        second_power.button_down.emit()
        _expect(
            events == ["pressed:mass-energy-equivalence", "down:light-speed-strike"],
            "两个主动技能应各自发出带 skill_id 的信号",
        )

    # 七下西洋：tooltip 展示航程剩余回合与已累计金币。
    var voyage_view := FixtureScript.game_view()
    voyage_view["self"]["heroCardId"] = "zhenghe"
    voyage_view["self"]["heroPowers"] = [
        {
            "skillId": "free-refresh",
            "usesLeft": 1,
            "requiresTarget": false,
            "targetCandidates": [],
            "canUse": true,
            "disabledReason": null,
            "voyageRemaining": null,
            "voyagePending": 0,
        },
        {
            "skillId": "seven-voyages",
            "usesLeft": 1,
            "requiresTarget": false,
            "targetCandidates": [],
            "canUse": false,
            "disabledReason": "SEVEN_VOYAGES_ACTIVE",
            "voyageRemaining": 3,
            "voyagePending": 12,
        },
    ]
    row.refresh(_state_for(Protocol.PHASE_RECRUIT, voyage_view))
    await get_tree().process_frame
    var voyage_button: Control = row.find_child("HeroPowerButton1", true, false)
    _expect(voyage_button != null, "郑和应显示两个主动技能按钮")
    if voyage_button != null:
        var voyage_text: String = tr("ui.game.side_panel.voyage_status") % [3, 12]
        _expect(voyage_button.tooltip_text.contains(voyage_text), "七下西洋 tooltip 应显示航程进度")
        _expect(voyage_button.disabled, "航程进行中七下西洋不可用")
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
    var host: Control = await _host(Vector2(1148, 236))
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
    _expect(actions != null and actions.get_global_rect().size.y == 84, "商店操作行高应为 84")
    _expect(upgrade != null and upgrade.custom_minimum_size == Vector2(40, 40), "升级按钮应为 40×40")
    _expect(seller != null and seller is GameHeroCard, "售货员应复用英雄卡组件")
    _expect(seller != null and seller.custom_minimum_size == Vector2(84, 84), "售货员应为 84×84")
    _expect(
        seller != null and not (seller.find_child("HeroHpPlate", true, false) as Control).visible,
        "售货员不应显示生命牌",
    )
    _expect(seller != null and seller.find_child("AvatarFrameArt", true, false) != null, "售货员应显示立绘贴图")
    _expect(
        seller != null and seller.find_child("PortraitName", true, false) == null,
        "售货员不应再显示名字条",
    )
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

    # 跨回合保留冻结但没有解冻权：按钮仍显示「冻结」，按下也只是再次冻结。
    var carried := _base_view(Protocol.PHASE_RECRUIT)
    carried["self"]["affordances"]["canUnfreeze"] = false
    instance = await _mount(carried)
    if instance == null:
        return
    freeze = instance.find_child("FreezeButton", true, false)
    _expect(freeze != null, "冻结按钮应存在")
    if freeze == null:
        await _unmount(instance)
        return
    _expect(
        freeze.tooltip_text == TranslationServer.translate("ui.game.side_panel.freeze"),
        "跨回合保留冻结时 tooltip 应为冻结",
    )
    captured = []
    instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
    await _push_click(instance, freeze.get_global_rect().get_center())
    _expect(
        captured.size() == 1
            and str(captured[0].get("kind", "")) == Protocol.INTENT_FREEZE_SHOP
            and captured[0].get("slot") == null,
        "跨回合保留冻结时应发出整店冻结意图，实际 %s" % [captured],
    )
    await _unmount(instance)

    var thawed := _base_view(Protocol.PHASE_RECRUIT)
    for slot in thawed["self"]["shop"]:
        slot["frozen"] = false
    thawed["self"]["affordances"]["canUnfreeze"] = false
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
    var host: Control = await _host(Vector2(1148, 438))
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
        absf(view.get_combined_minimum_size().y - 438.0) < 1.0,
        "招募组合总高应为 438，实际 %s" % view.get_combined_minimum_size().y,
    )
    var shop: Control = view.find_child("ShopView", true, false)
    var river: Control = view.find_child("RecruitRiver", true, false)
    var board: Control = view.find_child("BoardPanel", true, false)
    _expect(shop != null and shop.get_global_rect().size.y == 236, "商店块高应为 236")
    _expect(river != null and river.get_global_rect().size.y == 44, "河道高应为 44")
    var river_offset := -1.0
    if river != null:
        river_offset = river.get_global_rect().position.y - view.get_global_rect().position.y
    _expect(
        is_equal_approx(river_offset, 242.0),
        "招募河道应位于组合顶部下方 242，实际 %.1f" % river_offset,
    )
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
    var host: Control = await _host(Vector2(1148, 84))
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
        portrait != null and portrait is GameHeroCard,
        "对手头像应复用英雄卡组件",
    )
    _expect(
        portrait != null and portrait.custom_minimum_size == Vector2(84, 84),
        "对手英雄卡应为 84×84",
    )
    _expect(
        _node_text(opponent).contains("机器人C"),
        "对手视图应显示展示名，实际「%s」" % _node_text(opponent),
    )
    _expect(opponent.find_child("HpBar", true, false) != null, "普通对手应有血量条")
    var hp_plate: Control = portrait.find_child("HeroHpPlate", true, false) if portrait != null else null
    _expect(hp_plate != null and hp_plate.visible, "普通对手英雄卡应显示生命牌")
    _expect(
        portrait != null and portrait.find_child("AvatarFrameArt", true, false) != null,
        "对手头像有素材时应显示头像图",
    )

    var ghost_view := FixtureScript.game_view()
    ghost_view["opponentPlayerId"] = "p2"
    GameStore.phase = Protocol.PHASE_COMBAT
    _set_combat({"mirror": true})
    opponent.refresh(_state_for(Protocol.PHASE_COMBAT, ghost_view))
    await get_tree().process_frame
    _expect(opponent.find_child("GhostIdentity", true, false) != null, "幽灵战应显示幽灵身份块")
    _expect(opponent.find_child("GhostBadge", true, false) != null, "幽灵战应有幽灵角标")
    _expect(opponent.find_child("HpBar", true, false) == null, "幽灵战不应显示血量条")
    var ghost_portrait: Control = opponent.find_child("OpponentPortrait", true, false)
    var ghost_plate: Control = (
        ghost_portrait.find_child("HeroHpPlate", true, false) if ghost_portrait != null else null
    )
    _expect(ghost_plate != null and not ghost_plate.visible, "幽灵战英雄卡应隐藏生命牌")
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
    _fake_connect()
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
    _expect(instance.find_child("TopBar", true, false) == null, "顶部工具条不应存在")
    _expect(instance.find_child("StatusLabel", true, false) == null, "对局页不应再有状态文本标签")
    var tools: Control = instance.find_child("GameTools", true, false)
    _expect(
        tools != null
            and tools.find_child("GalleryButton", true, false) != null
            and tools.find_child("HintButton", true, false) != null
            and tools.find_child("LeaveGameButton", true, false) != null,
        "右上角工具组应含图鉴、提示与退出对局",
    )
    var status_modal: Control = instance.find_child("StatusModal", true, false)
    _expect(
        status_modal != null and not status_modal.visible,
        "状态模态应存在且默认隐藏",
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

    # 英雄行 84，英雄卡 84×84；头像铺满，右下角只显示当前生命。
    var hero_row: Control = instance.find_child("HeroRow", true, false)
    var hero_card: Control = instance.find_child("HeroCard", true, false)
    _expect(
        hero_row != null and is_equal_approx(hero_row.size.y, 84.0),
        "英雄行高应为 84，实际 %.1f" % (0.0 if hero_row == null else hero_row.size.y),
    )
    _expect(
        hero_card != null and hero_card.size == Vector2(84, 84),
        "英雄卡应为 84×84，实际 %s" % ("null" if hero_card == null else str(hero_card.size)),
    )
    if hero_card != null:
        var hp: Label = hero_card.find_child("HeroHp", true, false)
        _expect(
            hp != null
                and hero_card.find_child("HeroCardName", true, false) == null
                and hero_card.find_child("HeroEra", true, false) == null,
            "英雄卡只保留生命，不再显示名字与时代",
        )
        _expect(
            hero_card.find_child("HeroAttack", true, false) == null,
            "英雄卡不应再显示攻击力",
        )
        if hp != null:
            _expect(hp.text == "35", "生命只显示当前值，实际「%s」" % hp.text)
            var plate := hero_card.find_child("HeroHpPlate", true, false) as Control
            _expect(
                plate != null
                    and plate.get_global_rect().get_center().x
                        > hero_card.get_global_rect().get_center().x
                    and plate.get_global_rect().get_center().y
                        > hero_card.get_global_rect().get_center().y,
                "生命牌应贴英雄卡右下角",
            )

    var power: Control = instance.find_child("HeroPowerButton0", true, false)
    _expect(power != null and power.size == Vector2(40, 40), "英雄技能图标应为 40×40")
    if power != null:
        _expect(power.find_child("HeroPower0CostBadge", true, false) != null, "技能应有费用角标")
        _expect(power.find_child("HeroPower0UsesBadge", true, false) != null, "技能应有次数角标")
    var gold_tile: Label = instance.find_child("RecruitGoldTile", true, false)
    _expect(gold_tile != null and gold_tile.text.contains("6"), "招募河道应显示金币")

    # 商店操作行：升时代在售货员左侧，刷新与冻结在右侧，均为 40×40 图标按钮。
    var seller: Control = instance.find_child("SellerTile", true, false)
    var upgrade: Control = instance.find_child("UpgradeButton", true, false)
    var reroll: Control = instance.find_child("RerollButton", true, false)
    var freeze: Control = instance.find_child("FreezeButton", true, false)
    _expect(seller != null and seller.size == Vector2(84, 84), "售货员应为 84×84")
    _expect(
        seller != null and seller.find_child("PortraitName", true, false) == null,
        "售货员不应再显示名字条",
    )
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
    _fake_disconnect()


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


## 真机触控端一次触摸会同时产生触摸与模拟鼠标事件；结束招募只能发一条意图。
func _check_countdown_touch_single_intent() -> void:
    ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
    ScreenProfile.set_native_touch_for_test(true)
    var view := _base_view(Protocol.PHASE_RECRUIT)
    var instance := await _mount(view)
    if instance == null:
        _restore_touch_test_input()
        return

    var flip: Control = instance.find_child("RecruitCountdownFlip", true, false)
    _expect(flip != null, "河道右端应有倒计时翻转区域")
    if flip != null:
        var captured: Array = []
        instance.intent_sent.connect(func(intent: Dictionary) -> void: captured.append(intent))
        var center := flip.get_global_rect().get_center()
        await _push_motion(instance, center)
        # 真机顺序：模拟鼠标按下先到，随后是触摸按下；两条都带同一次按压。
        await _push_button(instance, center, true)
        var touch := InputEventScreenTouch.new()
        touch.index = 0
        touch.position = center
        touch.pressed = true
        instance.get_viewport().push_input(touch, true)
        await get_tree().process_frame
        _expect(
            captured.size() == 1
                and str(captured[0].get("kind", "")) == Protocol.INTENT_END_RECRUIT,
            "真机触控一次触摸只应发一条结束招募意图，实际 %s" % [captured],
        )

    await _unmount(instance)
    _restore_touch_test_input()


func _restore_touch_test_input() -> void:
    ScreenProfile.set_native_touch_for_test(null)
    ScreenProfile.set_device_for_test("")


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
    var power: Control = instance.find_child("HeroPowerButton0", true, false)
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
