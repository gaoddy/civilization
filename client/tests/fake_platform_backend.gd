extends "res://platform/dev_backend.gd"
## Steam 登录冒烟用的假后端：票据、大厅与成就结果由用例直接设定。

var ticket := "deadbeef"
var persona := "Steam 玩家"
var reason := ""
var lobby_ok := true
var join_request: Dictionary = {}
var unlocked: Array = []
var invited := false
var left := false

var _ticket_result: Dictionary = {}
var _lobby_result := -2


func request_ticket() -> void:
    if reason != "":
        _ticket_result = {"ok": false, "reason": reason}
        return
    _ticket_result = {"ok": true, "ticket": ticket, "personaName": persona}


func take_ticket_result() -> Dictionary:
    var result := _ticket_result
    _ticket_result = {}
    return result


func persona_name() -> String:
    return persona


func create_lobby(_room_code: String, _invite_token: String, _expires_at: String) -> bool:
    _lobby_result = 123 if lobby_ok else -1
    return true


func take_lobby_result() -> int:
    var result := _lobby_result
    _lobby_result = -2
    return result


func invite_friends() -> void:
    invited = true


func take_join_request() -> Dictionary:
    var request := join_request
    join_request = {}
    return request


func leave_lobby() -> void:
    left = true


func unlock_achievement(api_name: String) -> void:
    unlocked.append(api_name)
