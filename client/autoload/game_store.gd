extends Node
## 全局状态：保存认证、房间与对局视图，只做服务端消息的字段搬运。

signal auth_changed
signal room_changed
signal view_changed
signal game_started
signal game_over
signal error_received
signal social_changed
signal invite_received
signal invite_result_received

var player_id := ""
var player_code := ""
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
var last_combat: Dictionary = {}
## `last_combat` 所属回合；与当前 `turn` 不一致时不展示，避免旧战报被当成新战斗。
var last_combat_turn := -1
var timer_seconds := -1

## 好友视图：服务端下发的四段列表原样保存，展示排序由服务端负责。
var friends: Array = []
var incoming_requests: Array = []
var outgoing_requests: Array = []
var recent_players: Array = []
## 待处理的好友邀请；收到 `InviteAccepted` 或回执后按 `inviteId` 移除。
var pending_invites: Array = []
var last_invite_result: Dictionary = {}


func handle(type: String, payload: Dictionary) -> void:
    match type:
        Protocol.MSG_AUTH_RESULT:
            player_id = str(payload.get("playerId", ""))
            player_code = str(payload.get("playerCode", ""))
            token = str(payload.get("token", ""))
            auth_changed.emit()
        Protocol.MSG_FRIEND_UPDATE:
            friends = payload.get("friends", [])
            incoming_requests = payload.get("incoming", [])
            outgoing_requests = payload.get("outgoing", [])
            recent_players = payload.get("recent", [])
            social_changed.emit()
        Protocol.MSG_FRIEND_INVITE:
            _upsert_invite(payload)
            social_changed.emit()
            invite_received.emit()
        Protocol.MSG_FRIEND_INVITE_RESULT:
            last_invite_result = payload
            _drop_invite(str(payload.get("inviteId", "")))
            social_changed.emit()
            invite_result_received.emit()
        Protocol.MSG_INVITE_ACCEPTED:
            # 接受邀请只拿到房间码；入房仍走普通 `JoinRoom`，避免跨房间任务挂载。
            _drop_invite(str(payload.get("inviteId", "")))
            social_changed.emit()
            reset_room()
            Net.send(Protocol.MSG_JOIN_ROOM, {"code": str(payload.get("roomCode", ""))})
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
            last_combat_turn = turn
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
    var previous_phase := phase
    view = value
    phase = str(view.get("phase", phase))
    turn = int(view.get("turn", turn))
    deadline = str(view.get("phaseDeadline", deadline))
    # 战报只属于刚结束的这一场：离开战斗（回到招募）即丢弃，否则没有本回合
    # `CombatResult` 的座位（未配对、已出局、消息丢失）会把旧战斗当新战斗重播。
    if previous_phase == Protocol.PHASE_COMBAT and phase != Protocol.PHASE_COMBAT:
        last_combat = {}
        last_combat_turn = -1


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
    last_combat = {}
    last_combat_turn = -1
    timer_seconds = -1


## 本人的英雄候选；服务端未下发时返回空数组。
func my_hero_candidates() -> Array:
    for player in room_players:
        if str(player.get("playerId", "")) == player_id:
            var list = player.get("heroCandidates")
            return list if list is Array else []
    return []


## 本人座位是否已确认（准备）。
func my_ready() -> bool:
    for player in room_players:
        if str(player.get("playerId", "")) == player_id:
            return bool(player.get("ready", false))
    return false


## 本人已选的英雄卡 id；未选返回空串。
func my_hero_id() -> String:
    for player in room_players:
        if str(player.get("playerId", "")) == player_id:
            var hero = player.get("heroCardId")
            return "" if hero == null else str(hero)
    return ""


## 战报是否属于当前回合。
func combat_is_current() -> bool:
    return not last_combat.is_empty() and last_combat_turn == turn


func reset_social() -> void:
    friends = []
    incoming_requests = []
    outgoing_requests = []
    recent_players = []
    pending_invites = []
    last_invite_result = {}


func reset_session() -> void:
    reset_room()
    reset_social()
    player_id = ""
    player_code = ""
    token = ""
    display_name = ""


## 尚未过期的待处理邀请；服务端仍是最终校验方，这里只用于避免展示已过期的条目。
func live_invites() -> Array:
    var now := Time.get_unix_time_from_system()
    var result: Array = []
    for invite in pending_invites:
        var expires := UI.unix_from_rfc3339(str(invite.get("expiresAt", "")))
        if expires <= 0.0 or expires > now:
            result.append(invite)
    return result


func _upsert_invite(payload: Dictionary) -> void:
    var invite_id := str(payload.get("inviteId", ""))
    if invite_id == "":
        return
    _drop_invite(invite_id)
    pending_invites.push_front(payload)


func _drop_invite(invite_id: String) -> void:
    if invite_id == "":
        return
    for index in range(pending_invites.size() - 1, -1, -1):
        if str(pending_invites[index].get("inviteId", "")) == invite_id:
            pending_invites.remove_at(index)


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
