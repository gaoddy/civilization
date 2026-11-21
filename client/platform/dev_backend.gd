extends RefCounted
## 开发构建的渠道后端：不加载 Steam，所有能力返回不可用 / 空凭证。
##
## 测试用假后端直接继承本类并覆写方法，业务层只依赖 `Platform` 暴露的语义接口。


## 启动后端；返回空串表示可用，否则返回 `not_running` / `not_logged_in` /
## `unavailable` 之一。
func start() -> String:
    return ""


## 每帧推进后端回调；开发后端无事可做。
func poll() -> void:
    pass


## 发起取票；结果由 `take_ticket_result` 取回。
func request_ticket() -> void:
    pass


## 取回登录票据；空字典表示仍在等待。
func take_ticket_result() -> Dictionary:
    return {"ok": false, "reason": "unavailable"}


func persona_name() -> String:
    return ""


## 创建 Steam 大厅；返回是否已发起请求。
func create_lobby(_room_code: String, _invite_token: String, _expires_at: String) -> bool:
    return false


## 取回大厅创建结果：`-2` 等待中，`-1` 失败，正数为大厅 id。
func take_lobby_result() -> int:
    return -1


## 打开好友邀请面板。
func invite_friends() -> void:
    pass


## 取回好友通过大厅发来的入房请求；空字典表示没有。
func take_join_request() -> Dictionary:
    return {}


func leave_lobby() -> void:
    pass


## 同步单个成就；调用失败不改变局内状态。
func unlock_achievement(_api_name: String) -> void:
    pass
