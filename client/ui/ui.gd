class_name UI
extends RefCounted
## 场景共用的小工具；只负责构建 Godot 控件。


static func clear(node: Node) -> void:
    for child in node.get_children():
        node.remove_child(child)
        child.queue_free()


static func label(text: String, size := 16) -> Label:
    var node := Label.new()
    node.text = text
    node.add_theme_font_size_override("font_size", size)
    node.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    return node


static func button(text: String) -> Button:
    var node := Button.new()
    node.text = text
    node.custom_minimum_size = Vector2(0, 34)
    return node


static func panel() -> PanelContainer:
    return PanelContainer.new()


static func margin(container: Control, value := 12) -> MarginContainer:
    var node := MarginContainer.new()
    node.add_theme_constant_override("margin_left", value)
    node.add_theme_constant_override("margin_right", value)
    node.add_theme_constant_override("margin_top", value)
    node.add_theme_constant_override("margin_bottom", value)
    node.add_child(container)
    return node


const ERROR_TEXT := {
    "ALREADY_MAX_ERA": "时代已满",
    "AUTH_FAILED": "登录失败",
    "AUTH_RATE_LIMITED": "登录过于频繁，请稍后再试",
    "BAD_PAYLOAD": "请求格式不正确",
    "CARD_NOT_IN_HAND": "手牌中没有这张牌",
    "HERO_NOT_SELECTED": "还没有选择英雄",
    "HERO_POWER_LIMIT": "英雄技能次数已用完",
    "INTERNAL_ERROR": "服务端内部错误",
    "INVALID_CHOICE": "选择无效",
    "INVALID_STAGE": "当前阶段不能执行该操作",
    "NOT_ENOUGH_GOLD": "金币不足",
    "NOT_HOST": "只有房主可以操作",
    "NOT_IN_ROOM": "已不在房间中",
    "NO_SPACE": "棋盘或手牌已满",
    "NO_SUCH_MINION": "随从不存在",
    "NO_VALID_TARGET": "需要选择目标",
    "RATE_LIMITED": "操作过于频繁，请稍后再试",
    "ROOM_FULL": "房间已满",
    "ROOM_NOT_FOUND": "房间不存在",
    "ROOM_NOT_READY": "房间尚未满足开局条件",
    "SEQUENCE_STALE": "消息序号过期",
    "SESSION_REPLACED": "账号已在其他位置登录",
    "SHOP_SLOT_EMPTY": "商店槽位为空",
    "TARGET_NOT_ALLOWED": "该操作不能指定目标",
    "TOKEN_EXPIRED": "登录已过期，请重新登录",
    "UNKNOWN_TYPE": "未知消息类型",
}


static func error_text(code: String) -> String:
    return ERROR_TEXT.get(code, code)


## 错误码对应的页面去向：login、lobby 或空串（留在当前页面）。
static func error_route(code: String) -> String:
    if code in ["AUTH_FAILED", "TOKEN_EXPIRED", "SESSION_REPLACED"]:
        return "login"
    if code in ["NOT_IN_ROOM", "ROOM_NOT_FOUND"]:
        return "lobby"
    return ""
