class_name LobbyQueueCard
extends PanelContainer
## 右栏中部排队卡：等待时长、已凑人数与积分窗口；取消入口在快捷操作里。
##
## 自己订阅 `GameStore.queue_changed`，每 0.5 秒按本地时钟刷新时长；
## 匹配成功后停表并显示成功文案，由大厅负责随后切页。

const REFRESH_SECONDS := 0.5

var _timer: Timer
var _clock: Label
var _count: Label
var _window: Label
var _hint: Label
var _matched := false


func _init() -> void:
    name = "LobbyQueueCard"
    add_theme_stylebox_override("panel", UITheme.panel_style("accent"))

    var box := VBoxContainer.new()
    box.alignment = BoxContainer.ALIGNMENT_CENTER
    box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(box)

    _clock = UIControls.label("00:00", UITheme.font_size("font-display-lg"), false)
    _clock.name = "QueueTimer"
    _clock.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _clock.add_theme_color_override("font_color", UITheme.color("color-accent"))
    box.add_child(_clock)

    var count_row := CenterContainer.new()
    box.add_child(count_row)
    _count = UIControls.pill_label("", "accent")
    _count.name = "QueueCount"
    count_row.add_child(_count)

    _window = UIControls.label("", UITheme.font_size("font-body-sm"), false)
    _window.name = "QueueWindow"
    _window.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _window.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    box.add_child(_window)

    _hint = UIControls.label(tr("ui.lobby.queue_hint"), UITheme.font_size("font-body-sm"))
    _hint.name = "QueueHint"
    _hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _hint.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    box.add_child(_hint)

    _timer = Timer.new()
    _timer.name = "QueueRefresh"
    _timer.wait_time = REFRESH_SECONDS
    _timer.timeout.connect(_refresh)
    add_child(_timer)


func _ready() -> void:
    if not GameStore.queue_changed.is_connected(_refresh):
        GameStore.queue_changed.connect(_refresh)
    _refresh()


func _exit_tree() -> void:
    if GameStore.queue_changed.is_connected(_refresh):
        GameStore.queue_changed.disconnect(_refresh)


## 匹配成功：停表并显示结果，等待大厅切页。
func set_matched() -> void:
    _matched = true
    _timer.stop()
    _clock.text = tr("ui.lobby.match_found")


func _refresh() -> void:
    if _clock == null:
        return
    if not GameStore.queueing:
        _matched = false
        _timer.stop()
        return
    _matched = false
    if _timer.is_stopped():
        _timer.start()
    _clock.text = _clock_text(GameStore.queue_wait_seconds())
    _count.text = tr("ui.queue.players") % int(GameStore.queue_state.get("waitingPlayers", 0))
    var window = GameStore.queue_state.get("ratingWindow", null)
    _window.text = (
        tr("ui.queue.window_unlimited")
        if window == null
        else tr("ui.queue.window") % int(window)
    )


## 等待时长按分:秒展示；超过一小时仍用分累计。
static func _clock_text(seconds: float) -> String:
    var total := maxi(0, int(seconds))
    return "%02d:%02d" % [floori(total / 60.0), total % 60]
