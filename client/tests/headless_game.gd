extends Node
## 无头对局驱动：用 1 个真实客户端 + 7 个机器人打完一局。
## 只读 GameStore 中的服务端视图与 affordances，不复制规则。

var _timeout := 300.0
var _elapsed := 0.0
var _finished := false
var _room_requested := false
var _hero_sent := false
var _ready_sent := false
var _intent_counter := 0
var _intents_sent := 0
var _errors := 0
var _soft_errors := 0
var _last_fingerprint := ""
var _pending_buys: Dictionary = {}
var _pending_intents: Dictionary = {}
## 本回合已被服务端以 `NO_VALID_TARGET` 拒绝的出手，键为「回合:手牌下标」。
## 该牌的目标池可能收窄（按分类或玩法过滤），首个己方随从未必在池内；
## 继续重发只会空转，本回合跳过它，交给后续的刷新与结束招募兜底。
var _blocked_hand: Dictionary = {}
## 本回合已被拒绝的目标，键为 `zone:id`；候选池非空，轮换到下一个即可。
var _failed_targets: Dictionary = {}
## 回合内被拒绝的出手次数；棋盘满时手牌没有别的出路，超过上限直接结束招募，
## 否则每来一次视图就空转一轮，直到演示超时。
var _blocked_count := 0
const BLOCKED_LIMIT := 3
var _reroll_count := 0
var _last_turn := 0
var _code_written := false
## 已回报演示完成的回合；无头驱动不播动画，进入战斗即视为播完。
var _combat_ack_turn := -1

## 未确认意图的保留上限，只需覆盖「发出一条后收到下一条错误」的流水线深度。
const PENDING_LIMIT := 8


func _ready() -> void:
    var raw_timeout := OS.get_environment("M1B_TIMEOUT_SECONDS")
    if raw_timeout != "":
        _timeout = float(raw_timeout)
    var display_name := OS.get_environment("M1B_NAME")
    GameStore.display_name = display_name if display_name != "" else "GodotClient"

    GameStore.auth_changed.connect(_on_auth)
    GameStore.room_changed.connect(_on_room)
    GameStore.view_changed.connect(_on_view)
    GameStore.game_over.connect(_on_game_over)
    GameStore.error_received.connect(_on_error)
    Net.connected.connect(_on_connected)
    Net.connect_to(OS.get_environment("M1B_URL"))


func _process(delta: float) -> void:
    if _finished:
        return
    _elapsed += delta
    if _elapsed >= _timeout:
        push_error("M1B 超时：%.0f 秒内未结束对局" % _timeout)
        get_tree().quit(2)


func _on_connected() -> void:
    if GameStore.token != "":
        return
    var payload := {
        "phone": "13900000000",
        "code": "123456",
        "displayName": GameStore.display_name,
    }
    Net.send(Protocol.MSG_AUTH_REQUEST, payload)


func _on_auth() -> void:
    if GameStore.room_id != "" or _room_requested:
        return
    _room_requested = true
    var room := OS.get_environment("M1B_ROOM")
    if room != "":
        Net.send(Protocol.MSG_JOIN_ROOM, {"code": room})
    else:
        Net.send(Protocol.MSG_CREATE_ROOM, {})


func _on_room() -> void:
    if not _room_requested:
        return
    if not _code_written and OS.get_environment("M1B_ROOM") == "" and GameStore.room_code != "":
        _code_written = true
        _write_room_code(GameStore.room_code)
    for player in GameStore.room_players:
        if str(player.get("playerId", "")) != GameStore.player_id:
            continue
        if player.get("heroCardId") == null and not _hero_sent:
            var hero := _pick_hero(player)
            if hero == "":
                return
            _hero_sent = true
            Net.send(Protocol.MSG_SELECT_HERO_CARD, {"cardId": hero})
        if not bool(player.get("ready", false)) and not _ready_sent:
            _ready_sent = true
            Net.send(Protocol.MSG_READY, {"ready": true})


## 无头客户端不渲染动画：进入战斗即回报演示完成，让服务端按播完结算。
##
## 同一回合只回报一次；断线重连后服务端仍按 `CombatPlayed` 幂等处理。
func _ack_combat() -> void:
    if GameStore.phase != Protocol.PHASE_COMBAT or not GameStore.combat_is_current():
        return
    if _combat_ack_turn == GameStore.turn:
        return
    _combat_ack_turn = GameStore.turn
    Net.send(Protocol.MSG_COMBAT_PLAYED, {})


## 从服务端分配给本人的候选里挑英雄：优先环境变量指定，不在候选里就用第一张。
func _pick_hero(player: Dictionary) -> String:
    var raw = player.get("heroCandidates")
    var candidates: Array = raw if raw is Array else []
    if candidates.is_empty():
        return ""
    var preferred := OS.get_environment("M1B_HERO")
    for candidate in candidates:
        if str(candidate) == preferred:
            return preferred
    return str(candidates[0])


func _on_view() -> void:
    _ack_combat()
    if _finished or GameStore.phase != Protocol.PHASE_RECRUIT:
        return
    var view: Dictionary = GameStore.view
    var self_state: Dictionary = view.get("self", {})
    if self_state.is_empty():
        return
    if int(view.get("turn", 0)) != GameStore.turn:
        return
    var fingerprint := "%d|%s" % [int(view.get("turn", 0)), JSON.stringify(self_state)]
    if fingerprint == _last_fingerprint:
        return
    _last_fingerprint = fingerprint
    if OS.get_environment("M1B_DEBUG") != "":
        _debug_view(view, self_state)
    _act(self_state)


func _debug_view(view: Dictionary, self_state: Dictionary) -> void:
    var shop := PackedStringArray()
    for slot in self_state.get("shop", []):
        var card = slot.get("cardId")
        shop.append("%d:%s" % [int(slot.get("index", 0)), "null" if card == null else str(card)])
    var buyable := PackedStringArray()
    for slot_aff in self_state.get("affordances", {}).get("shopSlots", []):
        if bool(slot_aff.get("buyable", false)):
            buyable.append(str(int(slot_aff.get("index", 0))))
    print("M1B_VIEW seq=%d turn=%d gold=%d hp=%d era=%d hand=%d board=%d recruited=%s shop=[%s] buyable=[%s]" % [
        GameStore.last_seq,
        int(view.get("turn", 0)),
        int(self_state.get("gold", 0)),
        int(self_state.get("hp", 0)),
        int(self_state.get("era", 0)),
        self_state.get("hand", []).size(),
        self_state.get("board", []).size(),
        str(bool(self_state.get("recruited", false))),
        ",".join(shop),
        ",".join(buyable),
    ])


func _act(self_state: Dictionary) -> void:
    # 换回合时先清回合内的拒绝记录，避免被拒的手牌或待选继续占着决策顺序。
    var turn := int(GameStore.view.get("turn", 0))
    if turn != _last_turn:
        _last_turn = turn
        _pending_buys.clear()
        _reroll_count = 0
        _blocked_hand.clear()
        _failed_targets.clear()
        _blocked_count = 0
    if bool(self_state.get("recruited", false)):
        return

    var choices: Array = self_state.get("pendingChoices", [])
    if not choices.is_empty():
        if str(choices[0].get("kind", "")) == "target":
            _choose_target()
            return
        _send_intent({"kind": Protocol.INTENT_CHOOSE_DISCOVER, "choice": 0})
        return

    var aff: Dictionary = self_state.get("affordances", {})
    var best_slot := -1
    var best_cost := -1
    for slot in self_state.get("shop", []):
        var card_id := str(slot.get("cardId", ""))
        if card_id == "":
            continue
        var index := int(slot.get("index", 0))
        if _pending_buys.has(_buy_key(index)):
            continue
        var slot_aff := _find_by_index(aff.get("shopSlots", []), index)
        if not bool(slot_aff.get("buyable", false)):
            continue
        var cost := CardDB.display_cost(card_id)
        if cost > best_cost:
            best_cost = cost
            best_slot = index
    if best_slot >= 0:
        _pending_buys[_buy_key(best_slot)] = true
        _send_intent({"kind": Protocol.INTENT_BUY_MINION, "slot": best_slot})
        return

    # 本回合已被拒的目标出手太多：棋盘满时手牌没有别的出路，直接结束招募。
    if _blocked_count >= BLOCKED_LIMIT and bool(aff.get("canEndRecruit", false)):
        _send_intent({"kind": Protocol.INTENT_END_RECRUIT})
        return

    for card in self_state.get("hand", []):
        var hand_index := int(card.get("index", 0))
        if not bool(card.get("playable", false)) or _hand_blocked(hand_index):
            continue
        var intent := {"kind": Protocol.INTENT_PLAY_CARD, "handIndex": hand_index}
        if bool(card.get("requiresTarget", false)):
            # 只从服务端下发的候选里选；候选为空时该牌本来就不可打出。
            var target := _next_candidate(Array(card.get("targetCandidates", [])), {})
            if target.is_empty():
                continue
            intent["target"] = target
        _send_intent(intent)
        return

    if bool(aff.get("canUpgradeEra", false)):
        _send_intent({"kind": Protocol.INTENT_UPGRADE_ERA})
        return
    if bool(aff.get("canReroll", false)):
        _reroll_count += 1
        _send_intent({"kind": Protocol.INTENT_REROLL_SHOP})
        return
    if bool(aff.get("canEndRecruit", false)):
        _send_intent({"kind": Protocol.INTENT_END_RECRUIT})


func _find_by_index(items: Array, index: int) -> Dictionary:
    for item in items:
        if int(item.get("index", -1)) == index:
            return item
    return {}


func _buy_key(slot: int) -> String:
    # 同一回合内同一槽位只买一次；刷新后槽位重填，允许再次购买。
    return "%d:%d" % [slot, _reroll_count]


func _send_intent(intent: Dictionary) -> void:
    _intent_counter += 1
    _intents_sent += 1
    var intent_id := "headless-%d-%d" % [Time.get_ticks_usec(), _intent_counter]
    print("M1B_INTENT %s %s" % [intent_id, JSON.stringify(intent)])
    if _pending_intents.size() >= PENDING_LIMIT:
        _pending_intents.erase(_pending_intents.keys()[0])
    _pending_intents[intent_id] = intent.duplicate(true)
    Net.send(Protocol.MSG_INTENT, {
        "clientIntentId": intent_id,
        "intent": intent,
    })


## 目标被拒后沿该牌下发的候选换下一个；候选遍历完返回 `false`。
##
## 不带目标的出手不再补选：服务端要求目标时必须直接带视图候选提交。
func _retry_with_target(intent: Dictionary) -> bool:
    if str(intent.get("kind", "")) != Protocol.INTENT_PLAY_CARD:
        return false
    if not intent.has("target"):
        return false
    var hand_index := int(intent.get("handIndex", -1))
    var card := _hand_card(hand_index)
    if card.is_empty():
        return false
    var target := _next_candidate(Array(card.get("targetCandidates", [])), intent["target"])
    if target.is_empty():
        return false
    var retried: Dictionary = intent.duplicate(true)
    retried["target"] = target
    _send_intent(retried)
    return true


## 候选列表里第一个未被拒绝的目标；`after` 非空时从它之后继续。没有时返回空字典。
func _next_candidate(candidates: Array, after: Dictionary) -> Dictionary:
    var skip := not after.is_empty()
    for candidate in candidates:
        if not (candidate is Dictionary):
            continue
        var key := _target_key(candidate)
        if _failed_targets.has(key):
            continue
        if skip:
            if key == _target_key(after):
                skip = false
            continue
        return candidate.duplicate()
    return {}


## 目标的稳定键：区域 + 实例编号。
func _target_key(target: Dictionary) -> String:
    return "%s:%s" % [str(target.get("zone", "")), str(target.get("id", ""))]


## 按手牌下标取视图条目；缺失时返回空字典。
func _hand_card(hand_index: int) -> Dictionary:
    for card in GameStore.view.get("self", {}).get("hand", []):
        if int(card.get("index", -1)) == hand_index:
            return card
    return {}


## 战吼目标待选：只取服务端下发的候选，跳过本回合已被拒绝的目标。
func _choose_target() -> void:
    var choices: Array = GameStore.view.get("self", {}).get("pendingChoices", [])
    var candidates: Array = choices[0].get("candidates", []) if not choices.is_empty() else []
    var target := _next_candidate(candidates, {})
    if target.is_empty():
        push_error("M1B 目标待选时没有可用候选")
        get_tree().quit(4)
        return
    _send_intent({"kind": Protocol.INTENT_CHOOSE_TARGET, "target": target})


## 队首目标待选是否还有未被拒绝的候选。
func _has_pending_candidate() -> bool:
    var choices: Array = GameStore.view.get("self", {}).get("pendingChoices", [])
    if choices.is_empty() or str(choices[0].get("kind", "")) != "target":
        return false
    return not _next_candidate(Array(choices[0].get("candidates", [])), {}).is_empty()


## 本回合内该手牌是否已被判定为目标不合法。
func _hand_blocked(index: int) -> bool:
    return _blocked_hand.has("%d:%d" % [int(GameStore.view.get("turn", 0)), index])


## 记录一次目标拒绝的来源：出手记到手牌（本回合跳过），目标待选记到随从（下个候选）。
## 返回是否记录到，用于判定该拒绝是可预期的目标池收窄而非缺陷。
func _remember_target_rejection(rejected: Dictionary) -> bool:
    var kind := str(rejected.get("kind", ""))
    if kind == Protocol.INTENT_PLAY_CARD:
        if rejected.has("target"):
            var turn := int(GameStore.view.get("turn", 0))
            var key := "%d:%d" % [turn, int(rejected.get("handIndex", -1))]
            if not _blocked_hand.has(key):
                _blocked_hand[key] = true
                _blocked_count += 1
            return true
    elif kind == Protocol.INTENT_CHOOSE_TARGET:
        var target: Dictionary = rejected.get("target", {})
        if not target.is_empty():
            _failed_targets[_target_key(target)] = true
            return true
    return false


func _write_room_code(code: String) -> void:
    print("ROOM_CODE=%s" % code)
    var path := OS.get_environment("M1B_CODE_FILE")
    if path == "":
        return
    var file := FileAccess.open(path, FileAccess.WRITE)
    if file == null:
        push_error("无法写入房间码文件 %s" % path)
        return
    file.store_string(code)
    file.close()


func _on_game_over() -> void:
    if _finished:
        return
    _finished = true
    var place := 0
    for entry in GameStore.ranking:
        if str(entry.get("playerId", "")) == GameStore.player_id:
            place = int(entry.get("place", 0))
    print("M1B_GAME_OVER place=%d seed=%s ranking=%d intents=%d errors=%d soft=%d" % [
        place,
        GameStore.game_seed,
        GameStore.ranking.size(),
        _intents_sent,
        _errors,
        _soft_errors,
    ])
    get_tree().quit(0)


func _on_error() -> void:
    _errors += 1
    var code := str(GameStore.last_error.get("code", ""))
    # 目标缺失是正常交互：补上己方第一个随从重发，计入软错误。
    if code == Protocol.ERROR_NO_VALID_TARGET:
        var rejected_id := str(GameStore.last_error.get("clientIntentId", ""))
        var rejected: Dictionary = _pending_intents.get(rejected_id, {})
        var recorded := _remember_target_rejection(rejected)
        # 阶段已离开招募（战斗已开始）时不再补发，避免把补选打进战斗阶段。
        var still_recruiting := GameStore.phase == Protocol.PHASE_RECRUIT
        if (
            still_recruiting
            and _pending_intents.has(rejected_id)
            and _retry_with_target(_pending_intents[rejected_id])
        ):
            _soft_errors += 1
            push_warning("M1B 补选目标后重发（intent=%s）" % rejected_id)
            return
        if recorded:
            # 目标池收窄导致的选择被拒：本回合跳过该牌或换下一个待选，按预期软错误处理。
            _soft_errors += 1
            if (
                str(rejected.get("kind", "")) == Protocol.INTENT_CHOOSE_TARGET
                and still_recruiting
                and _has_pending_candidate()
            ):
                # 拒绝不会带来新视图；不立即换下一个随从重发，整局会停在招募阶段。
                _choose_target()
            return
    # 被拒绝的购买允许重试，不再视为待确认。
    _pending_buys.clear()
    if code in [
        Protocol.ERROR_NO_SPACE,
        Protocol.ERROR_NOT_ENOUGH_GOLD,
        Protocol.ERROR_SHOP_SLOT_EMPTY,
    ]:
        # 驱动会在上一意图尚未确认时继续决策，服务端按最新状态拒绝属于预期；
        # 这三类不计为缺陷，其余任何错误都判失败。
        _soft_errors += 1
    push_warning("M1B 收到错误 %s：%s（intent=%s）" % [
        code,
        str(GameStore.last_error.get("message", "")),
        str(GameStore.last_error.get("clientIntentId", "")),
    ])
    if code in [
        Protocol.ERROR_AUTH_FAILED,
        Protocol.ERROR_TOKEN_EXPIRED,
        Protocol.ERROR_ROOM_NOT_FOUND,
        Protocol.ERROR_ROOM_FULL,
    ]:
        get_tree().quit(3)
