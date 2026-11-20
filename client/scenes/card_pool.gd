class_name CardPool
extends RefCounted
## 本局卡池：按服务端 `enabledFactions` 复算图鉴要展示的卡牌集合。
##
## 只用于展示筛选，不参与规则判定；Token 不属于本局卡池。判定与服务端
## `card_in_pool` 保持一致：非中立卡看玩法是否启用；中立卡带 `play` 关联时，
## 至少一个关联玩法启用才入池，没有关联则始终入池。

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
    var playstyle := str(entry.get("playstyle", ""))
    if playstyle != "neutral":
        return enabled.has(playstyle)
    var any := false
    var specialty: Variant = entry.get("specialty", [])
    if specialty is Array:
        for item in specialty:
            if not (item is Dictionary) or str(item.get("kind", "")) != "play":
                continue
            any = true
            if enabled.has(str(item.get("value", ""))):
                return true
    return not any
