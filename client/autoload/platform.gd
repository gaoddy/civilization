extends Node
## 渠道抽象：构建渠道判定与 Steam 语义接口。
##
## 页面、`GameStore` 与 `Net` 只通过这里判断渠道和发起登录 / 邀请 / 成就同步；
## Steamworks 调用全部收在后端实现里。开发构建使用 `DevBackend`，行为与接入前一致。

signal lobby_join_requested(room_code: String, invite_token: String)
signal steam_unavailable(reason: String)

const BuildConfig = preload("res://generated/build_config.gd")
const DevBackend = preload("res://platform/dev_backend.gd")
const SteamBackend = preload("res://platform/steam_backend.gd")

const CHANNEL_DEV := "dev"
const CHANNEL_STEAM := "steam"

const SIGN_IN_TIMEOUT_MS := 12_000
const LOBBY_WAIT_TIMEOUT_MS := 12_000

## Steam 启动失败原因：`not_running` / `not_logged_in` / `unavailable`。
var startup_error := ""

var _backend = null
var _channel_override := ""
var _lobby_active := false


func _ready() -> void:
    _configure()
    GameStore.achievement_unlocked.connect(_on_achievement_unlocked)
    lobby_join_requested.connect(_on_lobby_join_requested)


func _process(_delta: float) -> void:
    if _backend == null:
        return
    _backend.poll()
    var request: Dictionary = _backend.take_join_request()
    if not request.is_empty():
        lobby_join_requested.emit(
            str(request.get("room_code", "")),
            str(request.get("invite_token", "")),
        )


## 当前构建渠道，由导出预设的 `steam` 特性决定，不靠运行时猜测。
func channel() -> String:
    if _channel_override != "":
        return _channel_override
    return CHANNEL_STEAM if OS.has_feature("steam") else CHANNEL_DEV


func is_steam() -> bool:
    return channel() == CHANNEL_STEAM


## 获取 Steam Web API 票据；返回 `{ok, ticket, personaName}` 或 `{ok: false, reason}`。
## 失败原因：`not_running` / `not_logged_in` / `unavailable` / `timeout` / `auth_failed`。
func sign_in() -> Dictionary:
    if _backend == null:
        return {"ok": false, "reason": "unavailable"}
    if startup_error != "":
        return {"ok": false, "reason": startup_error}
    _backend.request_ticket()
    var deadline := Time.get_ticks_msec() + SIGN_IN_TIMEOUT_MS
    while Time.get_ticks_msec() < deadline:
        var result: Dictionary = _backend.take_ticket_result()
        if not result.is_empty():
            return result
        await get_tree().process_frame
    return {"ok": false, "reason": "timeout"}


func persona_name() -> String:
    if _backend == null:
        return ""
    return str(_backend.persona_name())


## Steam 启动失败后重新初始化（例如用户刚打开并登录 Steam 客户端）。
func retry_startup() -> String:
    if _channel_override != "" or not is_steam():
        return ""
    _configure()
    return startup_error


## 创建 Steam 大厅并把房间码与邀请 token 写进 metadata；返回大厅是否就绪。
## 重复创建时先离开旧大厅，避免残留旧 token 的大厅继续被加入。
func create_lobby(room_code: String, invite_token: String, expires_at: String) -> bool:
    if _backend == null:
        return false
    if _lobby_active:
        leave_lobby()
    _lobby_active = false
    if not _backend.create_lobby(room_code, invite_token, expires_at):
        return false
    var deadline := Time.get_ticks_msec() + LOBBY_WAIT_TIMEOUT_MS
    while Time.get_ticks_msec() < deadline:
        var lobby: int = _backend.take_lobby_result()
        if lobby == -1:
            return false
        if lobby > 0:
            _lobby_active = true
            return true
        await get_tree().process_frame
    return false


func invite_friends() -> void:
    if _backend == null or not _lobby_active:
        return
    _backend.invite_friends()


func is_in_lobby() -> bool:
    return _lobby_active


## 当前环境能否发起 Steam 大厅邀请；Steam 未初始化或启动失败时为 false。
func lobby_available() -> bool:
    return is_steam() and startup_error == "" and _backend != null


func leave_lobby() -> void:
    _lobby_active = false
    if _backend != null:
        _backend.leave_lobby()


func unlock_achievement(api_name: String) -> void:
    if _backend == null or api_name == "":
        return
    _backend.unlock_achievement(api_name)


## 测试注入：假后端与渠道名只影响当前进程，不写入构建配置。
func set_backend_for_test(backend, channel_name: String) -> void:
    _backend = backend
    _channel_override = channel_name
    startup_error = ""
    _lobby_active = false


func clear_backend_for_test() -> void:
    _backend = null
    _channel_override = ""
    startup_error = ""
    _lobby_active = false
    _configure()


func _configure() -> void:
    if is_steam() and int(BuildConfig.STEAM_APP_ID) > 0:
        var backend = SteamBackend.new(int(BuildConfig.STEAM_APP_ID))
        _backend = backend
        startup_error = str(backend.start())
    else:
        _backend = DevBackend.new()
        startup_error = "unavailable" if is_steam() else ""
    if startup_error != "":
        # 延迟到主场景 _ready 之后，登录页才连得上这个信号。
        steam_unavailable.emit.call_deferred(startup_error)


func _on_achievement_unlocked(api_name: String) -> void:
    if is_steam():
        unlock_achievement(api_name)


## Steam 邀请统一交给 `GameStore`：未登录先缓存，已在房间只提示，其余消费 token 入房。
func _on_lobby_join_requested(_room_code: String, invite_token: String) -> void:
    GameStore.accept_steam_invite(invite_token)
