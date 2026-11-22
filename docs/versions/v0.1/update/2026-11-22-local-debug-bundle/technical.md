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

1. `checking`：用 `StreamPeerTCP` 探测 `127.0.0.1:8080`；连接成功时不启动服务端（视为复用已有开发服务端），探测停在 `CONNECTING` 超过 3 秒进入 `failed("server_check_timeout")`，其余情况进入 `starting_server`。探测只到 TCP 连接，不做 WebSocket 握手。
2. `starting_server`：`OS.execute_with_pipe(room-service, ["--listen", "127.0.0.1:8080"], false)`，保存 pid 与 stdio。
3. `waiting_server`：每帧轮询 TCP 连接，10 秒内未就绪进入 `failed`；就绪后 `server_ready` 信号发出。
4. `starting_bots`：`OS.execute_with_pipe(sim, ["dev", "--connect", "ws://127.0.0.1:8080/ws", "--intent-seed", "0"], false)`，保存 pid 与 stdio。
5. `waiting_room_code`：每帧从 sim 的 stdio 读取行，匹配 `ROOM_CODE=<CODE>`；15 秒内未读到进入 `failed`；读到后保存 `room_code` 并发出 `room_code_ready(code)`。
6. `ready`：登录与入房流程使用 `room_code`。

- server 与 sim 的 stdout/stderr 写入 `user://local_debug.log`，便于失败排查。

### 3.3 进程回收

- `_exit_tree` 中 `OS.kill` 本进程启动的 server 与 sim；复用已有服务端时不 kill。
- 崩溃残留：下次启动探测到 8080 已监听则复用；旧 `sim` 进程仍可能存活并占用房间，属于已知限制，不自动清理（不误杀非本进程）。

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
- 端到端人工验收（GUI 步骤尚未执行）：调试产物已由 `make client-export-local` 导出成功；以下 GUI 步骤尚未执行：打开产物 → 自动登录进大厅 → 自动进入机器人房间 → 选英雄并准备 → 自动开局；退出后 8080 不再由本产物启动的服务端监听（复用场景除外）。上述 GUI 端到端尚未在真实产物中验收，由主 agent 在 8080 空闲时执行；当前只完成构建级与静态校验。
- 发行回归：`make client-export` 已运行，产物 `Contents/MacOS/` 不含 `room-service`、`sim`；`client-export-steam` 未运行，按预设 `custom_features=""` 静态核对未启用 `embedded_server`，未实际导出验证。

## 5. 风险与约束

- 产物未签名，只保证本机运行；跨机器分发可能被 Gatekeeper 拦截。
- sidecar 为宿主架构（arm64）；Intel Mac 需要另行构建 universal。
- 8080 被非本服务占用时不会启动服务端，登录会走现有错误提示。
- 崩溃残留的 sim 进程不自动清理，避免误杀其他进程。
