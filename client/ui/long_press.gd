class_name LongPressTracker
extends RefCounted
## 长按判定：只跟踪第一个触点，按帧累计时长，位移超过取消距离即标记取消。
##
## 详情层与选英雄页共用同一套阈值与计时方式，避免两套时钟在 450ms 附近判定不一致。
## 宿主收到触控事件后转给本类，并在 `_process` 中调用 `advance(delta)`。

const DEFAULT_LONG_PRESS_MS := 450.0
const DEFAULT_CANCEL_DISTANCE := 8.0

## 长按判定时长（毫秒）与位移取消阈值（画布单位）。
var long_press_ms := DEFAULT_LONG_PRESS_MS
var cancel_distance := DEFAULT_CANCEL_DISTANCE

## 当前触点编号；-1 表示没有进行中的触点。
var index := -1
var start := Vector2.ZERO
var elapsed_ms := 0.0
## 按下时命中的控件；取消后清空。
var target: Control = null
## 已达到长按阈值；置位后不再累计。
var fired := false
## 位移已越过取消距离；是否立即丢弃触点由宿主决定。
var cancelled := false


func _init(press_ms := DEFAULT_LONG_PRESS_MS, cancel_px := DEFAULT_CANCEL_DISTANCE) -> void:
    long_press_ms = press_ms
    cancel_distance = cancel_px


func active() -> bool:
    return index != -1


func matches(touch_index: int) -> bool:
    return index == touch_index


## 开始跟踪；已有触点时忽略并返回 false。
func begin(touch_index: int, point: Vector2, node: Control) -> bool:
    if index != -1:
        return false
    index = touch_index
    start = point
    elapsed_ms = 0.0
    target = node
    fired = false
    cancelled = false
    return true


## 位移判定；返回 true 表示本次刚越过取消距离。
func dragged(touch_index: int, point: Vector2) -> bool:
    if index != touch_index or fired or cancelled:
        return false
    if point.distance_to(start) <= cancel_distance:
        return false
    cancelled = true
    return true


## 按帧累计；返回 true 表示本次刚达到长按阈值。
func advance(delta: float) -> bool:
    if index == -1 or fired or cancelled:
        return false
    elapsed_ms += delta * 1000.0
    if elapsed_ms < long_press_ms:
        return false
    fired = true
    return true


## 丢弃当前触点并复位。
func cancel() -> void:
    index = -1
    start = Vector2.ZERO
    elapsed_ms = 0.0
    target = null
    fired = false
    cancelled = false
