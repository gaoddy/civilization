class_name CombatPlayback
extends RefCounted
## 服务端战斗动作序列的落地器：只把下发值写进双方随从列表，不做数值推算。
##
## 动作 `i` 在 `(i + 1) * PACE_MS` 时应用；全部动作播完并经过 `TAIL_MS` 后
## `banner_visible()` 为真。重连时用 `seek()` 直接跳到服务端 `phaseDeadline`
## 反推出的进度，已播动作不会重播。
##
## 列表保留已阵亡随从（引擎同样保留），这样 `summon.position` 与引擎下标一致。

## 单条动作的播放时长（与服务端 room-service 的 PACE_MS 一致）。
const PACE_MS := 320

## 结果横幅停留（与服务端 room-service 的 TAIL_MS 一致）。
const TAIL_MS := 2000

## 我方随从（含已阵亡，保持与引擎一致的顺序）。
var own: Array = []

## 对手随从（含已阵亡）。
var opponent: Array = []

## 本场动作序列。
var actions: Array = []

var _elapsed_ms := 0.0
var _applied := 0
var _banner := false


## 装载一场战斗：动作序列与双方战前阵容（深拷贝，外部改动不影响本对象）。
func start(actions_in: Array, own_board: Array, opponent_board: Array) -> void:
    actions = actions_in.duplicate(true)
    own = own_board.duplicate(true)
    opponent = opponent_board.duplicate(true)
    _elapsed_ms = 0.0
    _applied = 0
    _banner = false


## 演示总时长：动作数 × 节奏 + 横幅停留。
func total_ms() -> int:
    return actions.size() * PACE_MS + TAIL_MS


## 推进 `delta_ms` 并按节奏应用到期动作。
func advance(delta_ms: float) -> void:
    if is_finished():
        return
    _elapsed_ms += delta_ms
    _apply_due()
    if _elapsed_ms >= float(total_ms()):
        _banner = true


## 重连续播：把进度直接放到 `elapsed_ms`，只应用已到期的动作。
func seek(elapsed_ms: float) -> void:
    _elapsed_ms = maxf(elapsed_ms, 0.0)
    _banner = false
    _apply_due()
    if _elapsed_ms >= float(total_ms()):
        _banner = true


## 动作播完且经过横幅停留。
func banner_visible() -> bool:
    return _banner


## 演示结束（与横幅出现同一时刻）。
func is_finished() -> bool:
    return _banner


func _apply_due() -> void:
    while _applied < actions.size() and float((_applied + 1) * PACE_MS) <= _elapsed_ms:
        _apply(actions[_applied])
        _applied += 1


func _apply(action: Dictionary) -> void:
    match str(action.get("kind", "")):
        "attack":
            _apply_state(action.get("source"), str(action.get("sourceSide", "")))
            _apply_state(action.get("target"), str(action.get("targetSide", "")))
        "death", "reborn":
            # 这两类动作的定位字段写在 `sourceSide`（受影响的随从），状态在 `target`。
            _apply_state(action.get("target"), str(action.get("sourceSide", "")))
        "contagion":
            _apply_state(action.get("target"), str(action.get("targetSide", "")))
        "summon":
            _apply_summon(action.get("summon"))
        _:
            pass


## 把服务端快照写进对应随从：只写 `attack`/`health`/`alive`，不做任何推算。
##
## 侧别优先取动作携带的 `sourceSide`/`targetSide`（镜像战双方实例编号可能相同，
## 只按 id 会写错行）；缺失时退回按 id 在双方列表里查找。
func _apply_state(state: Variant, side: String = "") -> void:
    if not (state is Dictionary):
        return
    var snapshot: Dictionary = state
    var minion_id := str(snapshot.get("minionId", ""))
    var minion := _find_on(side, minion_id) if side != "" else _find(minion_id)
    if minion.is_empty():
        return
    minion["attack"] = int(snapshot.get("attack", minion.get("attack", 0)))
    minion["health"] = int(snapshot.get("health", minion.get("health", 0)))
    minion["alive"] = bool(snapshot.get("alive", minion.get("alive", true)))


## 在指定侧按实例 id 找随从；侧别未知时退回双侧查找。
func _find_on(side: String, minion_id: String) -> Dictionary:
    if minion_id == "":
        return {}
    var board: Array = own if side == "own" else opponent
    for minion in board:
        if str(minion.get("minionId", "")) == minion_id:
            return minion
    return {}


## 召唤：按服务端给出的侧别与下标插入完整随从（下标与引擎一致）。
func _apply_summon(summon: Variant) -> void:
    if not (summon is Dictionary):
        return
    var data: Dictionary = summon
    var minion_value: Variant = data.get("minion")
    if not (minion_value is Dictionary):
        return
    var minion: Dictionary = (minion_value as Dictionary).duplicate(true)
    minion["alive"] = true
    var target := own if str(data.get("side", "")) == "own" else opponent
    var position := clampi(int(data.get("position", target.size())), 0, target.size())
    target.insert(position, minion)


## 在双方列表里按实例 id 找随从字典；找不到返回空字典。
func _find(minion_id: String) -> Dictionary:
    if minion_id == "":
        return {}
    for minion in own:
        if str(minion.get("minionId", "")) == minion_id:
            return minion
    for minion in opponent:
        if str(minion.get("minionId", "")) == minion_id:
            return minion
    return {}
