class_name Fixture
extends RefCounted
## 冒烟用最小合法 PlayerView；字段与 protocol/schema/v1/view.schema.json 一致。


static func view() -> Dictionary:
    return {
        "roomId": "r_test",
        "phase": "Recruit",
        "turn": 3,
        "phaseDeadline": "2030-01-01T00:00:00Z",
        "enabledFactions": ["人类", "机械", "生物", "植物", "学者"],
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
                {"index": 0, "cardId": "dia", "playable": true, "disabledReason": null},
            ],
            "board": [
                {
                    "index": 0,
                    "minionId": "mi_1",
                    "cardId": "huo",
                    "golden": false,
                    "attack": 3,
                    "health": 4,
                    "keywords": ["战吼"],
                    "sellable": true,
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
                "alive": true,
                "boardCount": 0,
                "heroCardId": "tesla",
                "recruited": true,
                "placement": null,
                "managed": false,
            },
        ],
    }
