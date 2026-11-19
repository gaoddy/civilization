class_name GameUiState
extends RefCounted
## 一次刷新期间的页面快照：只承载视图数据与门控结果，不持有任何方法或节点。
##
## `game.gd` 在 `_refresh` 里构造一次并传给各视图；视图只读，不回写。

## 服务端下发的原始视图。
var view: Dictionary = {}

## 视图里的本人状态（`view.self`）。
var self_state: Dictionary = {}

## 当前阶段；缺省时沿用 `GameStore.phase`。
var phase := ""

## 本人是否已结束招募。
var recruited := false

## 视图回合与当前回合不一致：旧视图不能用来发起操作。
var stale := false

## 当前阶段倒计时已归零。
var expired := false

## 队首待选是目标待选：除选目标外不能操作。
var target_pending := false

## 网络连接是否可用。
var connected := false

## 视图版本号；拖拽数据带上它，落点回调据此判断拖拽是否已过期。
var epoch := 0

## 当前是否允许落点（`drop_gate` 的实时结果）。
var can_play := false

## 棋盘容量，取视图 `boardSlots`，缺失时回退 7。
var board_slots := 7

## 生命上限，取视图 `hpCap`。
var hp_cap := 40


## 手牌起始门控：与 `game.gd` 的 `drag_gate` 条件一致。
func can_drag(playable: bool) -> bool:
    return (
        phase == Protocol.PHASE_RECRUIT
        and not stale
        and not expired
        and connected
        and not target_pending
        and playable
    )
