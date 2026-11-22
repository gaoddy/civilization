extends Node
## 本地调试服务：仅在带 `embedded_server` 特性的调试导出里启动 room-service 与 sim。
##
## 编辑器、冒烟与发行包都没有该特性，`enabled` 保持 false，不产生任何进程。
## 启动流程与失败原因见 technical.md §3.2–3.3。

signal server_ready
signal room_code_ready(code: String)
signal failed(reason: String)

const SERVER_URL := "ws://127.0.0.1:8080/ws"
const SERVER_PORT := 8080
const SERVER_WAIT_SECONDS := 10.0
const ROOM_CODE_WAIT_SECONDS := 15.0

const ROOM_CODE_PREFIX := "ROOM_CODE="
const LISTEN_ADDRESS := "127.0.0.1:8080"
const LOG_PATH := "user://local_debug.log"

enum State {
    IDLE,
    CHECKING,
    STARTING_SERVER,
    WAITING_SERVER,
    STARTING_BOTS,
    WAITING_ROOM_CODE,
    READY,
    FAILED,
}

var enabled := false
var auto_flow := true
var room_code := ""

var _state := State.IDLE
var _server_path := ""
var _sim_path := ""
var _server_pid := -1
var _sim_pid := -1
var _server_stdout: FileAccess = null
var _sim_stdout: FileAccess = null
var _log: FileAccess = null
var _probe: StreamPeerTCP = null
var _wait_elapsed := 0.0
var _server_ready_emitted := false
var _room_code_emitted := false


func _ready() -> void:
    enabled = OS.has_feature("embedded_server")
    auto_flow = OS.get_environment("CIVILIZATION_LOCAL_AUTO") != "0"
    if not enabled:
        return
    _open_log()
    var base_dir := OS.get_executable_path().get_base_dir()
    var paths := sidecar_paths(base_dir)
    if not paths["server_exists"] or not paths["sim_exists"]:
        _fail("sidecar_missing")
        return
    _server_path = str(paths["server"])
    _sim_path = str(paths["sim"])
    _state = State.CHECKING


func _process(delta: float) -> void:
    if not enabled:
        return
    match _state:
        State.CHECKING:
            _poll_checking()
        State.WAITING_SERVER:
            _drain_pipes()
            _wait_elapsed += delta
            _poll_waiting_server()
        State.WAITING_ROOM_CODE:
            _drain_pipes()
            if _state != State.WAITING_ROOM_CODE:
                return
            _wait_elapsed += delta
            _poll_waiting_room_code()
        State.READY:
            # 继续排空管道，避免子进程写满缓冲后阻塞。
            _drain_pipes()
        State.FAILED:
            # 失败后服务端可能仍存活，保持排空同样是为了防止写满阻塞。
            _drain_pipes()


func _exit_tree() -> void:
    _close_probe()
    # 只回收本进程启动的子进程；复用已有服务端时 pid 保持 -1。
    if _server_pid > 0:
        OS.kill(_server_pid)
        _server_pid = -1
    if _sim_pid > 0:
        OS.kill(_sim_pid)
        _sim_pid = -1
    if _log != null:
        _log.flush()
        _log = null


func is_server_ready() -> bool:
    return _server_ready_emitted


func consume_room_code() -> String:
    var code := room_code
    room_code = ""
    return code


static func room_code_from_line(line: String) -> String:
    var trimmed := line.strip_edges()
    if not trimmed.begins_with(ROOM_CODE_PREFIX):
        return ""
    return trimmed.trim_prefix(ROOM_CODE_PREFIX).strip_edges()


static func sidecar_paths(base_dir: String) -> Dictionary:
    var server := base_dir.path_join("room-service")
    var sim := base_dir.path_join("sim")
    return {
        "server": server,
        "sim": sim,
        "server_exists": FileAccess.file_exists(server),
        "sim_exists": FileAccess.file_exists(sim),
    }


func _poll_checking() -> void:
    var status := _probe_status()
    if status == StreamPeerTCP.STATUS_CONNECTED:
        _close_probe()
        _log_line("reuse server on " + LISTEN_ADDRESS)
        _finish_server_wait()
        return
    if status == StreamPeerTCP.STATUS_CONNECTING:
        return
    _close_probe()
    _start_server()


func _poll_waiting_server() -> void:
    var status := _probe_status()
    if status == StreamPeerTCP.STATUS_CONNECTING:
        return
    if status == StreamPeerTCP.STATUS_CONNECTED:
        _close_probe()
        _log_line("server ready on " + LISTEN_ADDRESS)
        _finish_server_wait()
        return
    _close_probe()
    if _server_pid > 0 and not OS.is_process_running(_server_pid):
        _fail("server_exited")
        return
    if _wait_elapsed >= SERVER_WAIT_SECONDS:
        _fail("server_timeout")


func _poll_waiting_room_code() -> void:
    if not OS.is_process_running(_sim_pid):
        _fail("bots_exited")
        return
    if _wait_elapsed >= ROOM_CODE_WAIT_SECONDS:
        _fail("room_code_timeout")


func _start_server() -> void:
    _state = State.STARTING_SERVER
    var result := _spawn(_server_path, PackedStringArray(["--listen", LISTEN_ADDRESS]))
    _server_pid = int(result.get("pid", -1))
    if _server_pid <= 0:
        _fail("server_spawn_failed")
        return
    _server_stdout = result.get("stdio") as FileAccess
    _wait_elapsed = 0.0
    _state = State.WAITING_SERVER


func _start_bots() -> void:
    _state = State.STARTING_BOTS
    var result := _spawn(_sim_path, PackedStringArray([
        "dev", "--connect", SERVER_URL, "--intent-seed", "0",
    ]))
    _sim_pid = int(result.get("pid", -1))
    if _sim_pid <= 0:
        _fail("bots_spawn_failed")
        return
    _sim_stdout = result.get("stdio") as FileAccess
    _wait_elapsed = 0.0
    _state = State.WAITING_ROOM_CODE


func _finish_server_wait() -> void:
    _wait_elapsed = 0.0
    if not _server_ready_emitted:
        _server_ready_emitted = true
        server_ready.emit()
    _start_bots()


func _open_log() -> void:
    # 保留历史日志，便于排查端口占用与残留进程。
    if FileAccess.file_exists(LOG_PATH):
        _log = FileAccess.open(LOG_PATH, FileAccess.READ_WRITE)
        if _log != null:
            _log.seek_end()
            return
    _log = FileAccess.open(LOG_PATH, FileAccess.WRITE)


func _spawn(path: String, args: PackedStringArray) -> Dictionary:
    # 4.7.2 的 execute_with_pipe 只重定向 stdout；stderr 跟随启动本进程的终端。
    return OS.execute_with_pipe(path, args, false)


func _probe_status() -> int:
    if _probe == null:
        _probe = StreamPeerTCP.new()
        var error := _probe.connect_to_host("127.0.0.1", SERVER_PORT)
        if error != OK:
            return StreamPeerTCP.STATUS_ERROR
    _probe.poll()
    return _probe.get_status()


func _close_probe() -> void:
    if _probe == null:
        return
    if _probe.get_status() == StreamPeerTCP.STATUS_CONNECTED:
        _probe.disconnect_from_host()
    _probe = null


func _drain_pipes() -> void:
    _drain_stream(_server_stdout)
    _drain_sim()
    if _log != null:
        _log.flush()


func _drain_stream(stream: FileAccess) -> void:
    if stream == null:
        return
    while true:
        var line := stream.get_line()
        if line == "":
            return
        _log_line(line)


func _drain_sim() -> void:
    if _sim_stdout == null:
        return
    while true:
        var line := _sim_stdout.get_line()
        if line == "":
            return
        _log_line(line)
        var code := room_code_from_line(line)
        if code != "" and not _room_code_emitted:
            _room_code_emitted = true
            room_code = code
            _state = State.READY
            room_code_ready.emit(code)


func _log_line(line: String) -> void:
    if _log == null:
        return
    _log.store_line(line)


func _fail(reason: String) -> void:
    if _state == State.FAILED:
        return
    _state = State.FAILED
    _log_line("failed: " + reason)
    # autoload `_ready` 早于主场景，延迟到场景订阅信号之后再发。
    failed.emit.call_deferred(reason)
