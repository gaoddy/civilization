extends Node
## 只读卡牌展示表：卡名、技能文本与展示费用，不参与任何规则判定。

var cards: Dictionary = {}
var tokens: Dictionary = {}
## 技能展示表：只取费用等展示字段，不参与规则判定。
var skills: Dictionary = {}
## 科学分类树：节点 id → {id, parent, rank}。
var taxonomy: Dictionary = {}
## 玩法 id 顺序，与 taxonomy.md §2 一致，供图鉴分类目录使用。
var playstyles: Array = []
## 美术资源清单：card_id → {portrait, avatar, card_face, display}。缺图时回退程序化绘制。
var art: Dictionary = {}

## level → 稀有度档位（纯展示，服务端数据与协议不变）。
const RARITY := {
    1: "common", 2: "common",
    3: "rare", 4: "rare",
    5: "epic", 6: "epic",
    7: "legendary",
}

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
    _load_table("res://generated/skills.json", skills)
    _load_taxonomy("res://generated/taxonomy.json")
    _load_art("res://assets/asset_manifest.json")
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


## 分类树为单对象 `{nodes, playstyles}`；按节点 id 建索引，展示名仍走文本表。
func _load_taxonomy(path: String) -> void:
    var file := FileAccess.open(path, FileAccess.READ)
    if file == null:
        push_error("无法读取分类树 %s" % path)
        return
    var parsed = JSON.parse_string(file.get_as_text())
    if not (parsed is Dictionary):
        push_error("分类树格式错误 %s" % path)
        return
    for node in parsed.get("nodes", []):
        if node is Dictionary:
            taxonomy[str(node.get("id", ""))] = node
    playstyles.clear()
    for entry in parsed.get("playstyles", []):
        if entry is String:
            playstyles.append(entry)


## 美术清单是单对象 `{version, assets}`；按 card_id 建索引。文件缺失不报错，
## 让全部界面回退到程序化绘制（无素材时游戏照常可玩）。
func _load_art(path: String) -> void:
    if not FileAccess.file_exists(path):
        return
    var file := FileAccess.open(path, FileAccess.READ)
    if file == null:
        return
    var parsed = JSON.parse_string(file.get_as_text())
    if not (parsed is Dictionary):
        return
    for id in parsed.get("assets", {}):
        if parsed["assets"][id] is Dictionary:
            art[str(id)] = parsed["assets"][id]


func card(id: String) -> Dictionary:
    return cards.get(id, {})


func skill(id: String) -> Dictionary:
    return skills.get(id, {})


## 技能展示费用；技能不存在或缺少 `cost` 时返回 -1，供界面隐藏费用角标。
func skill_cost(id: String) -> int:
    var entry := skill(id)
    if not entry.has("cost"):
        return -1
    return int(entry.get("cost", -1))


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


## 技能展示名：取 `skill.<id>.name`，缺键返回空串，避免内部 id 泄露到界面。
func skill_name(id: String) -> String:
    if id == "":
        return ""
    var key := "skill.%s.name" % id
    var text := tr(key)
    return "" if text == key else text


## 时代展示名；未知序号回退为数字文本，缺键回退为文本 id。
func era_name(era: int) -> String:
    var id: String = ERA_IDS.get(era, "")
    if id == "":
        return str(era)
    var key := "enum.era.%s" % id
    var text := tr(key)
    return id if text == key else text


## 玩法展示名；空 id 返回空串，缺键回退为 id。
func playstyle_name(id: String) -> String:
    if id == "":
        return ""
    var key := "enum.playstyle.%s" % id
    var text := tr(key)
    return id if text == key else text


## 科学分类节点展示名；空 id 返回空串，缺键回退为 id。
func taxon_name(id: String) -> String:
    if id == "":
        return ""
    var key := "enum.taxon.%s" % id
    var text := tr(key)
    return id if text == key else text


## 从叶子沿 parent 上溯到根，返回「界 → 科」顺序的展示名数组。
## 最多 7 层；未知节点或空 id 返回空数组。
func taxon_path(id: String) -> Array:
    var names: Array = []
    var current := id
    while current != "" and taxonomy.has(current) and names.size() < 7:
        names.append(taxon_name(current))
        var parent = taxonomy[current].get("parent")
        current = "" if parent == null else str(parent)
    names.reverse()
    return names


func heroes() -> Array:
    var out: Array = []
    for id in cards:
        var entry: Dictionary = cards[id]
        if bool(entry.get("is_hero", false)):
            out.append(entry)
    out.sort_custom(func(a, b): return str(a.get("id", "")) < str(b.get("id", "")))
    return out


## 该卡是否已有美术资源；无资源时调用方应回退程序化绘制。
func has_art(id: String) -> bool:
    return art.has(id)


## 取某类资源路径（kind: portrait / avatar / card_face）；无资源返回空串。
func art_path(id: String, kind: String) -> String:
    if not art.has(id):
        return ""
    return str(art[id].get(kind, ""))


## 展示用稀有度档位：common / rare / epic / legendary；未知等级回退 common。
func rarity_tier(id: String) -> String:
    var level := int(card(id).get("level", 1))
    return str(RARITY.get(level, "common"))
