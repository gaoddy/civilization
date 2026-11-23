extends Node
## 全局状态：保存认证、房间与对局视图，只做服务端消息的字段搬运。

signal auth_changed
signal room_changed
signal view_changed
## 一封 `StateUpdate` 携带的领域事件；只服务动画与音效，不作为状态来源。
signal events_received(events: Array)
signal game_started
signal game_over
## 房间被解散或本人被移出；页面据此提示原因并回大厅。
signal room_closed
signal error_received
signal social_changed
signal invite_received
signal invite_result_received
## 账号视图（`AccountView`）更新；账号页与大厅底栏据此展示绑定状态。
signal account_changed
## 绑定验证码已发出；账号页开始 60 秒倒计时。
signal phone_code_sent
## 房间内邀请 token 已创建；房间页据此打开 Steam 好友邀请。
signal room_invite_received
## 成就新解锁；参数为 Steam 成就 API 名，由 `Platform` 同步到 Steam。
signal achievement_unlocked(api_name: String)
## Steam 邀请不能处理：`already_in_room`；页面据此提示，不改房间状态。
signal steam_invite_rejected(reason: String)
## 匹配队列状态变化：入队、人数刷新或取消；`queue_state` 只在排队时有意义。
signal queue_changed
## 自动匹配成局；参数为分配的座位号，房间 id 已写入 `room_id`。
signal match_found(seat: int)
## 全服排行榜已更新；排行榜页据此重排。
signal leaderboard_changed
## 个人主页已更新；个人主页据此重排。
signal profile_changed

var player_id := ""
var player_code := ""
var token := ""
var display_name := ""
## 服务端下发的本人账号视图；未登录时为空字典。
var account: Dictionary = {}
## 绑定验证码过期时间；只用于账号页倒计时。
var phone_code_expires_at := ""
## 本房间的 Steam 邀请 token 视图；离开房间即清空。
var room_invite: Dictionary = {}
## 未登录时收到的 Steam 邀请 token；认证成功后补发。
var pending_steam_invite := ""
## 已解锁成就：`achievementId -> steamApiName`，用于幂等补同步。
var unlocked_achievements: Dictionary = {}

var room_id := ""
var room_code := ""
var room_players: Array = []

## 本次结算的实际积分变化与段位前后；房间局四项为 null/空串。
var rating_delta = null
var rating_after = null
var tier_before := ""
var tier_after := ""
## 最近一次查询的全服排行榜条目。
var leaderboard: Array = []
## 最近一次查询的个人主页；未查询时为空字典。
var profile: Dictionary = {}
## 个人主页目标玩家与返回页面，由入口页写入。
var profile_target := ""
var profile_return_page := ""

## 最近一条 `QueueState`；未排队时为空字典。
var queue_state: Dictionary = {}
var queueing := false
## 收到 `QueueState` 的本地毫秒时刻；两次下发之间由本地时钟累计等待时长。
var queue_state_at_msec := 0
## `MatchFound` 分配的座位号；未匹配时为 -1。
var match_seat := -1

var phase := ""
var turn := 1
var deadline := ""
var view: Dictionary = {}
var last_seq := 0

var ranking: Array = []
var game_seed := ""
## 服务端下发的本人本局统计；结算页直接展示，不做二次计算。
var self_stats: Dictionary = {}
## `RoomClosed` 的原因：`kicked`（被移出）或 `dissolved`（房间解散）。
var room_closed_reason := ""
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
            _flush_pending_steam_invite()
            auth_changed.emit()
        Protocol.MSG_ACCOUNT_VIEW:
            account = payload
            player_code = str(payload.get("playerCode", player_code))
            display_name = str(payload.get("displayName", display_name))
            account_changed.emit()
        Protocol.MSG_PHONE_CODE_SENT:
            phone_code_expires_at = str(payload.get("expiresAt", ""))
            phone_code_sent.emit()
        Protocol.MSG_ROOM_INVITE_CODE:
            room_invite = payload
            room_invite_received.emit()
        Protocol.MSG_ACHIEVEMENT_STATE:
            var unlocked = payload.get("unlocked", [])
            if unlocked is Array:
                for item in unlocked:
                    if item is Dictionary:
                        _unlock_achievement(item)
        Protocol.MSG_ACHIEVEMENT_UNLOCKED:
            _unlock_achievement(payload)
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
        Protocol.MSG_QUEUE_STATE:
            queue_state = payload
            queueing = true
            queue_state_at_msec = Time.get_ticks_msec()
            queue_changed.emit()
        Protocol.MSG_MATCH_FOUND:
            # 匹配房不下发大厅 `RoomUpdate`；这里先写入房间 id，断线重连才能找回该局。
            match_seat = int(payload.get("seat", -1))
            room_id = str(payload.get("roomId", room_id))
            _clear_queue()
            match_found.emit(match_seat)
        Protocol.MSG_GAME_STARTED:
            _apply_view(payload.get("view", {}))
            game_started.emit()
            view_changed.emit()
        Protocol.MSG_STATE_UPDATE:
            _apply_view(payload.get("view", {}))
            _emit_events(payload.get("events", []))
            view_changed.emit()
        Protocol.MSG_SNAPSHOT:
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
            self_stats = payload.get("selfStats", {})
            # 房间局四项为 null；客户端只搬运，不在本地算分。
            rating_delta = payload.get("ratingDelta")
            rating_after = payload.get("ratingAfter")
            tier_before = str(payload.get("tierBefore", "") if payload.get("tierBefore") != null else "")
            tier_after = str(payload.get("tierAfter", "") if payload.get("tierAfter") != null else "")
            phase = Protocol.PHASE_GAME_OVER
            game_over.emit()
        Protocol.MSG_LEADERBOARD:
            leaderboard = payload.get("entries", [])
            leaderboard_changed.emit()
        Protocol.MSG_PLAYER_PROFILE:
            profile = payload
            profile_changed.emit()
        Protocol.MSG_ROOM_CLOSED:
            # `reset_room` 会清空关闭原因，先暂存再在清空后写回，页面才能区分提示文案。
            var reason := str(payload.get("reason", ""))
            reset_room()
            room_closed_reason = reason
            Net.send(Protocol.MSG_LEAVE_ROOM, {})
            room_closed.emit()
        Protocol.MSG_ERROR:
            last_error = payload
            # 已在房间时服务端拒绝入队；本地排队态已无意义，直接收起浮层。
            if str(payload.get("code", "")) == Protocol.ERROR_QUEUE_IN_ROOM:
                _clear_queue()
            error_received.emit()
        _:
            pass


## 事件只按消息转发一次：同一封 `StateUpdate` 的 `view_changed` 可能来自
## 多个字段，音效与动画不能按视图差异重复推断。
func _emit_events(value) -> void:
    if value is Array and not value.is_empty():
        events_received.emit(value)


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
    self_stats = {}
    rating_delta = null
    rating_after = null
    tier_before = ""
    tier_after = ""
    room_closed_reason = ""
    last_error = {}
    last_combat = {}
    last_combat_turn = -1
    timer_seconds = -1
    room_invite = {}
    match_seat = -1
    _clear_queue()
    if is_instance_valid(Platform):
        Platform.leave_lobby()


## 结算页返回等待区：只清本局视图，保留房间信息与社交列表。
func clear_game_view() -> void:
    phase = ""
    turn = 1
    deadline = ""
    view = {}
    ranking = []
    self_stats = {}
    rating_delta = null
    rating_after = null
    tier_before = ""
    tier_after = ""
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
    account = {}
    leaderboard = []
    profile = {}
    profile_target = ""
    profile_return_page = ""
    phone_code_expires_at = ""
    unlocked_achievements = {}
    pending_steam_invite = ""


## Steam 登录：票据只在登录时使用，续登与重连仍走令牌。
func login_steam(ticket: String, persona_name: String) -> void:
    var payload := {"steamTicket": ticket}
    var name := steam_display_name(persona_name)
    if name != "":
        payload["displayName"] = name
        display_name = name
    Net.send(Protocol.MSG_AUTH_REQUEST, payload)


## Steam 个人资料名只作展示：去空白后按字符截断到 20，与服务端上限一致。
static func steam_display_name(value: String) -> String:
    return value.strip_edges().left(20)


## 处理 Steam 好友邀请：未认证先缓存，已在房间只提示，成功则消费 token 入房。
func accept_steam_invite(invite_token: String) -> void:
    var value := invite_token.strip_edges()
    if value == "":
        return
    if player_id == "" or token == "" or not Net.is_open():
        pending_steam_invite = value
        return
    if room_id != "":
        # 页面按现有错误码展示「请先离开当前房间」，房间状态保持不变。
        last_error = {"code": Protocol.ERROR_ALREADY_IN_ROOM}
        error_received.emit()
        steam_invite_rejected.emit("already_in_room")
        return
    redeem_room_invite(value)


func _flush_pending_steam_invite() -> void:
    if pending_steam_invite == "" or room_id != "":
        return
    var value := pending_steam_invite
    pending_steam_invite = ""
    redeem_room_invite(value)


func request_account() -> void:
    Net.send(Protocol.MSG_REQUEST_ACCOUNT, {})


## 请求全服排行榜；结果经 `MSG_LEADERBOARD` 写入 `leaderboard`。
func request_leaderboard() -> void:
    Net.send(Protocol.MSG_REQUEST_LEADERBOARD, {})


## 当前账号的段位 id（服务端下发；未登录或缺失时为空串）。
func my_tier() -> String:
    return str(account.get("tier", ""))


## 当前账号积分；服务端未下发时为 0。
func my_rating() -> int:
    var value = account.get("rating", 0)
    return 0 if value == null else int(value)


## 打开个人主页：记录目标玩家与返回页面，由页面负责导航。
func open_profile(target_player_id: String, return_page: String) -> void:
    profile_target = target_player_id
    profile_return_page = return_page


## 请求个人主页；`target_player_id` 为空时请求本人。
func request_profile(target_player_id := "") -> void:
    var payload := {}
    if target_player_id != "":
        payload["playerId"] = target_player_id
    Net.send(Protocol.MSG_REQUEST_PROFILE, payload)


func request_phone_code(phone: String) -> void:
    Net.send(Protocol.MSG_REQUEST_PHONE_CODE, {"phone": phone.strip_edges()})


func bind_phone(phone: String, code: String) -> void:
    Net.send(
        Protocol.MSG_BIND_PHONE,
        {"phone": phone.strip_edges(), "code": code.strip_edges()},
    )


func create_room_invite() -> void:
    Net.send(Protocol.MSG_CREATE_ROOM_INVITE, {})


func redeem_room_invite(token: String) -> void:
    if token.strip_edges() == "":
        return
    Net.send(Protocol.MSG_REDEEM_ROOM_INVITE, {"token": token})


## 进入自动匹配队列；本地先进入排队态，人数与窗口以服务端 `QueueState` 为准。
func join_queue() -> void:
    if queueing:
        return
    queueing = true
    queue_state = {}
    queue_state_at_msec = Time.get_ticks_msec()
    queue_changed.emit()
    Net.send(Protocol.MSG_JOIN_QUEUE, {})


## 取消匹配：本地立即收起浮层；服务端取消不计败，也不扣分。
func leave_queue() -> void:
    var was_queueing := queueing
    _clear_queue()
    if was_queueing:
        Net.send(Protocol.MSG_LEAVE_QUEUE, {})


## 断线或回大厅时丢弃本地排队态；不发送取消，服务端按断线清理。
func abandon_queue() -> void:
    _clear_queue()


## 已等待秒数：以最近一条 `QueueState` 的 `waitingMs` 为基准，本地时钟补足间隔。
func queue_wait_seconds() -> float:
    if not queueing:
        return 0.0
    var served := float(int(queue_state.get("waitingMs", 0))) / 1000.0
    if queue_state_at_msec <= 0:
        return served
    var drift := maxi(0, Time.get_ticks_msec() - queue_state_at_msec)
    return served + float(drift) / 1000.0


func _clear_queue() -> void:
    var changed := queueing or not queue_state.is_empty()
    queueing = false
    queue_state = {}
    queue_state_at_msec = 0
    if changed:
        queue_changed.emit()


func _unlock_achievement(item: Dictionary) -> void:
    var achievement_id := str(item.get("achievementId", ""))
    var api_name := str(item.get("steamApiName", ""))
    if achievement_id == "" or api_name == "":
        return
    if str(unlocked_achievements.get(achievement_id, "")) == api_name:
        return
    unlocked_achievements[achievement_id] = api_name
    achievement_unlocked.emit(api_name)


## 尚未过期的待处理邀请；服务端仍是最终校验方，这里只用于避免展示已过期的条目。
func live_invites() -> Array:
    var now := Time.get_unix_time_from_system()
    var result: Array = []
    for invite in pending_invites:
        var expires := UIFormat.unix_from_rfc3339(str(invite.get("expiresAt", "")))
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
