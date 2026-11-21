class_name DetailLayer
extends Control
## 随从详情浮层：覆盖全屏但不拦截输入，层级位于内容与战报之上、选择浮层之下。
##
## 卡片通过 `track()` 登记，本层用 `_input` 做跨子控件的矩形命中判断；
## 模态浮层打开时由页面把 `blocked` 置真，本层直接隐藏且不再响应悬停。
## 各详情内容由 `ui/detail_sections/` 的单文件组件渲染，本层只做命中、定位与装配。

## 详情面板的挂载方向：商店卡在左右，棋盘卡在上下。
const DETAIL_SIDES := {
    "shop": ["right", "left"],
    "seat": ["right", "left"],
    "minion": ["bottom", "top"],
    "seller": ["bottom", "top"],
    "hero": ["left", "right"],
}
## 触控长按判定时长与位移取消阈值（画布单位），与选英雄页共用同一套取值。
const LONG_PRESS_MS := LongPressTracker.DEFAULT_LONG_PRESS_MS
const TOUCH_CANCEL_DISTANCE := LongPressTracker.DEFAULT_CANCEL_DISTANCE

## 模态浮层打开时置真：详情层会盖住浮层内容，直接隐藏且不再响应悬停。
var blocked := false
## 左栏战报浮层打开时置真：战报优先，悬停详情不再抢占画面。
var report_open := false

var _panel: PanelContainer
var _cards: Array = []
var _hover_card: Control = null
## 触控长按状态：只跟踪第一个触点；`fired` 表示长按已触发显示。
var _touch := LongPressTracker.new()


func _init() -> void:
    name = "MinionDetailLayer"
    set_anchors_preset(Control.PRESET_FULL_RECT)
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    visible = false

    _panel = UIControls.card_panel("raised")
    _panel.name = "MinionDetailPanel"
    _panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _panel.custom_minimum_size = Vector2(UITheme.stage_metric("detail-panel-width"), 0)
    _panel.visible = false
    add_child(_panel)


## 卡片统一挂详情：悬停进入/离开、鼠标左键按下/松开、触摸按下/松开。
## `data.detail_kind` 决定详情内容（shop / minion）与面板挂载方向。
## 卡内控件全部 IGNORE 鼠标，祖先悬停通知会被子控件截断，
## 因此真正的矩形命中判断放在 `_input`，这里只保留就近的进入/离开兜底。
func track(card: Control, data: Dictionary) -> void:
    # 重建时旧行已 queue_free；登记新行前裁掉已释放的条目，避免无限累积。
    _cards = _cards.filter(func(entry): return is_instance_valid(entry))
    card.set_meta("detail_data", data)
    _cards.append(card)
    # 常驻节点每轮刷新都会重新登记：信号只连一次，避免重复连接告警。
    if card.has_meta("detail_bound"):
        return
    card.set_meta("detail_bound", true)
    card.mouse_entered.connect(_on_card_mouse_entered.bind(data, card))
    card.gui_input.connect(_on_card_gui_input.bind(data, card))


## 鼠标悬停进入卡片：触控端由长按状态机驱动，触摸模拟出的悬停不直接显示。
func _on_card_mouse_entered(data: Dictionary, card: Control) -> void:
    if ScreenProfile.is_touch():
        return
    _show_detail(data, card)


## 拖拽开始即收起详情：拖拽途中的悬停经过不再显示别的卡，落点或取消后由悬停重新触发。
func _notification(what: int) -> void:
    if what == NOTIFICATION_DRAG_BEGIN:
        hide_panel()


## 每次刷新前清空登记并隐藏当前面板。
func reset() -> void:
    _clear_touch()
    hide_panel()
    _cards.clear()


## 隐藏详情层与面板，并清空当前悬停卡。
func hide_panel() -> void:
    if _panel != null:
        _panel.visible = false
    visible = false
    _hover_card = null


func _on_card_gui_input(event: InputEvent, data: Dictionary, card: Control) -> void:
    # 触控端由 `_input` 的长按状态机统一驱动，同时忽略触摸模拟出的鼠标事件，
    # 避免「按下即显示」；桌面端保持按下显示、松开重新命中的原有行为。
    if ScreenProfile.is_touch():
        return
    if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        if event.pressed:
            _show_detail(data, card)
        else:
            _update_detail_hover(_local_point(card.get_global_rect().position) + event.position)
    elif event is InputEventScreenTouch:
        if event.pressed:
            _show_detail(data, card)
        else:
            _update_detail_hover(_local_point(card.get_global_rect().position) + event.position)


## 跨子控件的真实命中判断：指针落在卡片矩形内即显示，移出才隐藏。
func _input(event: InputEvent) -> void:
    if ScreenProfile.is_touch():
        _input_touch(event)
        return
    if event is InputEventMouseMotion:
        _update_detail_hover(_canvas_point(event.position))
    elif event is InputEventScreenTouch:
        _update_detail_hover(_canvas_point(event.position))
    elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
        _update_detail_hover(_canvas_point(event.position))


## 触控端输入：忽略模拟鼠标，只由第一个触点驱动长按状态机。
func _input_touch(event: InputEvent) -> void:
    if event is InputEventScreenTouch:
        if event.pressed:
            _begin_touch(event)
        else:
            _end_touch(event)
    elif event is InputEventScreenDrag:
        _drag_touch(event)


## 按下只登记触点与卡片，不立即显示；多触点只认第一个。
func _begin_touch(event: InputEventScreenTouch) -> void:
    if _touch.active() or blocked or report_open:
        return
    var point := _canvas_point(event.position)
    var card := _card_at(point)
    if card == null:
        return
    _touch.begin(event.index, point, card)
    set_process(true)


## 位移超过 8 个画布单位即取消长按，不再显示。
func _drag_touch(event: InputEventScreenDrag) -> void:
    if not _touch.matches(event.index) or _touch.fired:
        return
    if _touch.dragged(event.index, _canvas_point(event.position)):
        _clear_touch()


## 抬手：已显示则隐藏；未到长按时长的短按不改变原有单击行为。
func _end_touch(event: InputEventScreenTouch) -> void:
    if not _touch.matches(event.index):
        return
    if _touch.fired:
        hide_panel()
    _clear_touch()


## 长按判定放在 `_process` 中按 delta 累加，不依赖具体帧率。
func _process(delta: float) -> void:
    if not _touch.active() or _touch.fired:
        return
    if blocked or report_open or not is_instance_valid(_touch.target):
        _clear_touch()
        return
    if not _touch.advance(delta):
        return
    _show_detail(_touch.target.get_meta("detail_data", {}), _touch.target)


func _clear_touch() -> void:
    _touch.cancel()
    set_process(false)


## 视口坐标转为本层局部坐标，与转换后的卡片矩形同一坐标系。
func _canvas_point(viewport_point: Vector2) -> Vector2:
    return _local_point(get_viewport().get_canvas_transform().affine_inverse() * viewport_point)


func _local_point(global: Vector2) -> Vector2:
    return get_global_transform().affine_inverse() * global


## 全局画布矩形转本层局部矩形；尺寸按本层全局缩放换算。
func _local_rect(global: Rect2) -> Rect2:
    if global.size == Vector2.ZERO:
        return Rect2()
    var scale := get_global_transform().get_scale().x
    return Rect2(_local_point(global.position), global.size / scale)


## 当前是否有卡片正在拖拽；拖拽期间详情层不参与悬停命中。
func _is_dragging() -> bool:
    var viewport := get_viewport()
    return viewport != null and viewport.gui_is_dragging()


## 局部坐标命中的第一张卡片；未登记或已释放的条目直接跳过。
func _card_at(point: Vector2) -> Control:
    for card in _cards:
        if is_instance_valid(card) and card.is_visible_in_tree() and _local_rect(card.get_global_rect()).has_point(point):
            return card
    return null


func _update_detail_hover(point: Vector2) -> void:
    # 模态浮层打开时详情层会盖住浮层内容，直接隐藏且不再响应悬停。
    if blocked or report_open:
        hide_panel()
        return
    var found := _card_at(point)
    if found == null:
        if _hover_card != null:
            hide_panel()
        return
    if found == _hover_card and _panel != null and _panel.visible:
        return
    _show_detail(found.get_meta("detail_data", {}), found)


func _show_detail(data: Dictionary, anchor: Control) -> void:
    if blocked or report_open:
        return
    # 拖拽进行中不显示悬停详情：经过的卡与本次拖拽无关，弹出会遮挡并误导落点。
    if _is_dragging():
        hide_panel()
        return
    if _panel == null or anchor == null or not is_instance_valid(anchor):
        return
    var kind := str(data.get("detail_kind", "minion"))
    # 同一张卡已在展示：只校正位置，避免每次按下/移入都重建整块面板。
    # `reset()` 会先隐藏面板，因此数据更新仍会触发重建。
    if anchor == _hover_card and _panel.visible:
        _place_detail(_local_rect(anchor.get_global_rect()), kind)
        return
    if not is_instance_valid(_hover_card) or _hover_card != anchor:
        _hover_card = anchor
    UIControls.clear(_panel)
    var box := VBoxContainer.new()
    box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    _panel.add_child(box)

    if kind == "shop":
        _render_shop_detail(box, data)
    elif kind == "seat":
        _render_seat_detail(box, data.get("player", {}), int(data.get("hpCap", 0)))
    elif kind == "seller":
        _render_seller_detail(box)
    elif kind == "hero":
        _render_hero_detail(box, data.get("heroId"))
    else:
        _render_minion_detail(box, data.get("minion", {}))

    visible = true
    _panel.visible = true
    _place_detail(_local_rect(anchor.get_global_rect()), kind)


## 商店卡详情：类型、科学分类、玩法、关联分类、费用、原始数值、时代、
## 技能文本与不可购买原因。
func _render_shop_detail(box: VBoxContainer, data: Dictionary) -> void:
    ShopDetailSection.render(box, data, _kit())


## 座位详情：名字、英雄、生命（当前/上限）、时代、随从数与状态。
func _render_seat_detail(box: VBoxContainer, player: Dictionary, hp_cap: int) -> void:
    SeatDetailSection.render(box, player, hp_cap, _kit())


## 售货员详情：营业状态与可执行的商店操作，替代常驻文字。
func _render_seller_detail(box: VBoxContainer) -> void:
    SellerDetailSection.render(box, _kit())


## 我方英雄详情：介绍、玩法、关联分类与英雄技能；英雄卡不含科学分类与时代。
func _render_hero_detail(box: VBoxContainer, hero_id: Variant) -> void:
    HeroDetailSection.render(box, hero_id, _kit())


## 棋盘随从详情：名称、科学分类、玩法、关联分类与攻击/生命的构成分层。
func _render_minion_detail(box: VBoxContainer, minion: Dictionary) -> void:
    MinionDetailSection.render(box, minion, _kit())


## 当前详情块的公共排版；文本宽度按面板令牌与子类覆写计算。
func _kit() -> DetailKit:
    return DetailKit.new(_detail_text_width())


func _detail_name(text: String) -> Label:
    return _kit().name_label(text)


## 面板宽度扣除卡片内边距后的文本宽度，供技能文本换行。
func _detail_text_width() -> int:
    return UITheme.stage_metric("detail-panel-width") - UITheme.space("space-lg") * 2


## 详情里的介绍/技能文本块：按面板宽度换行，颜色弱于正文。
func _detail_text_block(text: String) -> Label:
    return _kit().text_block(text)


## 详情定位：商店卡先右后左，棋盘卡先下后上；夹紧内容框且不覆盖当前卡片。
func _place_detail(card_rect: Rect2, kind: String) -> void:
    var bounds := size
    var panel_size := _panel.get_combined_minimum_size()
    var margin := UITheme.space("space-xs")
    var gap := UITheme.space("space-sm")
    var fallback := Vector2.ZERO
    var has_fallback := false
    for side in DETAIL_SIDES.get(kind, DETAIL_SIDES["minion"]):
        var pos := Vector2.ZERO
        match side:
            "right":
                pos = Vector2(card_rect.end.x + gap, card_rect.position.y)
            "left":
                pos = Vector2(card_rect.position.x - panel_size.x - gap, card_rect.position.y)
            "bottom":
                pos = Vector2(card_rect.position.x, card_rect.end.y + gap)
            _:
                pos = Vector2(card_rect.position.x, card_rect.position.y - panel_size.y - gap)
        pos.x = clampf(pos.x, margin, maxf(margin, bounds.x - panel_size.x - margin))
        pos.y = clampf(pos.y, margin, maxf(margin, bounds.y - panel_size.y - margin))
        if not has_fallback:
            fallback = pos
            has_fallback = true
        if not Rect2(pos, panel_size).intersects(card_rect):
            _panel.position = pos
            return
    _panel.position = fallback
