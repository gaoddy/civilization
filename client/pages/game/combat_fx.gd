class_name CombatFx
extends Control
## 战斗演出覆盖层：攻方克隆卡碰撞往返、命中闪光、伤害飘字与阵亡淡出。
##
## 只做视觉：数值全部来自服务端动作快照，本层不做任何推算。演出由页面每帧
## 调用 [`advance`] 推进，不用 `Tween`，这样无头冒烟测试可以按帧断言。
##
## 覆盖层 `mouse_filter = IGNORE` 且不参与容器布局；克隆卡一律忽略鼠标，
## 避免抢走原卡与详情层的悬停。

## 攻方单程飞行时长（毫秒）；往返共两程，需小于播放器单条动作节奏 `PACE_MS`。
const FLY_MS := 150.0

## 阵亡卡淡出时长（毫秒）；与淡入一样短于 320ms 的动作节拍，不跨到下一条动作。
const FADE_MS := 260.0

## 复生 / 召唤卡的缩放淡入时长（毫秒）；同样短于动作节拍。
const APPEAR_MS := 260.0

## 命中闪光回正时长（毫秒）。
const FLASH_MS := 180.0

## 伤害飘字上浮与淡出时长（毫秒）。
const DAMAGE_MS := 950.0
## 命中火花数量与寿命（毫秒）；火花直接在覆盖层上绘制，不建节点。
const SPARK_COUNT := 20
const SPARK_MS := 650.0
## 命中冲击环的扩散时长与最大半径（像素）。
const RING_MS := 520.0
const RING_RADIUS := 38.0
## 受击卡的挤压回弹时长（毫秒）。
const PUNCH_MS := 280.0
## 伤害数字上浮距离与初始放大倍数。
const DAMAGE_RISE := 46.0
const DAMAGE_POP := 1.6
## 火花初速区间与重力，取值固定，保证无头冒烟逐帧结果一致。
const SPARK_SPEED_MIN := 130.0
const SPARK_SPEED_MAX := 240.0
const SPARK_GRAVITY := 520.0

## 飞行中的克隆卡：[{node, start, end, source, target, damage, t, hit}]。
var _flights: Array = []
## 淡出中的克隆卡：[{node, t}]。
var _fades: Array = []
## 缩放淡入中的原卡：[{node, t}]。
var _appears: Array = []
## 闪光中的原卡：[{node, t}]。
var _flashes: Array = []
## 伤害飘字：[{node, t, start}]。
var _damage_labels: Array = []
## 命中火花：[{pos, vel, t, size}]，全部按帧推进后在本层 `_draw` 里画。
var _sparks: Array = []
## 命中冲击环：[{center, t}]。
var _rings: Array = []
## 受击卡的挤压回弹：[{node, t}]。
var _punches: Array = []


func _init() -> void:
    name = "CombatFxLayer"
    mouse_filter = Control.MOUSE_FILTER_IGNORE


## 攻方从原位飞向目标并返回；返回后释放克隆、恢复原卡。
##
## 原卡在起飞时隐藏（保留占位，避免行内重排），原位矩形此刻抓取；即使攻方
## 随后阵亡、行内重排，克隆仍飞回被抓取的位置。
func play_attack(source: Control, target: Control, damage: int) -> void:
    if source == null or target == null:
        return
    var start := _local_point(source.global_position)
    var end := _local_point(target.global_position) + (target.size - source.size) * 0.5
    var clone := _clone_card(source, "CombatFxAttack")
    if clone == null:
        return
    clone.position = start
    source.modulate.a = 0.0
    _flights.append({
        "node": clone,
        "start": start,
        "end": end,
        "source": source,
        "target": target,
        "damage": maxi(damage, 0),
        "t": 0.0,
        "hit": false,
    })


## 阵亡卡在原位淡出：立刻隐藏原卡（交给行内回收），克隆按原位淡到透明。
func fade_out(card: Control) -> void:
    if card == null:
        return
    var clone := _clone_card(card, "CombatFxFade")
    if clone == null:
        return
    clone.position = _local_point(card.global_position)
    card.modulate.a = 0.0
    _fades.append({"node": clone, "t": 0.0})


## 复生 / 召唤卡出现：原卡缩放淡入。
func appear(card: Control) -> void:
    if card == null:
        return
    card.modulate.a = 0.0
    card.pivot_offset = card.size * 0.5
    card.scale = Vector2(0.85, 0.85)
    _appears.append({"node": card, "t": 0.0})


## 卡片短暂变色提示（传染等非碰撞动作）。
func flash(card: Control) -> void:
    if card == null:
        return
    _flashes.append({"node": card, "t": 0.0})


## 清理全部演出节点与进行中的动画。
func clear() -> void:
    for flight in _flights:
        _free_node(flight["node"])
    for fade in _fades:
        _free_node(fade["node"])
    for label in _damage_labels:
        _free_node(label["node"])
    for appear in _appears:
        _reset_card(appear["node"])
    for flash in _flashes:
        _reset_card(flash["node"])
    for punch in _punches:
        _reset_card(punch["node"])
    _flights.clear()
    _fades.clear()
    _appears.clear()
    _flashes.clear()
    _damage_labels.clear()
    _sparks.clear()
    _rings.clear()
    _punches.clear()
    for child in get_children():
        child.queue_free()


## 按帧推进全部演出；由页面在演示播放期间调用。
func advance(delta: float) -> void:
    var delta_ms := delta * 1000.0
    _advance_flights(delta_ms)
    _advance_fades(delta_ms)
    _advance_appears(delta_ms)
    _advance_flashes(delta_ms)
    _advance_damage_labels(delta_ms)
    _advance_sparks(delta_ms)
    _advance_rings(delta_ms)
    _advance_punches(delta_ms)
    queue_redraw()


func _advance_flights(delta_ms: float) -> void:
    var index := _flights.size() - 1
    while index >= 0:
        var flight: Dictionary = _flights[index]
        flight["t"] += delta_ms
        var clone: Control = flight["node"]
        if not is_instance_valid(clone):
            _flights.remove_at(index)
            index -= 1
            continue
        var t := float(flight["t"])
        if t <= FLY_MS:
            clone.position = flight["start"].lerp(flight["end"], t / FLY_MS)
        elif t <= FLY_MS * 2.0:
            if not bool(flight["hit"]):
                flight["hit"] = true
                _strike(flight)
            clone.position = flight["end"].lerp(flight["start"], (t - FLY_MS) / FLY_MS)
        else:
            _free_node(clone)
            _restore_source(flight["source"])
            _flights.remove_at(index)
        index -= 1


## 命中反馈：目标闪光并飘出伤害数字。
func _strike(flight: Dictionary) -> void:
    var target: Control = flight["target"]
    var damage := int(flight["damage"])
    if is_instance_valid(target) and damage > 0:
        flash(target)
        var hit: Vector2 = flight["end"] + target.size * 0.5
        _spawn_impact(hit)
        _spawn_damage_label(hit, damage)
        _punches.append({"node": target, "t": 0.0})


## 命中冲击：一圈扩散环 + 一束向外飞散的火花，让「打到了」有重量。
func _spawn_impact(center: Vector2) -> void:
    _rings.append({"center": center, "t": 0.0})
    for index in SPARK_COUNT:
        var spread := float(index) / float(SPARK_COUNT)
        var angle := -PI * 0.5 + (spread - 0.5) * PI * 1.5
        var speed := lerpf(SPARK_SPEED_MIN, SPARK_SPEED_MAX, fmod(spread * 3.7, 1.0))
        _sparks.append({
            "pos": center,
            "vel": Vector2(cos(angle), sin(angle)) * speed,
            "t": 0.0,
            "size": 1.8 + 1.4 * fmod(spread * 5.3, 1.0),
        })


## 伤害数字：深色底牌 + 亮红数字，压在立绘上也一眼能读到。
func _spawn_damage_label(center: Vector2, damage: int, label_name := "CombatFxDamage") -> void:
    var holder := PanelContainer.new()
    holder.name = label_name
    holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
    var plate := StyleBoxFlat.new()
    plate.bg_color = UITheme.alpha("color-bg", 0.78)
    plate.border_color = UITheme.alpha("color-danger", 0.55)
    plate.set_border_width_all(1)
    plate.set_corner_radius_all(9)
    plate.set_content_margin_all(5)
    holder.add_theme_stylebox_override("panel", plate)

    # 关闭自动换行：底牌宽度按内容增长，负号与数字不会被拆成两行。
    var label := UIControls.label("-%d" % damage, UITheme.font_size("font-hero") * 1.35, false)
    label.add_theme_color_override("font_color", UITheme.color("color-damage"))
    label.add_theme_constant_override("outline_size", 5)
    label.add_theme_color_override("font_outline_color", UITheme.alpha("color-black", 0.85))
    label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    holder.add_child(label)

    add_child(holder)
    holder.reset_size()
    var start := center - holder.size * 0.5
    holder.position = start
    _damage_labels.append({"node": holder, "t": 0.0, "start": start})


func _advance_sparks(delta_ms: float) -> void:
    var step := delta_ms / 1000.0
    var index := _sparks.size() - 1
    while index >= 0:
        var spark: Dictionary = _sparks[index]
        spark["t"] += delta_ms
        if float(spark["t"]) >= SPARK_MS:
            _sparks.remove_at(index)
            index -= 1
            continue
        var vel: Vector2 = spark["vel"]
        vel.y += SPARK_GRAVITY * step
        spark["vel"] = vel
        spark["pos"] = (spark["pos"] as Vector2) + vel * step
        index -= 1


## 受击挤压：先压到 0.92 再弹回 1.0，命中瞬间有被推了一下的重量。
func _advance_punches(delta_ms: float) -> void:
    var index := _punches.size() - 1
    while index >= 0:
        var punch: Dictionary = _punches[index]
        punch["t"] += delta_ms
        var card: Control = punch["node"]
        if not is_instance_valid(card):
            _punches.remove_at(index)
            index -= 1
            continue
        var k := clampf(float(punch["t"]) / PUNCH_MS, 0.0, 1.0)
        var scale := 1.0 - 0.08 * sin(k * PI)
        card.pivot_offset = card.size * 0.5
        card.scale = Vector2(scale, scale)
        if k >= 1.0:
            _reset_card(card)
            _punches.remove_at(index)
        index -= 1


func _advance_rings(delta_ms: float) -> void:
    var index := _rings.size() - 1
    while index >= 0:
        var ring: Dictionary = _rings[index]
        ring["t"] += delta_ms
        if float(ring["t"]) >= RING_MS:
            _rings.remove_at(index)
        index -= 1


## 火花与冲击环直接画在覆盖层上：数量固定、无随机，无头冒烟可逐帧断言。
func _draw() -> void:
    for spark in _sparks:
        var k := clampf(float(spark["t"]) / SPARK_MS, 0.0, 1.0)
        var color := UITheme.fade(UITheme.color("color-accent"), (1.0 - k) * 0.95)
        draw_circle(spark["pos"], float(spark["size"]) * (1.0 - k * 0.6), color)
    for ring in _rings:
        var k := clampf(float(ring["t"]) / RING_MS, 0.0, 1.0)
        var fade := (1.0 - k) * (1.0 - k)
        # 冲击核：命中头 30% 的一团白光。
        draw_circle(ring["center"], lerpf(5.0, 16.0, k), UITheme.fade(UITheme.color("color-accent").lightened(0.45), fade * 0.75))
        draw_circle(ring["center"], lerpf(3.0, 10.0, k), UITheme.alpha("color-white", fade * 0.9))
        draw_arc(
            ring["center"],
            lerpf(6.0, RING_RADIUS, k),
            0.0,
            TAU,
            32,
            UITheme.fade(UITheme.color("color-accent"), fade * 0.85),
            2.5,
            true,
        )


func _advance_fades(delta_ms: float) -> void:
    var index := _fades.size() - 1
    while index >= 0:
        var fade: Dictionary = _fades[index]
        fade["t"] += delta_ms
        var clone: Control = fade["node"]
        if not is_instance_valid(clone):
            _fades.remove_at(index)
            index -= 1
            continue
        var k := clampf(float(fade["t"]) / FADE_MS, 0.0, 1.0)
        clone.modulate.a = 1.0 - k
        if k >= 1.0:
            _free_node(clone)
            _fades.remove_at(index)
        index -= 1


func _advance_appears(delta_ms: float) -> void:
    var index := _appears.size() - 1
    while index >= 0:
        var appear: Dictionary = _appears[index]
        appear["t"] += delta_ms
        var card: Control = appear["node"]
        if not is_instance_valid(card):
            _appears.remove_at(index)
            index -= 1
            continue
        var k := clampf(float(appear["t"]) / APPEAR_MS, 0.0, 1.0)
        card.modulate.a = k
        var scale := 0.85 + 0.15 * k
        card.scale = Vector2(scale, scale)
        if k >= 1.0:
            _reset_card(card)
            _appears.remove_at(index)
        index -= 1


func _advance_flashes(delta_ms: float) -> void:
    var index := _flashes.size() - 1
    while index >= 0:
        var flash_entry: Dictionary = _flashes[index]
        flash_entry["t"] += delta_ms
        var card: Control = flash_entry["node"]
        if not is_instance_valid(card):
            _flashes.remove_at(index)
            index -= 1
            continue
        var k := clampf(float(flash_entry["t"]) / FLASH_MS, 0.0, 1.0)
        card.modulate = UITheme.color("color-white").lerp(
            UITheme.color("color-combat-tint"), 1.0 - k
        )
        if k >= 1.0:
            _reset_card(card)
            _flashes.remove_at(index)
        index -= 1


func _advance_damage_labels(delta_ms: float) -> void:
    var index := _damage_labels.size() - 1
    while index >= 0:
        var entry: Dictionary = _damage_labels[index]
        entry["t"] += delta_ms
        var label: Control = entry["node"]
        if not is_instance_valid(label):
            _damage_labels.remove_at(index)
            index -= 1
            continue
        var k := clampf(float(entry["t"]) / DAMAGE_MS, 0.0, 1.0)
        label.position = entry["start"] + Vector2(0, -DAMAGE_RISE * k)
        # 前 40% 从 1.6 倍回落到 1 倍；前 70% 保持不透明，尾段再淡出。
        var pop := 1.0 + (DAMAGE_POP - 1.0) * maxf(0.0, 1.0 - k * 2.5)
        label.pivot_offset = label.size * 0.5
        label.scale = Vector2(pop, pop)
        label.modulate.a = clampf((1.0 - k) / 0.30, 0.0, 1.0)
        if k >= 1.0:
            _free_node(label)
            _damage_labels.remove_at(index)
        index -= 1


## 攻方回位后恢复原卡；若原卡已随阵亡移出行内则跳过。
func _restore_source(source: Control) -> void:
    if not is_instance_valid(source) or source.is_queued_for_deletion():
        return
    source.modulate.a = 1.0


## 复制一张同尺寸卡面放进覆盖层；失败返回 null。
func _clone_card(card: Control, clone_name: String) -> Control:
    var clone := _copy_card(card)
    if clone == null:
        return null
    clone.name = clone_name
    _ignore_mouse(clone)
    add_child(clone)
    clone.size = card.size
    clone.scale = Vector2.ONE
    clone.modulate = UITheme.color("color-white")
    return clone


## 复制卡面：英雄卡按状态重建，其余卡面直接复制。
##
## `GameHeroCard` 的头像框在 `_init()` 里建子节点，`duplicate()` 会先跑一遍 `_init()`
## 再把原节点子层复制进来，默认子层叠在上面会盖住头像与描边。
func _copy_card(card: Control) -> Control:
    if card is GameHeroCard:
        return (card as GameHeroCard).replica()
    return card.duplicate() as Control


func _ignore_mouse(node: Node) -> void:
    if node is Control:
        (node as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
    for child in node.get_children():
        _ignore_mouse(child)


## 全局坐标换算到覆盖层局部坐标；覆盖层随内容框缩放，不能只减 `global_position`。
func _local_point(point: Vector2) -> Vector2:
    return get_global_transform().affine_inverse() * point


func _reset_card(card: Control) -> void:
    if not is_instance_valid(card):
        return
    card.modulate = UITheme.color("color-white")
    card.scale = Vector2.ONE
    card.pivot_offset = Vector2.ZERO


func _free_node(node: Variant) -> void:
    if node is Node and is_instance_valid(node):
        (node as Node).queue_free()
