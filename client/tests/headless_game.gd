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
var _last_fingerprint := ""
var _code_written := false


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
    Net.send("AuthRequest", payload)


func _on_auth() -> void:
    if GameStore.room_id != "" or _room_requested:
        return
    _room_requested = true
    var room := OS.get_environment("M1B_ROOM")
    if room != "":
        Net.send("JoinRoom", {"code": room})
    else:
        Net.send("CreateRoom", {})


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
            _hero_sent = true
            var hero := OS.get_environment("M1B_HERO")
            Net.send("SelectHeroCard", {"cardId": hero if hero != "" else "darwin"})
        if not bool(player.get("ready", false)) and not _ready_sent:
            _ready_sent = true
            Net.send("Ready", {"ready": true})


func _on_view() -> void:
    if _finished or GameStore.phase != "Recruit":
        return
    var view: Dictionary = GameStore.view
    var self_state: Dictionary = view.get("self", {})
    if self_state.is_empty():
        return
    var fingerprint := "%d|%s" % [int(view.get("turn", 0)), JSON.stringify(self_state)]
    if fingerprint == _last_fingerprint:
        return
    _last_fingerprint = fingerprint
    _act(self_state)


func _act(self_state: Dictionary) -> void:
    var choices: Array = self_state.get("pendingChoices", [])
    if not choices.is_empty():
        _send_intent({"kind": "ChooseDiscover", "choice": 0})
        return

    var aff: Dictionary = self_state.get("affordances", {})
    var best_slot := -1
    var best_cost := -1
    for slot in self_state.get("shop", []):
        var card_id := str(slot.get("cardId", ""))
        if card_id == "":
            continue
        var index := int(slot.get("index", 0))
        var slot_aff := _find_by_index(aff.get("shopSlots", []), index)
        if not bool(slot_aff.get("buyable", false)):
            continue
        var cost := CardDB.display_cost(card_id)
        if cost > best_cost:
            best_cost = cost
            best_slot = index
    if best_slot >= 0:
        _send_intent({"kind": "BuyMinion", "slot": best_slot})
        return

    for card in self_state.get("hand", []):
        if bool(card.get("playable", false)):
            _send_intent({"kind": "PlayCard", "handIndex": int(card.get("index", 0))})
            return

    if bool(aff.get("canUpgradeEra", false)):
        _send_intent({"kind": "UpgradeEra"})
        return
    if bool(aff.get("canReroll", false)):
        _send_intent({"kind": "RerollShop"})
        return
    if bool(aff.get("canEndRecruit", false)):
        _send_intent({"kind": "EndRecruit"})


func _find_by_index(items: Array, index: int) -> Dictionary:
    for item in items:
        if int(item.get("index", -1)) == index:
            return item
    return {}


func _send_intent(intent: Dictionary) -> void:
    _intent_counter += 1
    Net.send("Intent", {
        "clientIntentId": "headless-%d-%d" % [Time.get_ticks_usec(), _intent_counter],
        "intent": intent,
    })


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
    print("M1B_GAME_OVER place=%d seed=%s" % [place, GameStore.game_seed])
    get_tree().quit(0)


func _on_error() -> void:
    var code := str(GameStore.last_error.get("code", ""))
    push_warning("M1B 收到错误 %s：%s" % [code, str(GameStore.last_error.get("message", ""))])
    if code in ["AUTH_FAILED", "TOKEN_EXPIRED", "ROOM_NOT_FOUND", "ROOM_FULL"]:
        get_tree().quit(3)
