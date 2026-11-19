class_name CardText
extends RefCounted
## 卡牌与卡面周边共用的显示文本格式化。
##
## 只做取值与格式化，不构造控件；卡面组件、详情层与座位行都从这里取文本。


## 卡面封面的首字水印；空 id 回退问号。
static func initial(card_id: String) -> String:
    var display := CardDB.display_name(card_id)
    return display.substr(0, 1) if display != "" else "?"


## 卡面类型行：卡面宽度有限，只放玩法；完整类型与分类放在详情面板。
static func playstyle_text(entry: Dictionary) -> String:
    var playstyle := str(entry.get("playstyle", ""))
    var text := UI.enum_text("playstyle", playstyle) if playstyle != "" else ""
    if text != "" and text != playstyle:
        return text
    return TranslationServer.translate("ui.game.detail.undetermined")


## 详情类型行：只显示卡牌类别，玩法与分类由详情单独成行。
static func type_text(entry: Dictionary) -> String:
    var card_type := str(entry.get("card_type", ""))
    return UI.enum_text("card_type", card_type) if card_type != "" else "—"


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
