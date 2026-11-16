extends Node
## 只读卡牌展示表：卡名、技能文本与展示费用，不参与任何规则判定。

var cards: Dictionary = {}
var skills: Dictionary = {}
var tokens: Dictionary = {}

## 时代序号到展示名，与服务端 `Era` 枚举一致（1–7）。
const ERA_NAMES := {
    1: "史前",
    2: "原始",
    3: "古典",
    4: "封建",
    5: "工业",
    6: "现代",
    7: "未来",
}


func _ready() -> void:
    _load_table("res://generated/cards.json", cards)
    _load_table("res://generated/skills.json", skills)
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


func display_name(id: String) -> String:
    return str(card(id).get("name", id))


func display_text(id: String) -> String:
    var entry := card(id)
    var lines: Array = []
    for skill_id in entry.get("skills", []):
        var skill: Dictionary = skills.get(str(skill_id), {})
        var line := str(skill.get("text", ""))
        if line != "":
            lines.append(line)
    return "\n".join(lines)


func display_cost(id: String) -> int:
    return int(card(id).get("display_cost", 0))


## 时代展示名；未知序号回退为数字文本。
func era_name(era: int) -> String:
    return str(ERA_NAMES.get(era, str(era)))


func heroes() -> Array:
    var out: Array = []
    for id in cards:
        var entry: Dictionary = cards[id]
        if bool(entry.get("is_hero", false)):
            out.append(entry)
    out.sort_custom(func(a, b): return str(a.get("id", "")) < str(b.get("id", "")))
    return out
