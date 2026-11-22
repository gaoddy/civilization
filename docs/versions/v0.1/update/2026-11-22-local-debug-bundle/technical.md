# 单机调试包（内嵌 room-service 与 sim）技术方案

> 状态：已实施
> 需求归属：v0.1
> 变更性质：纯技术改动，只影响本地调试产物，不改变产品规则与发行链路

## 1. 改动边界

- 包含：`Makefile` 新增调试导出目标、`client/export_presets.cfg` 新增预设、客户端新增 `LocalServer` autoload、登录页与大厅的本地自动流程、`client/project.godot` autoload 注册、`docs/current/technical/client.md`。
- 不包含：发行导出目标（`client-export`、`client-export-steam`、`client-export-ios`）、room-service 与 sim 源码、协议、数据库、规则引擎。
- 生效开关：导出预设的 `custom_features="embedded_server"`。编辑器运行、冒烟、发行包都没有该特性，`LocalServer` 不启动任何进程。

## 2. 产物与构建

### 2.1 导出预设

- `client/export_presets.cfg` 新增 `macOS Local Debug`（`preset.3`）：
  - `platform="macOS"`、`custom_features="embedded_server"`、`export_filter="all_resources"`、`script_export_mode=2`。
  - `export_path="../output/desktop-local/Civilization.app"`。
  - `binary_format/architecture="universal"`、`binary_format/embed_pck=true`。
  - `application/bundle_identifier="com.civilization.banner"`、`application/name="Civilization"`、`codesign/codesign=0`，与现有 macOS 预设一致。
- 预设只用于调试导出，不参与 Steam 与 iOS 产物。

### 2.2 Makefile 目标

新增 `client-export-local`：

- 依赖 `client-export-templates`、`client-import`、`m1b-build`。
- 仅 macOS 可用，非 Darwin 直接失败。
- 清空 `output/desktop-local/` 后执行 `"$(GODOT)" --headless --path client --export-debug "macOS Local Debug" "$(CURDIR)/output/desktop-local/Civilization.app"`。
- 校验 `Contents/MacOS/Civilization` 可执行。
- 复制 `server/target/debug/room-service` 与 `server/target/debug/sim` 到 `Civilization.app/Contents/MacOS/`，`chmod +x`，并校验两份可执行。
- 不做 zip、不签名：产物只用于本机调试。

### 2.3 体积

- room-service debug 31M、sim debug 13M；debug 导出本身已含调试符号（本机产物约 340M），服务端二进制的增量可接受。
- 使用 debug 构建以保留 panic 信息与符号。

## 3. 客户端组件 LocalServer

### 3.1 注册与开关

- `client/autoload/local_server.gd`，`client/project.godot` 的 `[autoload]` 追加 `LocalServer="*res://autoload/local_server.gd"`。
- `enabled` 仅在 `OS.has_feature("embedded_server")` 为真时为真；带特性启动后若 `sidecar_paths(base_dir)` 找不到两份 sidecar，进入 `failed("sidecar_missing")`，不启动任何进程。
- `auto_flow` 取 `OS.get_environment("CIVILIZATION_LOCAL_AUTO") != "0"`；为 `0` 时只关闭自动登录与自动入房，仍启动服务端与机器人，便于手测登录/大厅流程。

### 3.2 启动状态机

`idle → checking → starting_server → waiting_server → starting_bots → waiting_room_code → ready`；任一步失败进入 `failed`，记录原因，不阻塞页面其他手动操作。

1. `checking`：用 `HTTPRequest` GET `http://127.0.0.1:8080/healthz`（超时 3 秒）；仅 `RESULT_SUCCESS` 且响应码 200 视为已有可复用服务端，直接复用并跳过启动；连接失败、超时、非 200 及其他结果都进入 `starting_server`。Godot 4.7.2 的 `StreamPeerTCP` 对已关闭端口会短暂误报 `CONNECTED`，因此不再用它判定复用。
2. `starting_server`：`OS.execute_with_pipe(room-service, ["--listen", "127.0.0.1:8080"], false)`，保存 pid、stdio 与 stderr。
3. `waiting_server`：每 0.25 秒重复同一健康检查；服务端进程退出时立即 `failed("server_exited")`，10 秒内未就绪进入 `failed("server_timeout")`；就绪后发出 `server_ready` 信号。
4. `starting_bots`：`OS.execute_with_pipe(sim, ["dev", "--connect", "ws://127.0.0.1:8080/ws", "--intent-seed", "0"], false)`，保存 pid 与 stdio。
5. `waiting_room_code`：每帧从 sim 的 stdio 读取行，匹配 `ROOM_CODE=<CODE>`；15 秒内未读到进入 `failed`；读到后保存 `room_code` 并发出 `room_code_ready(code)`。
6. `ready`：登录与入房流程使用 `room_code`。

- server 与 sim 的 stdout/stderr 写入 `user://local_debug.log`，便于失败排查。

### 3.3 进程回收

- `_exit_tree` 中 `OS.kill` 本进程启动的 server 与 sim；复用已有服务端时不 kill。
- 崩溃残留：下次启动 `/healthz` 返回 200 则复用；旧 `sim` 进程仍可能存活并占用房间，属于已知限制，不自动清理（不误杀非本进程）。

### 3.4 自动登录与自动入房

- 登录页：`LocalServer.enabled` 且自动流程开启时，等待 `server_ready`（已就绪则立即），用现有预填的开发账号自动提交登录；等待期间状态标签复用 `ui.login.connecting`（显示默认服务地址），不新增文案 key。
- 大厅：拿到 `room_code` 后填入房间码输入框并自动 `JoinRoom`；先到大厅后到码时订阅 `room_code_ready` 补发，房间码只消费一次。
- `LocalServer.failed` 时页面状态标签显示失败原因，登录按钮保持可用，允许连其他服务端。

### 3.5 纯函数

- `room_code_from_line(line: String) -> String`：解析 `ROOM_CODE=` 行，非匹配行返回空串。
- `sidecar_paths(base_dir: String) -> Dictionary`：返回 `server` / `sim` 路径与存在性。
- 两个函数不依赖引擎状态，供 headless 冒烟直接断言。

## 4. 测试与验收

- `client/tests/ui_smoke_local_server.gd/.tscn`：断言 `room_code_from_line` 的合法行、前后空白、空行、带前缀噪声、`DEV_DONE` 行；断言 `sidecar_paths` 在不存在目录时返回缺失与两份路径；断言无 `embedded_server` 特性时 `enabled` 为假。不启动任何进程。
- `make client-check`；`make client-smoke SMOKE=local_server` 只跑新增用例。
- 端到端验收（2026-11-22，8080 空闲，`44da067` 构建）：
  - `make client-export-local` 退出 0，产物写入 `output/desktop-local/Civilization.app`。
  - 启动产物后日志新增 `server ready on 127.0.0.1:8080` 与 `ROOM_CODE=222222`，无 `reuse server`。
  - 自动登录：客户端进程与 `127.0.0.1:8080` 建立 ESTABLISHED 连接，服务端同时持有 8 条连接（`sim` 7 条 + 客户端 1 条）。
  - 自动入房：以 `sim bot --room 222222` 探测返回 `ROOM_FULL 房间已满`，说明 8 个席位已满（7 个机器人 + 客户端）。
  - 正常退出：`osascript -e 'quit app "Civilization"'` 后无 `room-service` / `sim` 残留，8080 释放。
  - 未执行：界面内选英雄、准备并完成一局。验收时本机锁屏无法输入，用户已确认以连接级证据（服务端 8 条连接、`ROOM_FULL`）通过验收并跳过该步。
- 发行回归：`make client-export` 已运行，产物 `Contents/MacOS/` 不含 `room-service`、`sim`；`client-export-steam` 未运行，按预设 `custom_features=""` 静态核对未启用 `embedded_server`，未实际导出验证。

## 5. 风险与约束

- 产物未签名，只保证本机运行；跨机器分发可能被 Gatekeeper 拦截。
- sidecar 为宿主架构（arm64）；Intel Mac 需要另行构建 universal。
- 8080 被非本服务占用时，`/healthz` 非 200，会尝试启动服务端并因绑定失败进入 `server_exited` / `server_timeout`；登录页显示该失败原因，仍可手动连其他服务端。
- 崩溃残留的 sim 进程不自动清理，避免误杀其他进程。
