class_name CombatHeroClash
extends CombatFx
## 战斗结束演出：双方英雄卡冲入战斗河道中央对冲，胜方顶退败方，败方闪白并飘出伤害。
##
## 本层挂在页面级覆盖层位置上，克隆只持有抓取时的矩形与克隆节点；原卡随后被重建、
## 隐藏或淘汰都不影响演出，恢复原卡时统一做有效性判断。
##
## 时间轴由页面每帧调用 [`advance`] 推进，不用 `Tween`，无头冒烟测试可按帧断言：
## 冲入 → 顶退（命中反馈）→ 停顿 → 回位。平局双方互顶后弹回，不飘伤害。

## 双方从原位冲到河道中央的时长（毫秒）。
const DASH_MS := 260.0
## 胜方顶、败方退的时长（毫秒）。
const PUSH_MS := 140.0
## 顶退后停顿的时长（毫秒）。
const HOLD_MS := 80.0
## 回到原位的时长（毫秒）。
const RETURN_MS := 300.0

## 相遇间隙占英雄卡高度的比例。
const GAP_RATIO := 0.08
## 胜方顶进占英雄卡高度的比例。
const WIN_PUSH_RATIO := 0.18
## 败方击退占英雄卡高度的比例。
const LOSE_KNOCK_RATIO := 0.75
## 平局双方互顶占英雄卡高度的比例。
const DRAW_PUSH_RATIO := 0.10

## 进行中的对冲：克隆、原卡、起止位置、胜方与伤害；空字典表示空闲。
var _clash: Dictionary = {}


func _init() -> void:
    super()
    name = "CombatHeroClash"
    mouse_filter = Control.MOUSE_FILTER_IGNORE


## 开始一场对冲；`winner_side` 取 `own` / `opponent` / 空串（平局）。
func start(
    own_card: Control,
    opponent_card: Control,
    river_global: Rect2,
    winner_side: String,
    damage: int,
) -> void:
    if own_card == null or opponent_card == null:
        return
    clear()
    var own_clone := _clone_card(own_card, "HeroClashOwn")
    var opp_clone := _clone_card(opponent_card, "HeroClashOpponent")
    if own_clone == null or opp_clone == null:
        _free_node(own_clone)
        _free_node(opp_clone)
        return
    var card_h := own_clone.size.y
    var center := _local_point(river_global.get_center())
    var gap := card_h * GAP_RATIO
    var own_from := _local_point(own_card.global_position)
    var opp_from := _local_point(opponent_card.global_position)
    # 对手在上、我方在下，双方卡面在河道中心上下贴合，见面后各让半段间隙。
    var own_meet := Vector2(center.x - own_clone.size.x * 0.5, center.y + gap * 0.5)
    var opp_meet := Vector2(
        center.x - opp_clone.size.x * 0.5, center.y - gap * 0.5 - opp_clone.size.y
    )
    var own_target := own_meet
    var opp_target := opp_meet
    match winner_side:
        "own":
            own_target = own_meet + Vector2(0, -card_h * WIN_PUSH_RATIO)
            opp_target = opp_meet + Vector2(0, -card_h * LOSE_KNOCK_RATIO)
        "opponent":
            own_target = own_meet + Vector2(0, card_h * LOSE_KNOCK_RATIO)
            opp_target = opp_meet + Vector2(0, card_h * WIN_PUSH_RATIO)
        _:
            own_target = own_meet + Vector2(0, -card_h * DRAW_PUSH_RATIO)
            opp_target = opp_meet + Vector2(0, card_h * DRAW_PUSH_RATIO)
    own_clone.position = own_from
    opp_clone.position = opp_from
    own_card.modulate.a = 0.0
    opponent_card.modulate.a = 0.0
    _clash = {
        "own_clone": own_clone,
        "opp_clone": opp_clone,
        "own_source": own_card,
        "opp_source": opponent_card,
        "own_from": own_from,
        "opp_from": opp_from,
        "own_meet": own_meet,
        "opp_meet": opp_meet,
        "own_target": own_target,
        "opp_target": opp_target,
        "winner": winner_side,
        "damage": maxi(damage, 0),
        "t": 0.0,
        "struck": false,
    }


## 先推进飘字与闪光，再推进对冲本身。
func advance(delta: float) -> void:
    super.advance(delta)
    if _clash.is_empty():
        return
    var own_clone: Control = _clash["own_clone"]
    var opp_clone: Control = _clash["opp_clone"]
    if not is_instance_valid(own_clone) or not is_instance_valid(opp_clone):
        _finish_clash()
        return
    _clash["t"] = float(_clash["t"]) + delta * 1000.0
    var t := float(_clash["t"])
    if t <= DASH_MS:
        var k := t / DASH_MS
        own_clone.position = _clash["own_from"].lerp(_clash["own_meet"], k)
        opp_clone.position = _clash["opp_from"].lerp(_clash["opp_meet"], k)
        return
    if not bool(_clash["struck"]):
        _clash["struck"] = true
        _clash_hit()
    if t <= DASH_MS + PUSH_MS:
        var k := (t - DASH_MS) / PUSH_MS
        own_clone.position = _clash["own_meet"].lerp(_clash["own_target"], k)
        opp_clone.position = _clash["opp_meet"].lerp(_clash["opp_target"], k)
        return
    if t <= DASH_MS + PUSH_MS + HOLD_MS:
        own_clone.position = _clash["own_target"]
        opp_clone.position = _clash["opp_target"]
        return
    if t <= DASH_MS + PUSH_MS + HOLD_MS + RETURN_MS:
        var k := (t - DASH_MS - PUSH_MS - HOLD_MS) / RETURN_MS
        own_clone.position = _clash["own_target"].lerp(_clash["own_from"], k)
        opp_clone.position = _clash["opp_target"].lerp(_clash["opp_from"], k)
        return
    _finish_clash()


## 演出是否仍在进行。
func playing() -> bool:
    return not _clash.is_empty()


func clear() -> void:
    _finish_clash()
    super.clear()


## 命中反馈：败方闪白并飘出伤害；平局双方各闪一次，不飘伤害。
func _clash_hit() -> void:
    var winner := str(_clash["winner"])
    var own_clone: Control = _clash["own_clone"]
    var opp_clone: Control = _clash["opp_clone"]
    if winner == "own":
        flash(opp_clone)
        _damage_at(opp_clone)
    elif winner == "opponent":
        flash(own_clone)
        _damage_at(own_clone)
    else:
        flash(own_clone)
        flash(opp_clone)


func _damage_at(card: Control) -> void:
    var damage := int(_clash["damage"])
    if damage <= 0:
        return
    _spawn_damage_label(card.position + card.size * 0.5, damage, "HeroClashDamage")


func _finish_clash() -> void:
    if _clash.is_empty():
        return
    _free_node(_clash["own_clone"])
    _free_node(_clash["opp_clone"])
    _restore_source(_clash["own_source"])
    _restore_source(_clash["opp_source"])
    _clash.clear()
