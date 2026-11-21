class_name UIWording
extends RefCounted
## 界面文案：枚举展示、错误码文案与错误详情。
##
## 只做翻译查找与回退，不构造控件；缺键时按调用方约定回退到内部 id 或空串。

## 按枚举组与英文 id 取展示文案；缺键回退为 id。
static func enum_text(group: String, id: String) -> String:
    var key := "enum.%s.%s" % [group, id]
    var text := TranslationServer.translate(key)
    return id if text == key else text


## 按完整文本 id 取文案；缺键返回空串，避免内部 id 泄露到界面。
static func translated_or_empty(key: String) -> String:
    var text := TranslationServer.translate(key)
    return "" if text == key else text


## 错误码对应的展示文案；缺键回退为错误码。
static func error_text(code: String) -> String:
    var key := "error.%s" % code
    var text := TranslationServer.translate(key)
    return code if text == key else text


## 按完整文本 id 取错误详情；空 id 或缺键返回空串，params 非空时按 % 格式化。
static func error_text_by_id(id: String, params: Array = []) -> String:
    if id == "":
        return ""
    var text := TranslationServer.translate(id)
    if text == id:
        return ""
    if not params.is_empty():
        text = text % params
    return text


## 错误提示的悬停详情：优先用服务端给出的 messageId 与参数，缺失时回退错误码文案。
static func error_tooltip(code: String) -> String:
    var raw_params = GameStore.last_error.get("messageParams", [])
    var params: Array = raw_params if raw_params is Array else []
    var detail := error_text_by_id(
        str(GameStore.last_error.get("messageId", "")),
        params,
    )
    return detail if detail != "" else error_text(code)
