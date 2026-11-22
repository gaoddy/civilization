extends RefCounted
## 运行态预览夹具：按名称写入 GameStore 与 Net 的最小状态。
##
## 夹具只服务开发预览，放在 tests/ 下；视图类不引用这里，避免生产代码依赖测试数据。

const FixtureScript = preload("res://tests/fixture.gd")

const NAMES := ["game", "game_combat", "none"]


## 每次预览都从干净会话开始，避免上一次的状态残留。
static func apply(name: String) -> bool:
    GameStore.reset_session()
    Net.socket = null
    match name:
        "game":
            return _game()
        "game_combat":
            return _game_combat()
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
