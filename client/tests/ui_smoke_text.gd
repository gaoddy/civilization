extends Node
## 文本表内容夹具：下方中文值断言校验文本表与取词接口，不是界面文案断言。


func _ready() -> void:
    _check(CardDB.display_name("adam-smith") == "亚当·斯密", "卡名")
    _check(CardDB.display_name("seedling") == "幼苗", "Token 名")
    _check(CardDB.display_text("adam-smith").contains("成长"), "技能文本")
    _check(CardDB.era_name(5) == "工业", "时代名")
    _check(UI.enum_text("tribe", "human") == "人类", "部落名")
    _check(UI.enum_text("evolve", "divine_shield") == "护盾", "进化选项")
    _check(UI.error_text("ROOM_NOT_FOUND") == "房间不存在", "错误码文案")
    _check(UI.error_text_by_id("error.AUTH_FAILED.invalid_code") == "短信验证码错误", "错误详情")
    _check(UI.error_text_by_id("error.DOES_NOT_EXIST") == "", "缺键回退为空串")
    print("UI_SMOKE_OK")
    get_tree().quit(0)


func _check(ok: bool, label: String) -> void:
    if not ok:
        push_error("TEXT_SMOKE_FAIL: " + label)
        get_tree().quit(1)
