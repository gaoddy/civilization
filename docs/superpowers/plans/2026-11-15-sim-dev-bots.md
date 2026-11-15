# sim dev / make dev-bots 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 提供 `make dev-bots`，一条命令起服务端与 7 个机器人，机器人建房并打印房间码，开发者在桌面客户端填码开局。

**Architecture:** 机器人编排放在 `sim` 库里（`sim::bot::dev`），复用已有的 `BotConfig::room_code_sink` 进程内取码通路；CLI 只做参数解析，Bash 脚本只管服务端进程生命周期。不改协议、不改客户端、不改服务端。

**Tech Stack:** Rust 2024 / tokio / futures-util / tokio-tungstenite；Bash；GNU Make。

**Spec:** `docs/superpowers/specs/2026-11-15-sim-dev-bots-design.md`

## Global Constraints

- 固定 7 个机器人 + 开发者 1 人 = `sim::PLAYER_COUNT`（8），不提供 `--bots` 参数。
- 机器人英雄卡：`sim::embedded_catalog()` 中按 id 升序（`BTreeMap` 天然有序）取前 7 张 `is_hero` 卡。
- 每个机器人 `intent_seed = intent_seed + index`。
- 取房间码超时 10 秒；取到码后整局流程超时默认 900 秒（`--intent-seed` 之外不新增 CLI 参数）。
- 不复用已有服务端：`DEV_PORT`（默认 8080）被占用即失败退出。
- 服务端启动后必须用 `kill -0` 校验进程存活，与 `scripts/m1b_e2e.sh:51-55` 一致。
- 不改 `sim bot` 子命令，不新增 `BotConfig` 字段，`room-service/tests/e2e.rs` 保持原样。
- 所有构建与验证走 `make` 目标。

## 与设计的一处偏差（实施时按此执行）

设计列了「等 `GameStarted` 900s / 整局 300s」两段超时。区分这两段需要给 `BotConfig` 加进度回调，而那会改动 `room-service/tests/e2e.rs`，与「e2e.rs 保持原样」冲突。改为**取到房间码后一个总超时 `run_timeout`（默认 900s）覆盖开局与对局**，超时消息写明「开发者未在 900 秒内进房并准备，或对局未结束」。行为上仍然不会永久挂起，且不侵入既有 API。

---

## Review Focus

1. **开发者始终不进房**：900 秒后必须报错退出并释放端口，不能永久挂起。
2. **8080 已被其他进程占用**：立即失败并打印可执行的换端口命令，不能连到错误服务。
3. **Ctrl-C**：只杀脚本自己起的 `room-service`，不留孤儿进程。
4. **某个机器人提前失败**（认证失败、房间已解散）：取消其余机器人，报出是哪个 bot 失败。
5. **内嵌卡表英雄卡不足 7 张**：明确报错，不 panic、不越界索引。

---

## File Structure

| 文件 | 责任 |
| --- | --- |
| `server/crates/sim/src/bot/dev.rs`（新建） | 7 个机器人的编排：选英雄、建房取码、并发加入、超时与失败取消 |
| `server/crates/sim/src/bot/mod.rs`（修改） | 注册 `pub mod dev;` |
| `server/crates/sim/src/main.rs`（修改） | `dev` 子命令解析、USAGE、stdout 输出 |
| `server/crates/room-service/tests/dev_bots.rs`（新建） | 真服务端 + `run_dev` + 第 8 个「开发者」bot 的端到端测试 |
| `scripts/dev_bots.sh`（新建） | 端口占用检查、起服务端、就绪与存活校验、trap 清理 |
| `Makefile`（修改） | `dev-bots` 目标 |
| `docs/current/technical/rule-engine-and-simulator.md`（修改） | 登记 `sim dev` 与 `make dev-bots` |
| `docs/current/technical/client.md`（修改） | 运行方式补一条 `make dev-bots` |

---

### Task 1: `sim::bot::dev::run`

**Files:**
- Create: `server/crates/sim/src/bot/dev.rs`
- Modify: `server/crates/sim/src/bot/mod.rs:3-4`
- Test: `server/crates/room-service/tests/dev_bots.rs`

**Interfaces:**
- Consumes: `crate::bot::client::{BotConfig, run, BotOutcome}`、`crate::embedded_catalog()`、`crate::PLAYER_COUNT`。
- Produces:
  - `pub const DEV_BOTS: usize`（值等于 `usize::from(PLAYER_COUNT) - 1`，即 7）
  - `pub struct DevConfig { pub url: String, pub intent_seed: u64, pub room_code_sink: Option<tokio::sync::oneshot::Sender<String>>, pub run_timeout: std::time::Duration }`
  - `pub struct DevOutcome { pub room_code: String, pub turns: u8, pub ranking: Vec<(String, u8)> }`
  - `pub async fn run(config: DevConfig) -> Result<DevOutcome, String>`
  - `fn take_heroes(cards: &BTreeMap<String, Card>, needed: usize) -> Result<Vec<String>, String>`（私有，带单元测试）

- [ ] **Step 1: 写失败的端到端测试**

`server/crates/room-service/tests/dev_bots.rs`（新文件）。测试自带的 `start_server` / `connect_with_retry` / `wait_for_server` 从 `tests/e2e.rs:37-67` 抄最小版本，不要抽公共模块（e2e.rs 保持原样）。

```rust
//! `sim::bot::dev` 的端到端测试：7 个机器人 + 1 个「开发者」座位打完整局。

use std::sync::Arc;
use std::time::{Duration, Instant};

use room_service::clock::{Clock, SystemClock};
use sim::bot::client::{BotConfig, run as run_bot};
use sim::bot::dev::{DevConfig, DEV_BOTS, run as run_dev};
use tokio::sync::oneshot;

/// 任意一张英雄卡，给「开发者」座位用。
fn any_hero() -> String {
    let catalog = sim::embedded_catalog().expect("内嵌卡表应能加载");
    catalog
        .cards
        .values()
        .find(|card| card.is_hero)
        .map(|card| card.id.clone())
        .expect("卡表应有英雄卡")
}

async fn start_server() -> std::net::SocketAddr {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let (addr, router) = room_service::bind("127.0.0.1:0", clock)
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;
    addr
}

async fn wait_for_server(addr: std::net::SocketAddr) {
    let url = format!("ws://{addr}/ws");
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut delay = Duration::from_millis(10);
    loop {
        match tokio_tungstenite::connect_async(&url).await {
            Ok((ws, _)) => {
                drop(ws);
                return;
            }
            Err(error) => {
                assert!(Instant::now() < deadline, "2 秒内无法连接 {url}: {error}");
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_millis(50));
            }
        }
    }
}

#[test]
fn dev_bots_leaves_exactly_one_seat_for_the_developer() {
    assert_eq!(DEV_BOTS, usize::from(sim::PLAYER_COUNT) - 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dev_run_finishes_a_game_once_the_developer_joins() {
    let addr = start_server().await;
    let url = format!("ws://{addr}/ws");

    let (code_tx, code_rx) = oneshot::channel();
    let dev = tokio::spawn(run_dev(DevConfig {
        url: url.clone(),
        intent_seed: 11,
        room_code_sink: Some(code_tx),
        run_timeout: Duration::from_secs(60),
    }));

    let room_code = tokio::time::timeout(Duration::from_secs(10), code_rx)
        .await
        .expect("10 秒内应能拿到房间码")
        .expect("建房 bot 不应提前退出");
    assert_eq!(room_code.len(), 6, "房间码应为 6 位");

    // 「开发者」座位：第 8 人，进了房间才开始。
    let human = tokio::spawn(run_bot(BotConfig {
        url,
        room: room_code.clone(),
        name: "human".to_string(),
        hero: any_hero(),
        create: false,
        intent_seed: 99,
        room_code_sink: None,
    }));

    let outcome = dev.await.expect("dev 任务不应 panic").expect("dev 应打完一局");
    let human_outcome = human.await.expect("human 任务不应 panic").expect("human 应打完一局");

    assert_eq!(outcome.room_code, room_code);
    assert_eq!(outcome.ranking.len(), 8, "排名应为 8 条");
    assert_eq!(human_outcome.ranking.len(), 8);
    assert!(outcome.turns <= 20, "回合数 {} 超过 20", outcome.turns);
    assert_eq!(outcome.ranking, human_outcome.ranking, "两个座位应看到同一份排名");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dev_run_gives_up_when_nobody_joins() {
    let addr = start_server().await;
    let error = run_dev(DevConfig {
        url: format!("ws://{addr}/ws"),
        intent_seed: 1,
        room_code_sink: None,
        run_timeout: Duration::from_millis(300),
    })
    .await
    .expect_err("无人进房时应超时报错");
    assert!(error.contains("未完成开局"), "错误消息应说明超时原因，实际：{error}");
}
```

同一任务里，给 `take_heroes` 加一个不依赖卡牌数据的单元测试（写在 `dev.rs` 末尾的 `#[cfg(test)] mod tests`）：

```rust
#[test]
fn take_heroes_fails_when_catalog_has_too_few_heroes() {
    let error = super::take_heroes(&std::collections::BTreeMap::new(), 7)
        .expect_err("空卡表应报错");
    assert!(error.contains("至少需要 7 张"), "实际：{error}");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p room-service --test dev_bots -v`
Expected: 编译失败，`unresolved import sim::bot::dev`。

- [ ] **Step 3: 实现 `server/crates/sim/src/bot/dev.rs`**

模块文档一行：`//! `sim dev`：起 7 个机器人建房并打完一局，给桌面客户端留出第 8 个座位。`

实现要点（签名由上，逻辑按此）：

- `hero_cards`：`catalog.cards.values().filter(|c| c.is_hero).take(DEV_BOTS).map(|c| c.id.clone()).collect()`；数量不足 `DEV_BOTS` 时返回 `Err(format!("内嵌卡表只有 {n} 张英雄卡，至少需要 {DEV_BOTS} 张"))`。`BTreeMap::values()` 已按 id 升序，不要额外排序。签名 `fn take_heroes(cards: &BTreeMap<String, Card>, needed: usize) -> Result<Vec<String>, String>`，`run` 里传 `&catalog.cards`，测试可传空表。
- `run` 的流程：
  1. 取 `embedded_catalog()` 与英雄卡。
  2. 建 `oneshot::channel()`，`tokio::spawn(client::run(BotConfig { create: true, room: String::new(), name: "bot0", hero: heroes[0], intent_seed, room_code_sink: Some(tx) }))`。
  3. `tokio::time::timeout(Duration::from_secs(10), rx)` 取码；超时返回 `Err("10 秒内未取得房间码：服务端可能未启动")`。`rx` 返回 `Err` 说明建房机器人已退出，返回 `Err("建房机器人提前退出，未取得房间码")`。
  4. 把 `Some(tx)` 已消费的情况下，通过 `config.room_code_sink.take()` 把房间码转发给调用方。
  5. 为 `index in 1..DEV_BOTS` 各 spawn 一个 `create: false, room: code.clone(), name: format!("bot{index}"), hero: heroes[index].clone(), intent_seed: config.intent_seed + index as u64`。
  6. 所有 handle 放进 `Vec<JoinHandle<Result<BotOutcome, String>>>`。用 `tokio::time::timeout(config.run_timeout, join_all(&mut handles))` 收集；`&mut JoinHandle` 实现了 `Future`（`JoinHandle: Unpin`），因此超时后仍能遍历 `handles` 调 `.abort()`。
  7. 超时：abort 全部 handle，返回 `Err(format!("未完成开局：开发者未进房并准备，或对局未结束（{} 秒）", config.run_timeout.as_secs()))`。消息里的秒数来自 `run_timeout`，不要写死 900——测试用 300ms 验证这条路径。
  8. 任一 handle 返回 `JoinError` 或 `Err`：abort 其余，返回 `Err(format!("{name} 失败: {err}"))`，名字用 `bot{index}`；多个失败只报第一个。
  9. 全部成功：返回 `DevOutcome { room_code, turns: outcomes[0].turns, ranking: outcomes[0].ranking }`（`outcomes[0]` 是建房机器人）。

（Step 1 的超时断言最终写成 `assert!(error.contains("未完成开局"))`，不要断言 `900`。）

- [ ] **Step 4: 注册模块**

`server/crates/sim/src/bot/mod.rs` 在 `pub mod client;` 后加 `pub mod dev;`。

- [ ] **Step 5: 运行测试确认通过**

Run: `cd server && cargo test -p room-service --test dev_bots -v`
Expected: 3 个用例全部 `ok`。

- [ ] **Step 6: 提交**

```bash
git add server/crates/sim/src/bot/dev.rs server/crates/sim/src/bot/mod.rs server/crates/room-service/tests/dev_bots.rs
git commit -m "feat(sim): 新增 dev 编排，7 个机器人建房并等开发者入座"
```

---

### Task 2: `sim dev` 子命令

**Files:**
- Modify: `server/crates/sim/src/main.rs:13-20`（USAGE）、`:33-123`（`run` 的 match）、新增 `run_dev` 函数
- Test: 手动执行（CLI 参数解析已由现有 `parse_options` 覆盖）

**Interfaces:**
- Consumes: Task 1 的 `sim::bot::dev::{DevConfig, run}`。
- Produces: 命令 `sim dev --connect ws://HOST:PORT/ws [--intent-seed N]`，stdout 首行 `ROOM_CODE=<code>`。

- [ ] **Step 1: 补 USAGE**

在 `USAGE` 的 `sim bot` 行后追加：

```
  sim dev    --connect ws://HOST:PORT/ws [--intent-seed N]
             起 7 个机器人建房并加入，打印房间码后等第 8 人（开发者）
```

同时把文件头 `//!` 首行的子命令清单从「五个子命令」改为「`play`、`batch`、`replay`、`report`、`check`、`bot`、`dev`」。

- [ ] **Step 2: 加 match 分支**

在 `"bot" => run_bot(rest),` 之后加：

```rust
"dev" => run_dev(rest),
```

- [ ] **Step 3: 实现 `fn run_dev(args: &[String]) -> Result<(), String>`**

- 解析：`parse_options(args, &["--connect", "--intent-seed"])`；`--connect` 必填，`--intent-seed` 缺省 0（照 `run_bot` 的写法）。
- 建 multi-thread tokio runtime，与 `run_bot` 同样方式（`Builder::new_multi_thread().enable_all().build()`）。
- `runtime.block_on(sim::bot::dev::run(DevConfig { url, intent_seed, room_code_sink: None, run_timeout: Duration::from_secs(900) }))`；`room_code_sink: None` 是因为 CLI 直接打印，不需要通道。
- 但 `run` 的返回要等整局结束。房间码需要在**开局前**就打出来，所以 CLI 必须自己拿通道：建 `oneshot::channel()`，把 `Some(tx)` 传进 `DevConfig`，在 `block_on` 之前用 `runtime.spawn` 起 `run`，先 `block_on(rx)` 打印两行，再 `block_on(handle)` 等结果。

```rust
let (code_tx, code_rx) = tokio::sync::oneshot::channel();
let handle = runtime.spawn(sim::bot::dev::run(DevConfig {
    url,
    intent_seed,
    room_code_sink: Some(code_tx),
    run_timeout: std::time::Duration::from_secs(900),
}));
let code = runtime
    .block_on(code_rx)
    .map_err(|_| "建房机器人提前退出，未取得房间码".to_string())?;
println!("ROOM_CODE={code}");
println!("在客户端大厅输入房间码 {code}，选英雄并准备后自动开局。");
let outcome = runtime
    .block_on(handle)
    .map_err(|err| format!("dev 任务 panic: {err}"))??;
println!(
    "DEV_DONE turns={} ranking={}",
    outcome.turns,
    outcome.ranking.len()
);
Ok(())
```

- [ ] **Step 4: 编译与空跑校验**

Run: `cd server && cargo build -q -p sim && ./target/debug/sim dev`
Expected: 退出码非 0，stderr 输出 `sim: 缺少参数 --connect`。

Run: `cd server && ./target/debug/sim dev --connect ws://127.0.0.1:1/ws`
Expected: 退出码非 0，stderr 输出连接失败。

- [ ] **Step 5: 提交**

```bash
git add server/crates/sim/src/main.rs
git commit -m "feat(sim): 新增 dev 子命令，打印房间码并等待开发者入座"
```

---

### Task 3: `scripts/dev_bots.sh` 与 `make dev-bots`

**Files:**
- Create: `scripts/dev_bots.sh`
- Modify: `Makefile:4`（`.PHONY`）、`:72-78` 附近
- Test: 脚本自身的手动验证 + 端口占用用例

**Interfaces:**
- Consumes: `server/target/debug/room-service`、`server/target/debug/sim`（由 `m1b-build` 产出）。
- Produces: `make dev-bots`（`DEV_PORT` 可覆盖，默认 8080）。

- [ ] **Step 1: 写脚本**

`scripts/dev_bots.sh`，`set -euo pipefail`，结构对齐 `scripts/m1b_e2e.sh`：

- `PORT=${DEV_PORT:-8080}`，`URL="ws://127.0.0.1:${PORT}/ws"`。
- 端口占用检查：若 `(exec 3<>"/dev/tcp/127.0.0.1/${PORT}") 2>/dev/null` 成功，关闭 fd 后打印
  `端口 ${PORT} 已被占用，用 make dev-bots DEV_PORT=8081 换端口` 并 `exit 1`。
- 起服务端：`"$ROOT/server/target/debug/room-service" --listen "127.0.0.1:${PORT}" &`，记 `SERVER_PID`。
- `trap` 在 `EXIT`、`INT`、`TERM` 上 `kill "$SERVER_PID"`，然后 `wait`。
- 就绪轮询：与 `m1b_e2e.sh:41-49` 同款循环（100 次 × 0.1s 试连端口）。
- 就绪后立刻 `kill -0 "$SERVER_PID"`，失败则 `tail -n 20` 服务端日志并 `exit 1`。
- 前台执行 `"$ROOT/server/target/debug/sim" dev --connect "$URL" "$@"`，退出码原样透传（`set -e` 会带着 `trap` 一起退出）。
- 服务端日志写到 `output/dev-bots/server.log`（`output/` 已被 gitignore），目录不存在先 `mkdir -p`。

- [ ] **Step 2: 验证端口占用分支**

Run: `python3 -m http.server 8080`（后台）后 `make dev-bots`
Expected: 立刻打印 `端口 8080 已被占用，用 make dev-bots DEV_PORT=8081 换端口`，退出码非 0。杀掉临时进程。

- [ ] **Step 3: 加 Makefile 目标**

在 `.PHONY` 行追加 `dev-bots`，并在 `m1b-e2e` 之后加：

```make
# 本地联调：起服务端与 7 个机器人，打印房间码后等开发者在大厅加入。
DEV_PORT ?= 8080

dev-bots: m1b-build
	DEV_PORT=$(DEV_PORT) bash scripts/dev_bots.sh
```

- [ ] **Step 4: 端到端人工验证**

Run: `make dev-bots`
Expected: 终端出现 `ROOM_CODE=<6 位>` 与提示行。

另开终端起客户端并进房：

Run: `/Applications/Godot.app/Contents/MacOS/Godot --path client`
在大厅输入房间码、选英雄、准备。Expected: 8 人齐自动开局，客户端可正常操作；对局结束后 `make dev-bots` 打印 `DEV_DONE turns=<n> ranking=8` 并退出码 0，端口释放（`lsof -i :8080` 为空）。

- [ ] **Step 5: 验证 Ctrl-C**

Run: `make dev-bots`，看到房间码后按 Ctrl-C。
Expected: 命令退出；`lsof -i :8080` 为空；`pgrep -f room-service` 与 `pgrep -f "sim dev"` 均无输出。

- [ ] **Step 6: 提交**

```bash
git add scripts/dev_bots.sh Makefile
git commit -m "feat(dev): 新增 make dev-bots，一键起服务端与 7 个机器人"
```

---

### Task 4: 文档回写

**Files:**
- Modify: `docs/current/technical/rule-engine-and-simulator.md:92`（命令行清单）、`:80-91`（`make` 目标表）
- Modify: `docs/current/technical/client.md:63-91`（§4 命令表、§5 运行方式）

- [ ] **Step 1: 更新模拟器文档**

`rule-engine-and-simulator.md` 第 92 行的 `sim` 命令清单末尾追加 `sim dev --connect ws://HOST:PORT/ws [--intent-seed N]`，并加一句：起 7 个机器人建房，打印 `ROOM_CODE=<code>` 后等开发者作为第 8 人入座；取码 10 秒超时，整局 900 秒超时。

`make` 目标表加一行：`make dev-bots | 本地联调：起服务端与 7 个机器人，打印房间码；`DEV_PORT` 默认 8080`。

- [ ] **Step 2: 更新客户端文档**

`client.md` §4 命令表加 `make dev-bots` 行（同上）；§5 运行方式在三条终端说明之前加一段「省事用法」：`make dev-bots` 会起服务端与 7 个机器人并打印房间码，开发者只需开客户端、在大厅填码；Ctrl-C 一次全部退出。保留原有的手动三终端写法，标注它用于需要自定义机器人参数时。

- [ ] **Step 3: 校验文档**

Run: `make data-check && python3 scripts/validate_docs.py`
Expected: 两条命令退出码 0。

- [ ] **Step 4: 提交**

```bash
git add docs/current/technical/rule-engine-and-simulator.md docs/current/technical/client.md
git commit -m "docs(dev): 回写 sim dev 与 make dev-bots 用法"
```

---

### Task 5: 全量验证

- [ ] **Step 1: 静态检查与测试**

Run: `make lint && make test`
Expected: 两条退出码 0，`clippy` 无警告。

- [ ] **Step 2: 确认既有验收未受影响**

Run: `make m1b-e2e`
Expected: 输出 `M1B_E2E_OK place=... ranking=8 ...`，退出码 0。

- [ ] **Step 3: 客户端验收未受影响**

Run: `make client-check && make client-smoke`
Expected: `client-smoke.log` 含 `UI_SMOKE_OK` 且无 `SCRIPT ERROR`。

- [ ] **Step 4: 提交（如有残留改动）**

`git status --porcelain` 应为空；若有生成物漂移，按仓库约定一并提交。
