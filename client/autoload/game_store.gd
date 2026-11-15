extends Node
## 全局状态：保存认证、房间与对局视图，只做服务端消息的字段搬运。

signal auth_changed
signal room_changed
signal view_changed
signal game_started
signal game_over
signal error_received
signal combat_received
signal connection_changed

var player_id := ""
var token := ""
var display_name := ""

var room_id := ""
var room_code := ""
var room_players: Array = []

var phase := ""
var turn := 1
var deadline := ""
var view: Dictionary = {}
var last_seq := 0

var ranking: Array = []
var game_seed := ""
var last_error: Dictionary = {}
var event_log: Array = []
var last_combat: Dictionary = {}
var timer_seconds := -1


func handle(type: String, payload: Dictionary) -> void:
    match type:
        Protocol.MSG_AUTH_RESULT:
            player_id = str(payload.get("playerId", ""))
            token = str(payload.get("token", ""))
            auth_changed.emit()
        Protocol.MSG_ROOM_UPDATE:
            room_id = str(payload.get("roomId", ""))
            room_code = str(payload.get("code", ""))
            room_players = payload.get("players", [])
            phase = str(payload.get("phase", Protocol.PHASE_LOBBY))
            room_changed.emit()
        Protocol.MSG_GAME_STARTED:
            _apply_view(payload.get("view", {}))
            game_started.emit()
            view_changed.emit()
        Protocol.MSG_STATE_UPDATE, Protocol.MSG_SNAPSHOT:
            _apply_view(payload.get("view", {}))
            for event in payload.get("events", []):
                if event is Dictionary:
                    event_log.append(event)
            while event_log.size() > 30:
                event_log.pop_front()
            view_changed.emit()
        Protocol.MSG_PHASE_CHANGED:
            phase = str(payload.get("phase", phase))
            turn = int(payload.get("turn", turn))
            deadline = str(payload.get("deadline", deadline))
            view_changed.emit()
        Protocol.MSG_TIMER_WARNING:
            timer_seconds = int(payload.get("secondsLeft", -1))
            view_changed.emit()
        Protocol.MSG_COMBAT_RESULT:
            last_combat = payload
            combat_received.emit()
            view_changed.emit()
        Protocol.MSG_GAME_OVER:
            ranking = payload.get("ranking", [])
            game_seed = str(payload.get("seed", ""))
            phase = Protocol.PHASE_GAME_OVER
            game_over.emit()
        Protocol.MSG_ERROR:
            last_error = payload
            error_received.emit()
        _:
            pass


func _apply_view(value) -> void:
    if not (value is Dictionary) or value.is_empty():
        return
    view = value
    phase = str(view.get("phase", phase))
    turn = int(view.get("turn", turn))
    deadline = str(view.get("phaseDeadline", deadline))


func reset_room() -> void:
    room_id = ""
    room_code = ""
    room_players = []
    phase = ""
    turn = 1
    deadline = ""
    view = {}
    last_seq = 0
    ranking = []
    game_seed = ""
    last_error = {}
    event_log = []
    last_combat = {}
    timer_seconds = -1


func player_name(pid: String) -> String:
    if pid == "":
        return ""
    for player in view.get("players", []):
        if str(player.get("playerId", "")) == pid:
            return str(player.get("displayName", pid))
    for player in room_players:
        if str(player.get("playerId", "")) == pid:
            return str(player.get("displayName", pid))
    return pid


func my_summary() -> Dictionary:
    for player in view.get("players", []):
        if str(player.get("playerId", "")) == player_id:
            return player
    return {}
