extends Node
## WebSocket 客户端：连接、自动重连、信封编解码与消息分发。

signal connected
signal disconnected

const RETRY_SECONDS := 3.0
const DESKTOP_URL := "ws://127.0.0.1:8080/ws"
## 移动端默认连开发机的 Bonjour 主机名：真机与开发机同一局域网即可直连，
## 开发机 IP 变化不影响。主机名取 `scutil --get LocalHostName`，换机器时改这里。
const MOBILE_URL := "ws://cooter.local:8080/ws"

var url := default_url()
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
    url = target_url if target_url.strip_edges() != "" else default_url()
    _want_connection = true
    _retry_left = 0.0
    _open()


## 当前平台的默认调试地址：桌面端连本机，移动端连局域网内的开发机。
static func default_url() -> String:
    return default_url_for(OS.has_feature("mobile"))


## 平台到默认地址的纯函数，供冒烟按平台直接断言。
static func default_url_for(mobile: bool) -> String:
    return MOBILE_URL if mobile else DESKTOP_URL


## 连接前把 URL 里的主机名换成解析出的 IPv4。
##
## `WebSocketPeer` 只取解析结果的第一条地址，双栈主机名（如 `cooter.local`）常先拿到
## IPv6，而服务端只监听 IPv4，直接连接会失败。主机已是 IP 字面量、解析失败或拿到
## 非 IPv4 时原样返回，交给连接层报错。
static func prefer_ipv4_url(url: String, resolver = null) -> String:
    var sep := url.find("://")
    if sep < 0:
        return url
    var rest := url.substr(sep + 3)
    var slash := rest.find("/")
    var authority := rest if slash < 0 else rest.substr(0, slash)
    var tail := "" if slash < 0 else rest.substr(slash)
    # IPv6 字面量必须带方括号，不需要也不应该改写。
    if authority.begins_with("["):
        return url
    var host := authority
    var port := ""
    var colon := authority.rfind(":")
    if colon >= 0:
        host = authority.substr(0, colon)
        port = authority.substr(colon)
    if host.is_valid_ip_address():
        return url
    var ip := ""
    if resolver is Callable and resolver.is_valid():
        ip = str(resolver.call(host))
    else:
        ip = IP.resolve_hostname(host, IP.TYPE_IPV4)
    if not ip.is_valid_ip_address() or ip.contains(":"):
        return url
    return "%s://%s%s%s" % [url.substr(0, sep), ip, port, tail]


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
##
## 关闭前先 `poll` 两次把已排队的出站帧（如离开房间）写出，否则同帧
## `send` 后立即关闭会丢掉最后一帧，服务端按断线而不是主动离开处理。
func relogin() -> void:
    _want_connection = true
    _pending_reconnect = false
    if socket != null:
        if socket.get_ready_state() == WebSocketPeer.STATE_OPEN:
            socket.poll()
            socket.poll()
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
    var err: int = socket.connect_to_url(prefer_ipv4_url(url), tls)
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
