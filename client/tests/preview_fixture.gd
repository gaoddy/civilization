extends RefCounted
## 运行态预览夹具：按名称写入 GameStore 与 Net 的最小状态。
##
## 夹具只服务开发预览，放在 tests/ 下；视图类不引用这里，避免生产代码依赖测试数据。

const FixtureScript = preload("res://tests/fixture.gd")

const NAMES := ["game", "game_combat", "lobby", "lobby_queue", "room", "none"]

## 大厅与房间共用的一组假好友：两名在线、一名离线，用于在线数与邀请入口。
const LOBBY_FRIENDS := [
    {"playerId": "f1", "displayName": "好友一号", "status": "online", "tier": "gold", "rating": 1420},
    {"playerId": "f2", "displayName": "好友二号", "status": "online", "tier": "silver", "rating": 1105},
    {"playerId": "f3", "displayName": "好友三号", "status": "offline", "tier": "bronze", "rating": 860},
]

## 房间等待区：房主 p0 已准备，四人已加入、三个空位，开启补位。
const ROOM_PLAYERS := [
    {"playerId": "p0", "displayName": "玩家A", "seat": 0, "ready": true, "isHost": true},
    {"playerId": "p1", "displayName": "玩家B", "seat": 1, "ready": true, "isHost": false},
    {"playerId": "p2", "displayName": "玩家C", "seat": 2, "ready": false, "isHost": false},
    {"playerId": "p4", "displayName": "玩家D", "seat": 4, "ready": true, "isHost": false},
    {"playerId": "p5", "displayName": "玩家E", "seat": 6, "ready": false, "isHost": false},
]


## 每次预览都从干净会话开始，避免上一次的状态残留。
static func apply(name: String) -> bool:
    GameStore.reset_session()
    Net.socket = null
    match name:
        "game":
            return _game()
        "game_combat":
            return _game_combat()
        "lobby":
            return _lobby()
        "lobby_queue":
            return _lobby_queue()
        "room":
            return _room()
        "none":
            return true
        _:
            return false


## 页面没有显式指定夹具时：有同名夹具就用，其余页面看空态。
static func default_for_page(page: String) -> String:
    return page if NAMES.has(page) else "none"


static func names() -> PackedStringArray:
    return PackedStringArray(NAMES)


static func _game() -> bool:
    var view := FixtureScript.game_view()
    GameStore.player_id = "p0"
    GameStore.phase = Protocol.PHASE_RECRUIT
    GameStore.turn = int(view.get("turn", 3))
    GameStore.deadline = str(view.get("phaseDeadline", "2030-01-01T00:00:00Z"))
    GameStore.view = view
    # 假装已连接，页面才会进可操作状态；不发起真实连接。
    Net.socket = FixtureScript.FakeSocket.new()
    return true


## 战斗态预览：同一局视图切到战斗阶段，并补齐当前回合战报，供对手镜像行与河道金币检查。
static func _game_combat() -> bool:
    if not _game():
        return false
    var view: Dictionary = GameStore.view
    view["phase"] = Protocol.PHASE_COMBAT
    view["opponentPlayerId"] = "p2"
    GameStore.phase = Protocol.PHASE_COMBAT
    GameStore.last_combat = FixtureScript.combat_result()
    GameStore.last_combat_turn = GameStore.turn
    return true


## 大厅：已登录、已连接的身份与段位，加一组在线/离线好友，右栏展示默认入口。
static func _lobby() -> bool:
    GameStore.player_id = "p0"
    GameStore.display_name = "玩家A"
    GameStore.player_code = "A7F2K9"
    GameStore.account = {"tier": "platinum", "rating": 1688}
    GameStore.phase = Protocol.PHASE_LOBBY
    GameStore.friends = LOBBY_FRIENDS.duplicate(true)
    Net.socket = FixtureScript.FakeSocket.new()
    return true


## 大厅排队中：在默认大厅状态上打开排队，计时从 42 秒前开始，已凑 3 人。
static func _lobby_queue() -> bool:
    if not _lobby():
        return false
    GameStore.queueing = true
    GameStore.queue_started_at_msec = Time.get_ticks_msec() - 42000
    GameStore.queue_state = {"waitingPlayers": 3}
    return true


## 房间等待区：房主视角，四人已加入、三个空位，补位开启。
static func _room() -> bool:
    if not _lobby():
        return false
    GameStore.room_id = "r_preview"
    GameStore.room_code = "Q3K8ZN"
    GameStore.room_bot_fill = true
    GameStore.room_players = ROOM_PLAYERS.duplicate(true)
    return true
