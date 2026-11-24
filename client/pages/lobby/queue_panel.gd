class_name QueuePanel
extends PopupLayer
## 匹配排队浮层：展示等待时长与已凑人数，并提供取消入口。
##
## 自己订阅 `GameStore.queue_changed`，排队中显示、其余时间隐藏；取消只发出
## `cancel_pressed`，由宿主决定发送 `LeaveQueue`。人数与等待基准都来自
## `GameStore.queue_state`，本组件不做二次判断。

signal cancel_pressed

const REFRESH_SECONDS := 0.5

var _waiting: Label
var _count: Label
var _timer: Timer


func _init() -> void:
    super(PopupLayer.Placement.CENTER, UITheme.opacity("scrim-alpha"))
    name = "QueuePanel"
    mouse_filter = Control.MOUSE_FILTER_STOP

    var panel := UIControls.card_panel("raised")
    panel.name = "QueueCard"
    panel.custom_minimum_size = Vector2(320, 0)
    set_content(panel)

    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", UITheme.space("space-md"))
    panel.add_child(box)
    box.add_child(UIControls.section_title(tr("ui.queue.title")))

    _waiting = UIControls.label("", UITheme.font_size("font-title"), false)
    _waiting.name = "QueueWaiting"
    box.add_child(_waiting)

    _count = UIControls.pill_label("", "accent")
    _count.name = "QueueCount"
    box.add_child(_count)

    var cancel := UIControls.button(tr("ui.queue.cancel"))
    cancel.name = "CancelQueueButton"
    cancel.pressed.connect(_on_cancel)
    box.add_child(cancel)


func _ready() -> void:
    if not GameStore.queue_changed.is_connected(_refresh):
        GameStore.queue_changed.connect(_refresh)
    _timer = Timer.new()
    _timer.name = "QueueRefresh"
    _timer.wait_time = REFRESH_SECONDS
    _timer.timeout.connect(_refresh)
    add_child(_timer)
    _refresh()


func _exit_tree() -> void:
    # 浮层随大厅释放；不主动断开会让全局信号持有已释放的节点。
    if GameStore.queue_changed.is_connected(_refresh):
        GameStore.queue_changed.disconnect(_refresh)


func _refresh() -> void:
    if _waiting == null:
        return
    var active: bool = GameStore.queueing
    visible = active
    if not active:
        if _timer != null:
            _timer.stop()
        return
    if _timer != null and _timer.is_stopped():
        _timer.start()
    _waiting.text = tr("ui.queue.waiting") % _clock(GameStore.queue_wait_seconds())
    var state: Dictionary = GameStore.queue_state
    _count.text = tr("ui.queue.players") % int(state.get("waitingPlayers", 0))


## 等待时长按分:秒展示；超过一小时仍用分累计，避免再引入小时单位。
static func _clock(seconds: float) -> String:
    var total := maxi(0, int(seconds))
    var minutes := floori(total / 60.0)
    return "%02d:%02d" % [minutes, total % 60]


func _on_cancel() -> void:
    cancel_pressed.emit()
