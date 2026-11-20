class_name CardPool
extends RefCounted
## 本局卡池：按服务端 `enabledFactions` 复算图鉴要展示的卡牌集合。
##
## 只用于展示筛选，不参与规则判定；Token 不属于本局卡池。判定与服务端
## `card_in_pool` 保持一致：任一条 `play:` 归属命中启用玩法即入池，没有
## `play:` 归属的卡不入池（加载期已拒绝这类数据）。

## 本局卡池的卡牌 id，按 id 升序；不含 Token。
static func card_ids(enabled: Array) -> Array:
    var enabled_set := {}
    for id in enabled:
        enabled_set[str(id)] = true
    var out: Array = []
    for id in CardDB.cards:
        if CardDB.tokens.has(id):
            continue
        var entry: Dictionary = CardDB.cards[id]
        if in_pool(entry, enabled_set):
            out.append(str(id))
    out.sort()
    return out


## 本局英雄候选池的卡牌 id，按 id 升序；口径与 `hero_in_pool` 一致。
static func hero_ids(enabled: Array) -> Array:
    var enabled_set := {}
    for id in enabled:
        enabled_set[str(id)] = true
    var out: Array = []
    for id in CardDB.heroes:
        var entry: Dictionary = CardDB.heroes[id]
        if in_pool(entry, enabled_set):
            out.append(str(id))
    out.sort()
    return out


## 单张卡是否在本局卡池；`enabled` 为玩法 id 集合。
static func in_pool(entry: Dictionary, enabled: Dictionary) -> bool:
    for playstyle in CardDB.play_categories(entry):
        if enabled.has(playstyle):
            return true
    return false
