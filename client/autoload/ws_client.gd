extends Node
## WebSocket 客户端：连接、自动重连、信封编解码与消息分发。

signal connected
signal disconnected

const RETRY_SECONDS := 3.0
const DEFAULT_URL := "ws://127.0.0.1:8080/ws"

var url := DEFAULT_URL
## 连接句柄：生产环境恒为 `WebSocketPeer`；冒烟可替换成只实现就绪状态的桩，
## 以便在没有服务端时验证「已连接」分支。`null` 表示未连接。
var socket = null

var _want_connection := false
var _was_open := false
var _retry_left := 0.0
var _counter := 0
var _pending_reconnect := false


func _ready() -> void:
    set_process(true)


func connect_to(target_url: String) -> void:
    url = target_url if target_url.strip_edges() != "" else DEFAULT_URL
    _want_connection = true
    _retry_left = 0.0
    _open()


func disconnect_from_server() -> void:
    _want_connection = false
    _pending_reconnect = false
    if socket != null:
        socket.close()
        socket = null
    if _was_open:
        _was_open = false
        disconnected.emit()


## 关闭当前连接并用已有令牌重新认证，用于对局结束后回到大厅。
func relogin() -> void:
    _want_connection = true
    _pending_reconnect = false
    if socket != null:
        socket.close()
        socket = null
    if _was_open:
        _was_open = false
        disconnected.emit()
    _retry_left = 0.0
    _open()


func is_open() -> bool:
    return socket != null and socket.get_ready_state() == WebSocketPeer.STATE_OPEN


func send(type: String, payload: Dictionary) -> String:
    if not is_open():
        return ""
    _counter += 1
    var id := "c-%d-%d" % [Time.get_ticks_usec(), _counter]
    var envelope := {
        "v": 1,
        "id": id,
        "type": type,
        "ts": Time.get_datetime_string_from_system(true) + "Z",
        "payload": payload,
    }
    if socket.send_text(JSON.stringify(envelope)) != OK:
        return ""
    return id

func _process(delta: float) -> void:
    if socket == null:
        _tick_retry(delta)
        return

    socket.poll()
    var state: int = socket.get_ready_state()
    if state == WebSocketPeer.STATE_OPEN:
        if not _was_open:
            _was_open = true
            connected.emit()
            _resume_session()
        while socket.get_available_packet_count() > 0:
            _on_packet(socket.get_packet().get_string_from_utf8())
    elif state == WebSocketPeer.STATE_CLOSED:
        socket = null
        if _was_open:
            _was_open = false
            disconnected.emit()
        _tick_retry(delta)


func _tick_retry(delta: float) -> void:
    if not _want_connection:
        return
    _retry_left -= delta
    if _retry_left <= 0.0:
        _retry_left = RETRY_SECONDS
        _open()


func _open() -> void:
    if socket != null:
        socket.close()
        socket = null
    # 替换连接后重新开始：避免旧连接的 _was_open 让新连接漏发 connected 与续连。
    _was_open = false
    socket = WebSocketPeer.new()
    # dev 走 platform 自签证书域名时打开该设置；生产构建保持 false。
    var insecure_tls: bool = ProjectSettings.get_setting("network/allow_insecure_tls", false)
    var tls: TLSOptions = TLSOptions.client_unsafe() if insecure_tls else null
    var err: int = socket.connect_to_url(url, tls)
    if err != OK:
        push_error("连接 %s 失败：%s" % [url, error_string(err)])
        socket = null


func _resume_session() -> void:
    if GameStore.token == "":
        return
    send(Protocol.MSG_AUTH_REQUEST, {"token": GameStore.token})
    _pending_reconnect = GameStore.room_id != ""


func _on_packet(text: String) -> void:
    var parsed = JSON.parse_string(text)
    if not (parsed is Dictionary):
        return
    if int(parsed.get("v", 0)) != 1:
        return
    var type := str(parsed.get("type", ""))
    if type == "":
        return
    var seq := int(parsed.get("seq", 0))
    GameStore.last_seq = maxi(GameStore.last_seq, seq)
    var payload = parsed.get("payload", {})
    if not (payload is Dictionary):
        payload = {}
    GameStore.handle(type, payload)
    if type == Protocol.MSG_AUTH_RESULT and _pending_reconnect:
        _pending_reconnect = false
        send(Protocol.MSG_RECONNECT, {"roomId": GameStore.room_id, "lastSeq": GameStore.last_seq})
