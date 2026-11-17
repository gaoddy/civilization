class_name Fixture
extends RefCounted
## 冒烟用最小合法 PlayerView；字段与 protocol/schema/v1/view.schema.json 一致。


## 冒烟用的假连接：`Net.is_open()` 只读 `get_ready_state()`，其余接口按无动作实现。
class FakeSocket extends RefCounted:
    func get_ready_state() -> int:
        return WebSocketPeer.STATE_OPEN

    func poll() -> void:
        pass

    func get_available_packet_count() -> int:
        return 0

    func send_text(_text: String) -> int:
        return OK


static func view() -> Dictionary:
    return {
        "roomId": "r_test",
        "phase": "Recruit",
        "turn": 3,
        "phaseDeadline": "2030-01-01T00:00:00Z",
        "boardSlots": 7,
        "hpCap": 40,
        "combatLog": [],
        "enabledFactions": ["human", "machine", "biology", "plant", "scholar"],
        "self": {
            "playerId": "p0",
            "seat": 0,
            "hp": 35,
            "era": 2,
            "gold": 6,
            "heroCardId": "darwin",
            "heroPower": {"skillId": "evolution-discovery", "usesLeft": 1},
            "shop": [
                {"index": 0, "cardId": "huo", "frozen": false},
                {"index": 1, "cardId": "lun", "frozen": false},
                {"index": 2, "cardId": null, "frozen": false},
                {"index": 3, "cardId": "che", "frozen": true},
                {"index": 4, "cardId": null, "frozen": false},
                {"index": 5, "cardId": "fei", "frozen": false},
            ],
            "hand": [
                {"index": 0, "cardId": "dia", "playable": true, "requiresTarget": false, "disabledReason": null},
            ],
            "board": [
                {
                    "index": 0,
                    "minionId": "mi_1",
                    "cardId": "huo",
                    "golden": false,
                    "attack": 3,
                    "health": 4,
                    "keywords": ["battlecry"],
                    "sellable": true,
                    "stats": {
                        "attack": {"initial": 3, "permanent": 1, "turn": 0, "aura": 0, "damage": 0, "current": 4},
                        "health": {"initial": 4, "permanent": 2, "turn": -2, "aura": 1, "damage": 3, "current": 2},
                    },
                },
            ],
            "rerollCost": 1,
            "upgradeCost": 5,
            "recruited": false,
            "pendingChoices": [],
            "affordances": {
                "shopSlots": [
                    {"index": 0, "buyable": true, "disabledReason": null},
                    {"index": 1, "buyable": false, "disabledReason": "NOT_ENOUGH_GOLD"},
                    {"index": 2, "buyable": false, "disabledReason": "SHOP_SLOT_EMPTY"},
                    {"index": 3, "buyable": false, "disabledReason": "NOT_ENOUGH_GOLD"},
                    {"index": 4, "buyable": false, "disabledReason": "SHOP_SLOT_EMPTY"},
                    {"index": 5, "buyable": false, "disabledReason": "NOT_ENOUGH_GOLD"},
                ],
                "handCards": [
                    {"index": 0, "playable": true, "disabledReason": null},
                ],
                "boardMinions": [
                    {"index": 0, "sellable": true, "disabledReason": null},
                ],
                "canReroll": true,
                "canUpgradeEra": true,
                "upgradeDisabledReason": null,
                "canUseHeroPower": true,
                "heroPowerDisabledReason": null,
                "canEndRecruit": true,
            },
        },
        "players": [
            {
                "playerId": "p0",
                "seat": 0,
                "displayName": "玩家A",
                "hp": 35,
                "era": 2,
                "rank": 1,
                "alive": true,
                "boardCount": 1,
                "heroCardId": "darwin",
                "recruited": false,
                "placement": null,
                "managed": false,
            },
            {
                "playerId": "p1",
                "seat": 1,
                "displayName": "机器人B",
                "hp": 40,
                "era": 1,
                "rank": 2,
                "alive": true,
                "boardCount": 0,
                "heroCardId": "tesla",
                "recruited": true,
                "placement": null,
                "managed": false,
            },
        ],
    }


## 八人对局视图：棋盘、商店、手牌与积分榜都有内容，用于版式截图与布局断言。
static func game_view() -> Dictionary:
    var view := view()

    view["self"]["board"] = [
        _minion(0, "mi_0", "bruce-lee", 3, 4, ["battlecry"]),
        _minion(1, "mi_1", "airplane", 4, 5, ["divine_shield"]),
        _minion(2, "mi_2", "baiyi", 5, 3, ["contagion"]),
        _minion(3, "mi_3", "black-death", 2, 7, []),
    ]
    view["self"]["affordances"]["boardMinions"] = [
        {"index": 0, "sellable": true, "disabledReason": null},
        {"index": 1, "sellable": true, "disabledReason": null},
        {"index": 2, "sellable": false, "disabledReason": "INVALID_STAGE"},
        {"index": 3, "sellable": true, "disabledReason": null},
    ]
    view["self"]["shop"] = [
        {"index": 0, "cardId": "aristotle", "frozen": false},
        {"index": 1, "cardId": "bingdushizu", "frozen": false},
        {"index": 2, "cardId": null, "frozen": false},
        {"index": 3, "cardId": "alan-turing", "frozen": true},
        {"index": 4, "cardId": "artificial-womb", "frozen": false},
        {"index": 5, "cardId": "bisheng", "frozen": false},
    ]
    view["self"]["hand"] = [
        {"index": 0, "cardId": "archimedes", "playable": true, "requiresTarget": false, "disabledReason": null},
        {"index": 1, "cardId": "boxer", "playable": false, "requiresTarget": false, "disabledReason": "NO_SPACE"},
    ]
    view["self"]["affordances"]["handCards"] = [
        {"index": 0, "playable": true, "disabledReason": null},
        {"index": 1, "playable": false, "disabledReason": "NO_SPACE"},
    ]

    var heroes := ["darwin", "tesla", "edison", "newton", "adam-smith", "galileo", "faraday", "knuth"]
    var names := ["玩家A", "机器人B", "机器人C", "机器人D", "机器人E", "机器人F", "机器人G", "机器人H"]
    var board_counts := [4, 0, 3, 2, 5, 1, 0, 2]
    var players: Array = []
    for seat in range(8):
        var player_id := "p%d" % seat
        var is_self := seat == 0
        players.append({
            "playerId": player_id,
            "seat": seat,
            "displayName": names[seat],
            "hp": 35 - seat * 4,
            "era": 1 + seat % 3,
            "rank": seat + 1,
            "alive": seat != 7,
            "boardCount": board_counts[seat],
            "heroCardId": heroes[seat],
            "recruited": is_self and false,
            "placement": null,
            "managed": false,
        })
    view["players"] = players
    return view


## 战斗结果夹具：双方英雄与随从、动作序列、伤害与胜者，字段与 CombatResult 一致。
static func combat_result() -> Dictionary:
    return {
        "ownBoard": [
            _minion(0, "mi_0", "bruce-lee", 7, 6, ["battlecry"]),
            _minion(1, "mi_1", "airplane", 8, 7, ["divine_shield"]),
            _minion(2, "mi_2", "baiyi", 9, 5, ["contagion"]),
        ],
        "opponentBoard": [
            _minion(0, "mo_0", "aristotle", 6, 8, ["taunt"]),
            _minion(1, "mo_1", "bisheng", 5, 5, []),
        ],
        "actions": [
            {
                "kind": "attack",
                "sourceMinionId": "mi_0",
                "targetMinionId": "mo_0",
                "damage": 6,
                "source": {"minionId": "mi_0", "attack": 7, "health": 6, "alive": true},
                "target": {"minionId": "mo_0", "attack": 6, "health": 2, "alive": true},
            },
            {
                "kind": "death",
                "target": {"minionId": "mo_0", "attack": 6, "health": 0, "alive": false},
            },
            {
                "kind": "summon",
                "summon": {
                    "side": "own",
                    "position": 1,
                    "minion": {
                        "minionId": "mi_9",
                        "cardId": "seedling",
                        "golden": false,
                        "attack": 1,
                        "health": 1,
                        "keywords": [],
                    },
                },
            },
        ],
        "damage": 5,
        "winnerId": "p0",
        "mirror": false,
    }


## 一方清空夹具：对手只有 1 个随从并在首条动作阵亡，用于验证空行固定占位。
static func combat_result_wipe() -> Dictionary:
    return {
        "ownBoard": [
            _minion(0, "mi_0", "bruce-lee", 7, 6, []),
        ],
        "opponentBoard": [
            _minion(0, "mo_0", "aristotle", 6, 8, []),
        ],
        "actions": [
            {
                "kind": "death",
                "sourceSide": "opponent",
                "target": {"minionId": "mo_0", "attack": 6, "health": 0, "alive": false},
            },
        ],
        "damage": 5,
        "winnerId": "p0",
        "mirror": false,
    }


## 复生夹具：第一条阵亡、第二条复生，用于验证行内「阵亡即消失、复生回原位」。
static func combat_result_with_reborn() -> Dictionary:
    return {
        "ownBoard": [
            _minion(0, "mi_0", "bruce-lee", 7, 6, []),
            _minion(1, "mi_1", "airplane", 8, 7, []),
        ],
        "opponentBoard": [],
        "actions": [
            {
                "kind": "death",
                "sourceSide": "own",
                "target": {"minionId": "mi_0", "attack": 7, "health": 0, "alive": false},
            },
            {
                "kind": "reborn",
                "sourceSide": "own",
                "target": {"minionId": "mi_0", "attack": 7, "health": 6, "alive": true},
            },
        ],
        "damage": 0,
        "winnerId": null,
        "mirror": false,
    }


## 手牌条目夹具：`requires_target` 决定它是否需要选目标。
static func hand(card_id: String, requires_target: bool) -> Dictionary:
    return {
        "index": 0,
        "cardId": card_id,
        "playable": true,
        "requiresTarget": requires_target,
        "disabledReason": null,
    }


## 棋盘随从夹具：只填落点判定与展示需要的字段。
##
## `index` 必须与随从在 `self.board` 中的下标一致：页面把它写进卡片的
## `minion_index` 元数据，落点判定据此换算插入下标。
static func board_minion(minion_id: String, card_id: String, index := 0) -> Dictionary:
    return {
        "index": index,
        "minionId": minion_id,
        "cardId": card_id,
        "golden": false,
        "attack": 3,
        "health": 4,
        "keywords": [],
        "sellable": true,
    }


static func _minion(
    index: int,
    minion_id: String,
    card_id: String,
    attack: int,
    health: int,
    keywords: Array,
) -> Dictionary:
    return {
        "index": index,
        "minionId": minion_id,
        "cardId": card_id,
        "golden": false,
        "attack": attack,
        "health": health,
        "keywords": keywords,
        "sellable": true,
    }
