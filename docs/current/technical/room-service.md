# 房间服务与联网

> 状态：已实现（v0.1 M1a）
> 代码：`server/crates/room-service`、`server/crates/protocol`、`server/crates/protocol-gen`、`protocol/schema/v1`、`protocol/generated`
> 验收：`make m1-check`

本文记录 v0.1 M1a 已实现的房间服务现状：crate 边界、WebSocket 接入、下行序号、去重、逐座位过滤、计时与断线处理、`sim bot` 用法与验收方式；不描述 M1b 的 Godot 客户端，也不描述产品规则（规则见[规则引擎与模拟器](rule-engine-and-simulator.md)）。

## 1. 工程结构

```
server/
  Cargo.toml                    # workspace：crates/{protocol, protocol-gen, rule-engine, room-service, sim}
  crates/
    protocol/                   # serde 类型：由 schema 生成，含客户端/服务端消息与视图
    protocol-gen/               # 生成器：schema bundle → protocol/generated/protocol.rs
    rule-engine/                # 纯库：规则判定与逐座位视图（src/view.rs）
    room-service/               # 可执行：WebSocket 接入、房间、对局驱动
      src/lib.rs                # AppState、路由装配、内嵌 v0.1 数据
      src/main.rs               # --listen（默认 127.0.0.1:8080）
      src/net.rs                # /ws 连接 task、写端 task、房间 task（单写者）
      src/room.rs               # 大厅房间状态机与房间注册表
      src/game.rs               # 对局驱动：计时、阶段推进、去重、逐座位视图
      src/filter.rs             # 逐座位事件过滤与 CombatResult 映射
      src/view.rs               # 引擎 SeatView → 协议 PlayerView 的纯函数映射
      src/envelope.rs           # 单连接下行信封序号 SeatChannel
      src/auth.rs               # 手机号+开发验证码 / 令牌续登
      src/clock.rs              # Clock 注入（SystemClock / FixedClock）
      src/error.rs              # 稳定错误码 → ErrorMessage
      tests/                    # auth / room / game / filter / reconnect / e2e
    sim/
      src/bot/brain.rs          # 只读 PlayerView 的确定性机器人决策
      src/bot/client.rs         # `sim bot --connect` 的真实 WebSocket 客户端
      src/bot/mod.rs
      src/ai.rs                 # M0 确定性 AI（为保基线未改动）
      src/lib.rs                # run_games / play_game / replay；重导出 seed_from_u64
      src/main.rs               # play / batch / replay / report / check / bot
protocol/
  schema/v1/                    # 唯一契约来源（JSON Schema draft 2020-12）
  generated/protocol.rs         # typify 生成物，随源提交
  generated/protocol.gd         # GDScript 常量生成物，随源提交
```

## 2. 依赖方向与边界

- `room-service → protocol`、`room-service → rule-engine`；`room-service` 的 dev-dependencies 含 `sim`，供 e2e 用真实机器人驱动服务端。
- `sim → protocol`、`sim → rule-engine`；`sim` 不依赖 `room-service`，因此 `bot/mod.rs` 重复声明开发验证码 `DEV_SMS_CODE = "123456"`。
- `rule-engine` 不依赖 `protocol`、tokio、sqlx 或任何 I/O 库；它不读时钟、不做文件 I/O，并提供自有的逐座位视图 `SeatView`（`src/view.rs`）。
- 网络、计时与过滤全部在 `room-service`；规则判定全部在 `rule-engine`，由编译期依赖图保证引擎无副作用。

## 3. WebSocket 接入与信封

- 路由只有 `GET /ws`（`axum::routing::get`），升级后进入连接 task；`AppState` 内嵌 v0.1 的 `cards.json`、`skills.json`、`tokens.json`、`config.json`。
- 连接拓扑：每个连接一个读端 task 与一个写端 task；每个房间一个房间 task（单写者，独占 `Room`/`GameTask` 可变状态）。连接层从不直接改房间状态，只投递 `RoomMessage`。
- 上行统一为 `ClientEnvelope { v, id, type, ts, payload }`。**生成类型不校验 `v`**：连接层显式拒绝 `v != 1`，回 `BAD_PAYLOAD`（`不支持的协议版本`）后关闭连接，避免按 v1 语义执行 v2 载荷。
- `envelope.parse()` 按 `type` 判别并校验 `payload` 结构；未知 `type` 回 `UNKNOWN_TYPE`，载荷不匹配回 `BAD_PAYLOAD`（`上行载荷与类型不匹配`），两者都继续读下一条，不关闭连接。
- 未入房时，连接层错误走连接自己的 `SeatChannel`；入房后一律经房间 task 走该座位的统一下行通道，保证序号跨来源单调。

## 4. 下行序号

- 每个玩家连接持有独立的 `SeatChannel`，`seq` 从 1 起递增；失败的构造同样消耗序号，不回退。
- 同一玩家有三条下行来源：连接层 `direct`、大厅 `slot.channel`、对局 `GameTask` 通道。三者共享一个会话级 `SeqFloor`（`Arc<AtomicU64>` 下限）：编码前先把通道推进到当前下限，发送后再把新 `seq` 记回下限，因此**同一连接的任何下行（含 `Error`）都严格递增**，且跨「大厅 → 对局」连续。
- `GameTask::envelope_for` 每调用一次为该座位递增一次；每条 `SeatUpdate` 只调用一次 `envelope_for`，`CombatResult` 等逐座位消息各自独立编号。
- `Snapshot.seq` 由编码前先取该座位下一个序号再写入载荷，与信封自身 `seq` 必然一致。

## 5. 大厅流程与房间生命周期

- 认证：手机号 + 开发验证码 `123456` 或令牌续登；`playerId`/`token` 为服务端生成的不可预测串，不暴露手机号。会话有效期 24 小时（`SESSION_TTL_MS`），过期在下次访问或 `purge_expired` 时清除，M1 不跑后台清理任务。
- 建房：创建者坐 0 号座位并成为房主；房间码为 6 位大写字母数字，字母表排除易混字符 `0O1I`，首个房间码 `222222`。对局种子由房间码经 FNV-1a 64 位派生（`seed_for_code`）。
- 入房：坐最小空位；同一玩家重复加入同一房间幂等。非 `Lobby` 阶段成员集合冻结，加入/准备/选英雄/离开都返回 `INVALID_STAGE`。
- 离开：座位立即空出且不重排；房主离开时转给座位号最小的剩余玩家；**大厅最后一人离开即销毁房间**（注册表同时删 `by_code` 与 `rooms`）。
- 开局：8 个座位全部占用、全部准备且全部选好英雄后，由房主 `try_start` 把阶段推进到 `Recruit`，并在同一房间 task 内构造 `GameTask`。
- 房间 task 在 `hub` 条目被摘除后退出：`RoomTask::retire_if_done` 在「对局已结束且无连接」或「注册表已无该房间」时摘除 `hub` 条目并结束循环。

## 6. 对局计时与阶段推进

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `RECRUIT_SECONDS` | 90 | 开局与首回合的固定招募时长（秒） |
| `PACE_MS` | 320 | 单条战斗动作的演示时长（毫秒） |
| `TAIL_MS` | 2000 | 演示播完后的结果横幅停留（毫秒） |
| `ROUND_BUDGET_MS` | 150000 | 一个回合的总预算：招募 + 演示（毫秒） |
| `TIMER_WARNING_SECONDS` | 10 | 剩余秒数警告阈值（M1a 只暴露常量与 `seconds_left`） |
| `RECONNECT_GRACE_MS` | 30000 | 对局内断线保留窗口（毫秒） |
| `SEAT_COUNT` | 8 | 固定座位数 |

- 计时只走注入的 `Clock`，不读系统时间；房间 task 每 100 ms（`TICK_INTERVAL`）调用一次 `GameTask::tick`。
- 达到招募截止时刻时由服务端自动推进，不需要客户端 `EndRecruit`；全员结束招募也走同一条路径。
- 阶段推进分两步，`Combat` 是真实存在的时间段：
  - `enter_combat`：`finish_recruit → resolve_combat` → 存本场逐座位报告与演示时长 `T` → 阶段停在 `Combat`，`deadline = now + T`，逐座位下发 `CombatResult`。
  - `finish_combat`：由 `tick` 在 `Combat` 到点时调用 → `resolve_settle` →（非 `GameOver`）`start_turn`，下一回合招募截止为 `now + (150000 − T)`。
- 演示时长 `T = max(各座位 actions 数量) × PACE_MS + TAIL_MS`，不设上限；`recruit_budget_ms` 把差额夹到 0，演示超预算时下一回合招募截止即 `now`。
- 推到 `GameOver` 时逐座位下发 `GameOver`（排名从 1 起、种子为 16 位十六进制），不开始新的招募。
- 推进幂等：进入演示后阶段离开 `Recruit`，演示期间再次 `tick` 返回空；演示到点后阶段离开 `Combat`。
- 引擎 `resolve_combat` 会把阶段推到 `Settle`：`enter_combat` 随后把阶段拨回 `Combat` 并丢弃 `Settle` 阶段的 `PhaseChanged`，`finish_combat` 结算前再拨回 `Settle`，客户端在整场演示期间只看到 `Combat`。

## 7. 意图去重

- 去重键为 `(playerId, clientIntentId)`；表只增不删，随房间销毁。
- `Applied` 重发只回该座位当前视图的空事件 `StateUpdate`，不重复执行、不重复广播。
- `Rejected` 重发同一条 `Error`（同一错误码与消息），不重新判定。
- 未知玩家直接回 `NOT_IN_ROOM`，不进入去重表；非招募阶段的意图会写入去重表，重复提交返回同一条 `INVALID_STAGE` 错误。
- 意图先做只读预检再修改状态：拒绝时 `GameState` 与随机流逐字节不变，客户端可原样重发。
- 需要单一显式目标的效果（`target` 选择器，或 `friendly:single` 这类 `Side` + `Pick::Single`）在存在合法目标而未指定或目标不合法时返回 `NO_VALID_TARGET`。池为空时按效果来源区分：主动技能返回该错误（卡牌文档规定此时技能不可用），随从战吼与效果牌放行、效果落空。合法池只含意图提交前已存在的随从：战吼只在 `PlayCard` 时结算，买入不入场也不涉及目标。
- 目标预检不放进 `check_*`：这些函数同时用于生成视图 `affordances`，而视图层永远没有目标，加入目标判断会把需要补选的操作误标为不可用。
- 预检覆盖打出效果牌（`OnPlayEffect` 与全体 `OnUse` 响应）、打出手牌随从的战吼与英雄主动技能；买入只校验金币、手牌空位与牌池余量。

## 8. 逐座位过滤与战斗回放

`filter::for_seat` 把引擎事件映射为发给指定座位的协议事件；`filter::combat_result` 把逐座位战斗报告映射为 `CombatResult`。规则如下：

| 事件 | 可见范围 | 说明 |
| --- | --- | --- |
| `MinionBought`、`MinionSold`、`ShopRolled`、`EraUpgraded`、`TripleFormed`、`CardDiscovered` | 只发操作者本人 | 隐藏信息 |
| `DamageTaken`、`PlayerEliminated` | 全房间可见 | 公开信息，`playerId`/`seat` 指向涉及座位 |
| `TurnStarted`、`PhaseChanged`、`GameOver` | 全房间可见 | 不指向单个玩家，`playerId` 为 `null` |
| `CombatResolved` | 不下发 | 战斗结果改走逐座位 `CombatResult`，避免重复 |

- `CombatResult` 的动作类型固定为 `attack|death|reborn|contagion|summon`；侧别相对接收该消息的座位：己方棋盘标 `own`、对手标 `opponent`。`ownBoard`/`opponentBoard` 是接收座位视角的两份战前快照。
- 每条动作带受影响随从的实例 id 与动作后的状态：`attack` 带 `sourceMinionId`/`targetMinionId` 与 `source`/`target`（各含 `minionId`、`attack`、`health`、`alive`）；`death`/`reborn`/`contagion` 只带 `target`；`summon` 带 `summon{side, position, minion}`，`minion` 是完整的随从对象。客户端按这些下发值逐条落地，不做数值推算。
- `sourceIndex`/`targetIndex` 是战前快照下标，`sourceSide`/`targetSide` 指明它属于哪一侧（`own`/`opponent`，相对接收座位）；召唤物等不在快照中的实例下标与侧别为 `null`，位置改由 `summon.position` 给出。
- `mirror` 原样取自战斗报告；镜像战只向该座位发一条 `CombatResult`（报告里只有它自己的一条）。
- `View` 更新广播给全部 8 个座位：任何改变房间可见状态的操作都会推送 `StateUpdate`，因为对手的公开摘要（`boardCount`、`recruited`、阶段与回合）人人可见。事件仍按座位过滤：本人收到自己的私有事件，其他座位的 `events` 为空数组。

## 9. 断线重连

- 对局内断线：座位保留 30 秒（`RECONNECT_GRACE_MS`）。窗口内重连回一条全量 `Snapshot`；超过窗口只把座位标记为离线，不释放座位、不结束对局，也不向其他座位下发通知。
- `Combat` 阶段重连：快照之后追加补发该座位本场 `CombatResult`（报告保留到 `finish_combat` 为止），客户端据此按 `phaseDeadline` 续播。
- 重连与 `RequestSnapshot` 共用全量快照路径；M1 **不重放历史**，`last_seq` 不参与判断，一律回全量快照，快照序号必然大于重连前最后一条。
- 大厅断线：没有对局状态需要保护，立即离开并释放座位，随后广播 `RoomUpdate`。
- 连接代次：每次 `Attach`/`Reconnect` 带进程内唯一 `generation`；`Detach`/`Leave` 只在代次与当前登记一致时生效，避免新旧连接并存时旧连接迟到关闭误释放座位。
- M1 不做托管：`PlayerSummary.managed` 恒为 `false`。

## 10. 协议生成物

- `protocol/schema/v1/` 是唯一契约来源；`protocol/generated/protocol.rs` 由 `protocol-gen` 用 typify 生成并随源提交，`protocol/generated/protocol.gd` 由 `scripts/gen_protocol_gdscript.py` 生成并随源提交。
- `make protocol-rs` 重新生成 Rust 类型；`make protocol-rs-check` 比较内存生成结果与磁盘内容，不一致即非零退出。
- `make protocol` 生成 GDScript 常量与协议字段表；`make protocol-check` 运行样例校验并比对两处生成物漂移。

## 11. `sim bot`

机器人客户端连接真实服务端，按只读视图决策打完一局：

```
sim bot --connect ws://HOST:PORT/ws --name NAME --hero CARD_ID
        [--room CODE] [--create] [--intent-seed N]
```

- `--connect`、`--name`、`--hero` 必填；`--room` 加入既有房间（与 `--create` 二选一，`--create` 时忽略 `--room`）；`--create` 建房并把房间码输出到 stdout 首行 `ROOM_CODE=<code>`；`--intent-seed` 默认 0，只用于在发现候选间做确定性选择。
- 协议顺序固定：`AuthRequest` →（`CreateRoom` | `JoinRoom`）→ `SelectHeroCard` → `Ready` → 等 `GameStarted` → 循环按 brain 决策发送 `IntentMessage`，直到 `GameOver`。
- 每个意图使用确定性递增的 `clientIntentId`（`{name}-{counter}`）；客户端只对「自身视角有新变化」的视图做一次决策，避免重播视图导致重复意图。
- 收到 `NO_VALID_TARGET` 时按 `clientIntentId` 找回被拒意图，补上棋盘第一个己方随从重发；没有随从时退回 `EndRecruit` 兜底。
- `bot/brain.rs` **只读** `PlayerView`（含服务端算好的 `affordances`）与静态卡表，不链接引擎判定、不读时钟与环境变量、不使用浮点；优先级固定为：消费 `pendingChoices`（发现/进化发 `ChooseDiscover`，目标待选发 `ChooseTarget` 取棋盘最左随从）→ 打最小下标可打手牌 → 买最贵可买槽位 → 升时代 → 刷新 → `EndRecruit`。
- `sim/src/ai.rs` 只新增 `PendingChoice::Target` 的确定性分支与 `PlayCard.position: None`（追加到最右），其余策略未改动，M0 基线数值不变；view 驱动的新逻辑在 `bot/brain.rs`。

## 12. 命令与验收证据

`make m1-check` 汇总 M1a 验收，目标体为：

```make
m1-check:
	$(MAKE) protocol-rs-check
	$(MAKE) protocol-check
	$(MAKE) data-check
	$(MAKE) test
	$(MAKE) lint
	$(MAKE) m0-determinism
```

**e2e 由 `make test` 覆盖**：`make test` 的 `cargo test --workspace` 已包含 `room-service/tests/e2e.rs`（真实 WebSocket 起 8 个 bot 打完一局），因此 `m1-check` 不单独再跑一次 e2e，避免重复起 8 个 bot。

下表为 `make m1-check` 一次实际运行的结果（整体退出码 0，总耗时约 13 分钟，其中 `dev_bots` 单跑约 242 秒）：

| 步骤 | 命令 | 结果摘要 | 退出码 |
| --- | --- | --- | --- |
| 协议 Rust 生成物 | `make protocol-rs-check` → `cargo run -q -p protocol-gen -- --check` | 无差异 | 0 |
| 协议生成物 | `make protocol-check` → `validate_protocol.py`、`gen_protocol_gdscript.py --check`、`gen_protocol_docs.py --check` | `协议样例校验通过`，GDScript 与字段表无漂移 | 0 |
| 卡牌数据 | `make data-check` → `gen_card_data.py --check`、`audit_effects.py` | `生成数据无漂移`、`审计通过` | 0 |
| 测试 | `make test` → `unittest discover`、`cargo test --workspace` | Python `Ran 70 tests ... OK`；workspace 全部 `test result: ok`，其中 `room-service` 集成测试 89 个（auth 10、e2e 9、filter 13、game 24、reconnect 9、room 22、dev_bots 2），e2e 用例 `eight_bots_finish_a_game` 通过 | 0 |
| 静态检查 | `make lint` → `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings` | 无警告 | 0 |
| 确定性 | `make m0-determinism` → `cargo test -p sim --test sim_determinism replay_same_log_100_times_has_same_hash` | `1 passed`（约 22 s） | 0 |

## 13. 与设计的三处偏差

1. 设计 §5.4 写 `view(state, seat, catalog) -> PlayerView`；因 `rule-engine` 不依赖 `protocol`，改为引擎返回自有 `SeatView`，由 `room-service::view::to_protocol` 做机械映射。规则判定仍全部在引擎。
2. 设计 §5.5 写「AI 决策改为同时接受 `PlayerView`」；为保住 M0 基线（`ai.rs` 的策略直接决定 1000 局基线数值），改为新增 view 驱动的 `sim/src/bot/brain.rs`，`ai.rs` 仍只读引擎状态、不接受 `PlayerView`（本需求只加了 `PendingChoice::Target` 的确定性分支与 `PlayCard.position: None`）。
3. 设计 §5.2 要求存 `tokenHash` 与 connection；M1a 在内存中保存明文 token，且不建立 connection 索引。令牌只存在于内存、有效期 24 小时、不落盘；`Session` 的 `Debug` 输出已把 `token` 标为 `<redacted>`，避免日志泄漏。

## 相关文档

- [规则引擎与模拟器](rule-engine-and-simulator.md)
- [技术现状索引](README.md)
- [服务端技术基线](../../versions/v0.1/technical/architecture/server-stack.md)
- [通信协议](../../versions/v0.1/technical/architecture/protocol.md)
