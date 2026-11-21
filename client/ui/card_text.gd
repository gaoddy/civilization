class_name CardText
extends RefCounted
## 卡牌与卡面周边共用的显示文本格式化。
##
## 只做取值与格式化，不构造控件；卡面组件、详情层与座位行都从这里取文本。

## 关键词展示顺序：沿用关键词判定表，未知词排在末尾。
const KEYWORD_ORDER := [
    "breed", "battlecry", "deathrattle", "reborn", "taunt", "divine_shield",
    "windfury", "contagion", "ignore_taunt", "poison", "armor",
    "stealth", "assemble", "sow", "evolve", "decompose",
]


## 卡面封面的首字水印；空 id 回退问号。
static func initial(card_id: String) -> String:
    var display := CardDB.display_name(card_id)
    return display.substr(0, 1) if display != "" else "?"


## 卡面类型行：卡面宽度有限，只放玩法；完整类型与分类放在详情面板。
## 多条玩法按固定玩法顺序排列，用「、」分隔。
static func playstyle_text(entry: Dictionary) -> String:
    var names := PackedStringArray()
    for playstyle in CardDB.play_categories(entry):
        var name := CardDB.playstyle_name(playstyle)
        if name != "":
            names.append(name)
    if names.is_empty():
        return TranslationServer.translate("ui.game.detail.undetermined")
    return "、".join(names)


## 详情「关联分类」：只取捕食与寄生分类，按「捕食：…；寄生：…」聚合。
## 两类都为空时返回空串，由调用方省略该行。
static func association_text(entry: Dictionary) -> String:
    var parts := PackedStringArray()
    var prey := _taxon_names(entry, "prey")
    var host := _taxon_names(entry, "host")
    if not prey.is_empty():
        parts.append("%s：%s" % [TranslationServer.translate("ui.game.detail.prey"), "、".join(prey)])
    if not host.is_empty():
        parts.append("%s：%s" % [TranslationServer.translate("ui.game.detail.host"), "、".join(host)])
    return "；".join(parts)


## 某一关联分类的展示名列表；空值与非字符串条目跳过。
static func _taxon_names(entry: Dictionary, kind: String) -> PackedStringArray:
    var names := PackedStringArray()
    for value in CardDB.category_values(entry, kind):
        var name := CardDB.taxon_name(value)
        if name != "":
            names.append(name)
    return names


## 详情类型行：只显示卡牌类别，玩法与分类由详情单独成行。
static func type_text(entry: Dictionary) -> String:
    var card_type := str(entry.get("card_type", ""))
    return UIWording.enum_text("card_type", card_type) if card_type != "" else "—"


## 卡面数值：服务端或卡表缺失时用破折号占位。
static func stat_text(value: Variant) -> String:
    if value == null:
        return "—"
    return str(int(value))


static func stat_layers(value: Variant) -> Dictionary:
    if typeof(value) == TYPE_DICTIONARY:
        return value
    return {}


static func signed(value: int) -> String:
    return "+%d" % value if value >= 0 else str(value)


## 受击行以「损失量」呈现：层和高于当前量为负数，复生超过层和为正数。
static func damage_text(value: int) -> String:
    if value > 0:
        return "-%d" % value
    if value < 0:
        return "+%d" % -value
    return "0"


## 统一时代文案：顶栏、积分榜、属性行与战斗条都显示具体时代名。
static func era_text(era: int) -> String:
    return TranslationServer.translate("ui.game.era_format") % CardDB.era_name(era)


## 可空字段转文本：缺失统一为空串。
static func optional_text(value: Variant) -> String:
    if value == null:
        return ""
    return str(value)


## 关键词基名：`breed 2` → `breed`；无层数的词原样返回。
static func keyword_base(raw: String) -> String:
    var parts := raw.split(" ", false, 1)
    return parts[0] if not parts.is_empty() else raw


## 关键词层数：`breed 2` → `2`；无层数返回空串。
static func keyword_layer(raw: String) -> String:
    var parts := raw.split(" ", false, 1)
    return parts[1] if parts.size() > 1 else ""


## 关键词展示名：枚举名加层数，如「减伤 2」；缺文案回退英文 id。
static func keyword_name(raw: String) -> String:
    var name := UIWording.enum_text("keyword", keyword_base(raw))
    var layer := keyword_layer(raw)
    return "%s %s" % [name, layer] if layer != "" else name


## 关键词规则说明；缺文案或未知词返回空串，由调用方省略该行。
static func keyword_description(raw: String) -> String:
    var key := "ui.game.detail.keyword.%s" % keyword_base(raw)
    var text := TranslationServer.translate(key)
    if text == key:
        return ""
    var layer := keyword_layer(raw)
    if layer == "" or not text.contains("%d"):
        return text
    return text % int(layer)


## 对局中获得的关键词：当前关键词里卡面技能未登记的部分。
## 卡表缺失时无法区分卡面与获得项，返回空数组；层数增加按完整字符串视为获得。
static func gained_keywords(card_id: String, current: Variant) -> Array:
    var entry := CardDB.card(card_id)
    if entry.is_empty() or not (current is Array):
        return []
    var skills: Variant = entry.get("skills", [])
    var base := {}
    if skills is Array:
        for skill_id in skills:
            var keyword: Variant = CardDB.skill(str(skill_id)).get("keyword")
            if keyword != null and str(keyword) != "":
                base[str(keyword)] = true
    var gained: Array = []
    for value in current:
        var raw := str(value)
        if raw != "" and not base.has(raw) and not gained.has(raw):
            gained.append(raw)
    gained.sort_custom(func(a: String, b: String) -> bool:
        return _keyword_before(a, b)
    )
    return gained


## 按固定关键词顺序排序；未知词排在已知词之后，彼此按 id 排序。
static func _keyword_before(a: String, b: String) -> bool:
    var rank_a := KEYWORD_ORDER.find(keyword_base(a))
    var rank_b := KEYWORD_ORDER.find(keyword_base(b))
    if rank_a == -1 and rank_b == -1:
        return a < b
    if rank_a == -1:
        return false
    if rank_b == -1:
        return true
    return rank_a < rank_b
