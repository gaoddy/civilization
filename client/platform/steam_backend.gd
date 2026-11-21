extends RefCounted
## Steam 渠道后端：唯一调用 GodotSteam 单例的位置。
##
## Steam 单例按名字动态取用，不在脚本里静态引用：开发构建与无头冒烟即使没有加载
## GDExtension，本文件也能通过编译，`Platform` 走开发后端。

const SERVICE_IDENTITY := "civilization"
const TICKET_TIMEOUT_MS := 10_000
const LOBBY_TIMEOUT_MS := 10_000
const LOBBY_MAX_MEMBERS := 8
const RESULT_OK := 1
const LOBBY_JOINED_OK := 1
const LOBBY_KEY_ROOM_CODE := "room_code"
const LOBBY_KEY_INVITE_TOKEN := "invite_token"
const LOBBY_KEY_INVITE_EXPIRES := "invite_expires_at"
const LOBBY_KEY_INVITER_NAME := "inviter_name"

var _steam: Object = null
var _app_id := 0
var _startup_reason := ""

var _ticket_result: Dictionary = {}
var _ticket_pending := false
var _ticket_deadline := 0

var _lobby_result := -2
var _lobby_pending := false
var _lobby_deadline := 0
var _lobby_metadata: Dictionary = {}
var _lobby_id := 0

var _join_request: Dictionary = {}


func _init(app_id: int) -> void:
    _app_id = app_id


func start() -> String:
    if not Engine.has_singleton("Steam"):
        _startup_reason = "unavailable"
        return _startup_reason
    _steam = Engine.get_singleton("Steam")
    if not bool(_steam.isSteamRunning()):
        _startup_reason = "not_running"
        return _startup_reason
    var init: Dictionary = _steam.steamInitEx(_app_id)
    if int(init.get("status", -1)) != 0:
        _startup_reason = "not_running" if not bool(_steam.isSteamRunning()) else "not_logged_in"
        return _startup_reason
    if int(_steam.getSteamID()) == 0:
        _startup_reason = "not_logged_in"
        return _startup_reason
    _connect_signals()
    _consume_connect_lobby()
    return ""


func poll() -> void:
    if _steam == null:
        return
    _steam.run_callbacks()
    var now := Time.get_ticks_msec()
    if _ticket_pending and now >= _ticket_deadline:
        _ticket_pending = false
        _ticket_result = {"ok": false, "reason": "timeout"}
    if _lobby_pending and now >= _lobby_deadline:
        _lobby_pending = false
        _lobby_result = -1


func request_ticket() -> void:
    _ticket_result = {}
    if _steam == null:
        _ticket_result = {"ok": false, "reason": _startup_reason}
        return
    _ticket_pending = true
    _ticket_deadline = Time.get_ticks_msec() + TICKET_TIMEOUT_MS
    var handle := int(_steam.getAuthTicketForWebApi(SERVICE_IDENTITY))
    if handle == 0:
        _ticket_pending = false
        _ticket_result = {"ok": false, "reason": "auth_failed"}


func take_ticket_result() -> Dictionary:
    var result := _ticket_result
    _ticket_result = {}
    return result


func persona_name() -> String:
    if _steam == null:
        return ""
    return str(_steam.getPersonaName()).strip_edges()


func create_lobby(room_code: String, invite_token: String, expires_at: String) -> bool:
    if _steam == null or _lobby_pending:
        _lobby_result = -1
        return false
    _lobby_metadata = {
        LOBBY_KEY_ROOM_CODE: room_code,
        LOBBY_KEY_INVITE_TOKEN: invite_token,
        LOBBY_KEY_INVITE_EXPIRES: expires_at,
        LOBBY_KEY_INVITER_NAME: persona_name(),
    }
    _lobby_result = -2
    _lobby_pending = true
    _lobby_deadline = Time.get_ticks_msec() + LOBBY_TIMEOUT_MS
    _steam.createLobby(_steam.LOBBY_TYPE_PRIVATE, LOBBY_MAX_MEMBERS)
    return true


func take_lobby_result() -> int:
    var result := _lobby_result
    _lobby_result = -2
    return result


func invite_friends() -> void:
    if _steam != null and _lobby_id > 0:
        _steam.activateGameOverlayInviteDialog(_lobby_id)


func take_join_request() -> Dictionary:
    var request := _join_request
    _join_request = {}
    return request


func leave_lobby() -> void:
    if _steam != null and _lobby_id > 0:
        _steam.leaveLobby(_lobby_id)
    _lobby_id = 0


func unlock_achievement(api_name: String) -> void:
    if _steam == null or api_name == "":
        return
    var status: Dictionary = _steam.getAchievement(api_name)
    if not bool(status.get("ret", false)):
        return
    if bool(status.get("achieved", false)):
        return
    if bool(_steam.setAchievement(api_name)):
        _steam.storeStats()


func _connect_signals() -> void:
    for item in [
        ["get_ticket_for_web_api", _on_ticket],
        ["lobby_created", _on_lobby_created],
        ["lobby_joined", _on_lobby_joined],
        ["join_requested", _on_join_requested],
    ]:
        if not _steam.is_connected(item[0], item[1]):
            _steam.connect(item[0], item[1])


func _on_ticket(result: Dictionary) -> void:
    if not _ticket_pending:
        return
    _ticket_pending = false
    var ticket := ""
    var buffer = result.get("ticket_buffer")
    if buffer is PackedByteArray:
        ticket = buffer.hex_encode()
    if int(result.get("result", 0)) != RESULT_OK or ticket == "":
        _ticket_result = {"ok": false, "reason": "auth_failed"}
        return
    _ticket_result = {"ok": true, "ticket": ticket, "personaName": persona_name()}


func _on_lobby_created(result: Dictionary) -> void:
    if not _lobby_pending:
        return
    _lobby_pending = false
    var lobby := int(result.get("lobby", 0))
    if int(result.get("connect", 0)) != RESULT_OK or lobby == 0:
        _lobby_result = -1
        return
    _lobby_id = lobby
    for key in _lobby_metadata:
        _steam.setLobbyData(lobby, key, str(_lobby_metadata[key]))
    _lobby_result = lobby


## 好友从 Steam 好友列表点「加入游戏」。
func _on_join_requested(result: Dictionary) -> void:
    var lobby := int(result.get("lobby", 0))
    if lobby > 0:
        _steam.joinLobby(lobby)


## 进入大厅后读取房主写入的 metadata，交给 `Platform` 转成入房请求。
func _on_lobby_joined(result: Dictionary) -> void:
    if int(result.get("response", 0)) != LOBBY_JOINED_OK:
        return
    var lobby := int(result.get("lobby_id", 0))
    if lobby == 0:
        return
    var room_code := str(_steam.getLobbyData(lobby, LOBBY_KEY_ROOM_CODE))
    var token := str(_steam.getLobbyData(lobby, LOBBY_KEY_INVITE_TOKEN))
    if room_code == "" or token == "":
        return
    _join_request = {"room_code": room_code, "invite_token": token}


## Steam 从外部启动游戏时带 `+connect_lobby <id>`；主动加入后由
## `lobby_joined` 统一读取 metadata。
func _consume_connect_lobby() -> void:
    var args: PackedStringArray = OS.get_cmdline_args() + OS.get_cmdline_user_args()
    for index in range(args.size() - 1):
        var flag := str(args[index])
        if flag != "+connect_lobby" and flag != "--connect-lobby":
            continue
        var lobby := int(str(args[index + 1]))
        if lobby > 0:
            _steam.joinLobby(lobby)
        return
