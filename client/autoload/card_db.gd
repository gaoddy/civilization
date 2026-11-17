extends Node
## 只读卡牌展示表：卡名、技能文本与展示费用，不参与任何规则判定。

var cards: Dictionary = {}
var tokens: Dictionary = {}

## 时代序号到文本 id，与服务端 `Era` 枚举一致（1–7）。
const ERA_IDS := {
    1: "prehistoric",
    2: "primitive",
    3: "classical",
    4: "feudal",
    5: "industrial",
    6: "modern",
    7: "future",
}


func _ready() -> void:
    _load_table("res://generated/cards.json", cards)
    _load_table("res://generated/tokens.json", tokens)
    for id in tokens:
        cards[id] = tokens[id]


func _load_table(path: String, target: Dictionary) -> void:
    var file := FileAccess.open(path, FileAccess.READ)
    if file == null:
        push_error("无法读取展示表 %s" % path)
        return
    var parsed = JSON.parse_string(file.get_as_text())
    if not (parsed is Array):
        push_error("展示表格式错误 %s" % path)
        return
    for entry in parsed:
        if entry is Dictionary:
            target[str(entry.get("id", ""))] = entry


func card(id: String) -> Dictionary:
    return cards.get(id, {})


## 展示名：Token 取 `token.<id>.name`，其余取 `card.<id>.name`；缺键回退为实体 id。
func display_name(id: String) -> String:
    var key := ("token.%s.name" if tokens.has(id) else "card.%s.name") % id
    var text := tr(key)
    return id if text == key else text


## 技能文本按技能顺序拼接；缺键的技能跳过。
func display_text(id: String) -> String:
    var lines: Array = []
    for skill_id in card(id).get("skills", []):
        var key := "skill.%s.text" % str(skill_id)
        var line := tr(key)
        if line == key:
            continue
        lines.append(line)
    return "\n".join(lines)


func display_cost(id: String) -> int:
    return int(card(id).get("display_cost", 0))


## 时代展示名；未知序号回退为数字文本，缺键回退为文本 id。
func era_name(era: int) -> String:
    var id: String = ERA_IDS.get(era, "")
    if id == "":
        return str(era)
    var key := "enum.era.%s" % id
    var text := tr(key)
    return id if text == key else text


func heroes() -> Array:
    var out: Array = []
    for id in cards:
        var entry: Dictionary = cards[id]
        if bool(entry.get("is_hero", false)):
            out.append(entry)
    out.sort_custom(func(a, b): return str(a.get("id", "")) < str(b.get("id", "")))
    return out
