# 房间服务与联网

> 状态：已实现（v0.1 M1a–M2 房间服务与联网，含断线重连一致性用例与对局观察日志）
> 代码：`server/crates/room-service`、`server/crates/protocol`、`server/crates/protocol-gen`、`server/migrations`、`protocol/schema/v1`、`protocol/generated`
> 验收：`make m1-check`、`make db-test`

本文记录 v0.1 已实现的房间服务现状：crate 边界、WebSocket 接入、下行序号、去重、逐座位过滤、计时与断线处理、好友与邀请、房间生命周期、赛后结果、PostgreSQL 持久化、`sim bot` 用法与验收方式；不描述 M1b 的 Godot 客户端，也不描述产品规则（规则见[规则引擎与模拟器](../rule-engine-and-simulator.md)）。

## 1. 工程结构

```
server/
  Cargo.toml                    # workspace：crates/{protocol, protocol-gen, rule-engine, room-service, sim}
  crates/
    protocol/                   # serde 类型：由 schema 生成，含客户端/服务端消息与视图
    protocol-gen/               # 生成器：schema bundle → protocol/generated/protocol.rs
    rule-engine/                # 纯库：规则判定与逐座位视图（src/view.rs）；玩法抽取（src/playstyles.rs）
    room-service/               # 可执行：WebSocket 接入、房间、对局驱动
      src/lib.rs                # AppState、路由装配、内嵌 v0.1 数据
      src/main.rs               # --listen（默认 127.0.0.1:8080）
      src/net.rs                # /ws 连接 task、写端 task、房间 task（单写者）
      src/room.rs               # 大厅房间状态机、房间玩法抽取与房间注册表
      src/game.rs               # 对局驱动：计时、阶段推进、去重、逐座位视图
      src/filter.rs             # 逐座位事件过滤与 CombatResult 映射
      src/view.rs               # 引擎 SeatView → 协议 PlayerView 的纯函数映射
      src/envelope.rs           # 单连接下行信封序号 SeatChannel
      src/auth.rs               # 手机号+开发验证码 / 令牌续登 / 6 位玩家编号
      src/account.rs            # 手机号 → 持久化账号身份，内存缓存 + 写穿
      src/social.rs             # 好友、申请、最近对局与房间邀请，内存缓存 + 写穿
      src/db.rs                 # PostgreSQL 连接池、基线迁移与唯一 SQL 落点
      src/clock.rs              # Clock 注入（SystemClock / FixedClock）
      src/error.rs              # 稳定错误码 → ErrorMessage
      tests/                    # auth / room / game / filter / reconnect / social / social_e2e / persistence / e2e
    sim/
      src/bot/brain.rs          # 只读 PlayerView 的确定性机器人决策
      src/bot/client.rs         # `sim bot --connect` 的真实 WebSocket 客户端
      src/bot/mod.rs
      src/ai.rs                 # M0 确定性 AI（为保基线未改动）
      src/lib.rs                # run_games / play_game / replay；重导出 seed_from_u64
      src/main.rs               # play / batch / replay / report / check / bot
  migrations/
    0001_baseline.sql           # 账号、好友、申请、最近对局、邀请的基线迁移
protocol/
  schema/v1/                    # 唯一契约来源（JSON Schema draft 2020-12）
  generated/protocol.rs         # typify 生成物，随源提交
  generated/protocol.gd         # GDScript 常量生成物，随源提交
```

## 2. 依赖方向与边界

- `room-service → protocol`、`room-service → rule-engine`；`room-service` 的 dev-dependencies 含 `sim`，供 e2e 用真实机器人驱动服务端。
- `room-service` 直连 PostgreSQL（`sqlx` 0.8），SQL 只出现在 `src/db.rs`：`account.rs` 与 `social.rs` 调用 `Db` 的方法，自己只保留内存缓存与规则。`sqlx::migrate!("../../migrations")` 在启动时执行 `server/migrations/` 下的基线迁移。
- `sim → protocol`、`sim → rule-engine`；`sim` 不依赖 `room-service`，因此 `bot/mod.rs` 重复声明开发验证码 `DEV_SMS_CODE = "123456"`。
- `rule-engine` 不依赖 `protocol`、tokio、sqlx 或任何 I/O 库；它不读时钟、不做文件 I/O，并提供自有的逐座位视图 `SeatView`（`src/view.rs`）。
- 网络、计时与过滤全部在 `room-service`；规则判定全部在 `rule-engine`，由编译期依赖图保证引擎无副作用。

## 3. WebSocket 接入与信封

- 路由只有 `GET /ws`（`axum::routing::get`），升级后进入连接 task；`AppState` 内嵌 v0.1 的 `cards.json`、`heroes.json`、`skills.json`、`tokens.json`、`config.json`、`taxonomy.json`。
- 连接拓扑：每个连接一个读端 task 与一个写端 task；每个房间一个房间 task（单写者，独占 `Room`/`GameTask` 可变状态）。连接层从不直接改房间状态，只投递 `RoomMessage`。
- 上行统一为 `ClientEnvelope { v, id, type, ts, payload }`。**生成类型不校验 `v`**：连接层显式拒绝 `v != 1`，回 `BAD_PAYLOAD`（`不支持的协议版本`）后关闭连接，避免按 v1 语义执行 v2 载荷。
- `envelope.parse()` 按 `type` 判别并校验 `payload` 结构；未知 `type` 回 `UNKNOWN_TYPE`，载荷不匹配回 `BAD_PAYLOAD`（`上行载荷与类型不匹配`），两者都继续读下一条，不关闭连接。
- 未入房时，连接层错误走连接自己的 `SeatChannel`；入房后一律经房间 task 走该座位的统一下行通道，保证序号跨来源单调。

## 4. 下行序号

- 每个玩家连接持有独立的 `SeatChannel`，`seq` 从 1 起递增；失败的构造同样消耗序号，不回退。
- 同一玩家有多条下行来源：连接层 `direct`、大厅 `slot.channel`、对局 `GameTask` 通道，以及房间外的社交控制通道。三者共享一个会话级 `SeqFloor`（`Arc<AtomicU64>` 下限）：编码前先把通道推进到当前下限，发送后再把新 `seq` 记回下限，因此**同一连接的任何下行（含 `Error`）都严格递增**，且跨「大厅 → 对局」连续。
- `GameTask::envelope_for` 每调用一次为该座位递增一次；每条 `SeatUpdate` 只调用一次 `envelope_for`，`CombatResult` 等逐座位消息各自独立编号。
- `Snapshot.seq` 由编码前先取该座位下一个序号再写入载荷，与信封自身 `seq` 必然一致。

## 5. 大厅流程与房间生命周期

- 认证：手机号 + 开发验证码 `123456` 或令牌续登；手机号经 `AccountStore` 解析为持久化账号（`playerId`、6 位玩家编号、展示名），同一手机号重新登录恢复同一账号；`playerId`/`token` 为服务端生成的不可预测串，不暴露手机号。认证结果同时下发 6 位数字 `playerCode`，好友申请按编号定位玩家。会话有效期 24 小时（`SESSION_TTL_MS`），过期在下次访问或 `purge_expired` 时清除，M1 不跑后台清理任务；会话仍是进程内状态，不落盘。
- 建房：创建者坐 0 号座位并成为房主；房间码为 6 位大写字母数字，字母表排除易混字符 `0O1I`，首个房间码 `222222`。对局种子由房间码经 FNV-1a 64 位派生（`seed_for_code`）。
- 房间玩法：建房时用房间码种子调用 `rule_engine::playstyles::draw(seed_for_code(code))`，抽取 5 个玩法存入 `RoomState.playstyles`；大厅期间与断线重连保持不变。对局结束即销毁房间，不再重抽。玩法必须早于英雄候选确定，因此抽取时点在房间创建，而不是 `Engine::new`。
- 入房：坐最小空位；同一玩家重复加入同一房间幂等。非 `Lobby` 阶段成员集合冻结，加入/准备/选英雄/离开都返回 `INVALID_STAGE`。
- 英雄候选：建房与入房时按房间码派生的种子把全部英雄卡（`Catalog.heroes`）id 升序后做确定性 Fisher–Yates 洗牌，再按进房顺序每 4 张切给一名玩家（`CANDIDATE_COUNT`），因此同房间内各玩家候选互不重复；候选保存在 `PlayerSlot.hero_candidates`，重进与重连都不重抽。
- 英雄候选过滤：候选只取 `hero_in_pool` 通过的英雄卡，按本局 5 个玩法过滤；英雄 `categories` 中任一条 `play:` 命中即通过，学者、工具卡与普通英雄同一口径。过滤后一张英雄都没有时回退到全部英雄卡（产品兜底）。
- 候选池不足：池大小 ≥ `SEAT_COUNT * CANDIDATE_COUNT`（32）时沿用分段、房间内互不重复；不足 32 时按 `(segment * 4 + i) % len` 取 `min(4, len)` 张，允许跨玩家重复；池小于 4 时每人取整个池并至少 1 张。
- 选英雄校验：id 必须在英雄池 `Catalog.heroes` 中，且在本人候选内（否则 `NOT_CANDIDATE` → `INVALID_CHOICE`）；座位已 `ready` 时再选返回 `INVALID_STAGE`。
- 候选下发：`RoomUpdate.players[].heroCandidates` 只在发给本人的那份里填充，其余座位为 `null`；广播按座位逐个序列化，不共用 payload。这是展示规则而非机密：选中的 `heroCardId` 仍对全体成员可见，候选分组也能由房间码确定性复算。
- 候选池校验：`AppState::new` 要求英雄卡至少 1 张，且每张英雄卡的 `categories` 至少 1 条 `play:`、取值是合法玩法（不含 `neutral`），`prey:` / `host:` 节点必须在分类树中；不再要求 32 张。池不足时由跨玩家重复兜底，房间仍能开局。
- 大厅断线：`Detach` 不再立即释放座位，而是标记离线并记录 `disconnected_at_ms`；房间 task 的 tick 调用 `tick_disconnects`，超过 `RECONNECT_GRACE_MS` 才按原规则释放座位。`Leave` 命令仍立即释放。座位已被释放后再 `Reconnect` 回 `NOT_IN_ROOM`，并摘除该座位的下行登记。
- 离开：座位立即空出且不重排；房主离开时转给座位号最小的剩余玩家；**大厅最后一人离开即销毁房间**（注册表同时删 `by_code` 与 `rooms`）。
- 移除：`KickMember{targetPlayerId}` 映射为 `RoomCommand::Kick`；仅房主、仅 `Lobby`、不能移除自己。成功后被移除者收到 `RoomClosed{reason:"kicked"}` 并移除发送端，座位空出且编号不重排；房间码与其余成员不变。错误码沿用 `NOT_HOST` / `INVALID_STAGE` / `NOT_IN_ROOM` / `INVALID_CHOICE`（移除自己映射为 `InvalidTarget`），不新增错误码。
- 解散：`DissolveRoom{}` 映射为 `RoomCommand::Dissolve`；仅房主、仅 `Lobby`。成功后注册表同时删除 `rooms` 与 `by_code`，所有成员收到 `RoomClosed{reason:"dissolved"}`，房间码立即失效；房间 task 由 `retire_if_done` 回收。
- 开局：8 个座位全部占用、全部准备且全部选好英雄后自动 `try_start`（最后一次 `Ready` 触发，不需要房主额外操作），把阶段推进到 `Recruit`，并在同一房间 task 内构造 `GameTask`。
- `GameTask::new` 接收房间已抽好的 5 个玩法，开局原样传给 `Engine::new(catalog, seed, playstyles)`；同一局内 `GameOver.seed` 不变，断线重连复用同一组玩法。
- 对局结束即销毁：`RoomTask::deliver_excluding` 投递完 `GameOver` 后 `self.game.take()`、清空发送端并调用 `RoomRegistry::remove(room_id)`；房间码随之失效，`retire_if_done` 在下一轮发现注册表已无房间，摘除 `hub` 条目并结束 task。不保留等待区，也不重置给下一局。
- 对局中主动退出：对局阶段收到 `LeaveRoom` 时 `RoomTask.leave` 转成 `RoomCommand::Forfeit`；规则引擎 `settle::forfeit` 把未淘汰座位置 `hp = 0`、`eliminated = true`，把座位插到 `ranking` 最前（最近出局在前），发出 `PlayerEliminated{place: 当时存活人数}`；只剩 1 名未淘汰者时立即补齐排名并进入 `GameOver`。已淘汰座位再次退出返回空事件。退出者发送端立即移除、停止接收该局消息，座位保留到本局结束，房间销毁时一并消失。
- 房间 task 在 `hub` 条目被摘除后退出：`RoomTask::retire_if_done` 只在注册表已无该房间时摘除 `hub` 条目并结束循环。对局结束、解散与大厅最后一人离开都会删除注册表房间，task 随之退出。

## 6. 好友、在线状态与房间邀请

- 存储：账号、好友关系、待处理申请与发送记录、最近对局、房间邀请落 PostgreSQL（表见 `server/migrations/0001_baseline.sql`，逐表说明见 [表文档](models/README.md)）。`AccountStore` 与 `SocialStore` 在内存中持有全量缓存，读走内存；写操作先落库、成功后再改内存，失败回 `INTERNAL_ERROR` 且内存不变。启动时 `Db` 全量载入并清理过期申请、发送记录与邀请；未配置 `DATABASE_URL` 时进程退化为纯内存模式（仅供本地开发，启动时打印提示）。
- 注册：认证成功时用手机号解析账号（`account` 表 `account_id` 即 `playerId`），按 `player_code` 注册档案；`playerCode` 为 6 位数字（`100000` 起，`player_code_seq` 分配、`player_code` 唯一约束兜底），同一手机号复用同一编号，不对其它账号复用；`displayName` 首次创建为 `玩家N`（`account.ordinal` 自增序号，重启不回退），重复登录不覆盖已设置的名字。玩家对外只暴露编号，不暴露 `playerId`。
- 申请：`FriendRequest{toPlayerCode}` 按编号查找目标；自己、已是好友、任一方有待处理申请（四向检查）分别回 `FRIEND_SELF`、`FRIEND_ALREADY`、`FRIEND_REQUEST_PENDING`；申请 7 天过期，被拒绝后 24 小时内同一人最多再申请 3 次（`FRIEND_REQUEST_LIMIT`）。
- 上限：好友 100（双向），收到的待处理申请 20；超出回 `FRIEND_LIST_FULL` / `FRIEND_REQUEST_FULL`。
- 同意/拒绝：`FriendRespond{fromPlayerId, accept}` 只处理发给自己的申请；双方都会收到最新 `FriendUpdate`。
- 删除：`FriendRemove{friendId}` 双向解除；非好友回 `NOT_FRIENDS`。
- 最近对局：对局开始时按参战座位记录，每名玩家保留最近 20 名同局玩家，同一人只保留最近一次，最新的在前。
- 在线状态：`online`（已认证且连接在）、`inGame`（座位处于 `Recruit`/`Combat`）、`offline`。好友列表按 online → inGame → offline、展示名首字符、`playerId` 排序。
- 连接登记：认证成功后把连接登记进 `ConnectionRegistry`（`playerId` → 控制通道 + 代次），断开时按代次移除；`is_online` 与房间注册表共同决定好友状态。
- 推送路由：房间内玩家的社交消息投递到房间 task（`RoomMessage::Push`），由房间单写者写回该座位，序号与对局消息共用同一条通道；房间外走连接的控制通道（`ControlMessage`）。锁顺序固定为 connections → rooms → social，避免死锁。
- 邀请：`InviteToRoom{friendId}` 只在本人处于 `Lobby` 阶段房间、房间未满、对方在线且不在任何房间时成功，否则回 `NOT_IN_ROOM` / `INVALID_STAGE` / `ROOM_FULL` / `FRIEND_OFFLINE` / `FRIEND_BUSY`；邀请 60 秒过期。
- 接受：被邀请方收 `FriendInvite{inviteId, fromPlayerId, fromDisplayName, fromPlayerCode, roomCode, expiresAt}`；`RespondInvite{inviteId, accept}` 后服务端重新校验房间仍在大厅且未满，通过则给被邀请方 `InviteAccepted{inviteId, roomCode}`、给邀请方 `FriendInviteResult{inviteId, accepted, reason}`。客户端收到 `InviteAccepted` 后自行发普通 `JoinRoom{code}` 入房，房间 task 不跨任务挂载连接。
- 拒绝/失效：拒绝时双方都收到 `accepted=false` 的回执；邀请过期或已被处理回 `INVITE_EXPIRED` / `INVITE_NOT_FOUND`。
- 邀请不改变房间成员关系；被邀请者仍按普通 `JoinRoom` 坐最小空位。

### 6.1 持久化

- 连接：入口读取 `DATABASE_URL`，`Db::connect` 建池（10 条连接，5 秒获取超时）并执行 `sqlx::migrate!("../../migrations")`；随后 `AccountStore::load`、`SocialStore::load` 全量载入，再绑定端口。数据库连不上时进程直接失败，不静默降级。
- 表：`account`、`friendship`、`friend_request`、`friend_request_attempt`、`recent_opponent`、`room_invite` 六张表，基线迁移 `server/migrations/0001_baseline.sql`；逐表说明见 [表文档](models/README.md)。时间列统一 `TIMESTAMPTZ(3)`，值由注入的 `Clock` 传入，不用数据库 `now()`。
- 写穿：申请（申请行 + 发送记录）、处理申请（删申请 + 建双向好友 + 清双方发送记录）、删除好友（双向关系 + 两个方向申请与记录）、记录对局（逐对写入并裁剪到 20 人）、创建邀请（先清过期再写）各自在一个事务内完成；事务提交后才改内存。失败时映射为 `SocialError::Storage` / `INTERNAL_ERROR`，内存保持原状。
- 载入：`load_social` 先删除过期的申请、发送记录与邀请，再按上限读回；`AccountStore` 重建手机号索引与编号索引，`SocialStore` 重建好友、申请、发送记录、最近对局与邀请。
- 邀请 id 为 `inv_<uuid>`，跨重启与多实例不依赖进程内自增序号；房间本身仍在进程内，重启后邀请指向的房间已不存在，接受时按 `ROOM_NOT_FOUND` 失败。
- 测试：`room-service/tests/persistence.rs` 为每个用例建独立 schema（`CREATE SCHEMA` + `search_path`），在真实 PostgreSQL 上验证重启恢复；`make db-up` / `make db-test` / `make db-reset` 封装起库、跑持久化测试与重建库。未设置 `TEST_DATABASE_URL` / `DATABASE_URL` 时用例打印跳过原因并返回。

## 7. 对局计时与阶段推进

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `INITIAL_RECRUIT_SECONDS` | 60 | 首回合招募时长（秒） |
| `RECRUIT_STEP_SECONDS` | 15 | 每回合递增的招募秒数 |
| `RECRUIT_CAP_SECONDS` | 120 | 招募时长上限（秒） |
| `TIMER_WARNING_SECONDS` | 10 | 剩余秒数警告阈值（M1a 只暴露常量与 `seconds_left`） |
| `RECONNECT_GRACE_MS` | 30000 | 对局内断线保留窗口（毫秒） |
| `SEAT_COUNT` | 8 | 固定座位数 |

- 计时只走注入的 `Clock`，不读系统时间；房间 task 每 100 ms（`TICK_INTERVAL`）调用一次 `GameTask::tick`。
- 招募时长按回合增长：`recruit_seconds(turn) = min(60 + (turn − 1) × 15, 120)`；服务端不估算演示时长，服务端不再有 `PACE_MS` / `TAIL_MS` / 回合预算常量，演示节奏只在客户端。
- 达到招募截止时刻时由服务端自动推进，不需要客户端 `EndRecruit`；全员结束招募也走同一条路径。
- 阶段推进分两步，`Combat` 的退出由客户端回报驱动：
  - `enter_combat`：`finish_recruit → resolve_combat` → 存本场逐座位报告、记录开战时刻 `combat_started_at_ms`、标记参战座位 → 阶段停在 `Combat`，`deadline = now + recruit_seconds(turn + 1)`，逐座位下发带 `startedAt` 的 `CombatResult`。
  - `CombatPlayed`（`RoomCommand::CombatPlayed { player_id }`）：参战座位记为已回报；非参战座位与重复回报忽略。全部参战座位回报后立即调用 `finish_combat`。
  - `finish_combat`：`resolve_settle` → 追加战报 `combatLog` →（非 `GameOver`）`start_turn`；不改写 `deadline`，下一回合招募时钟沿用开战时设定的截止。
- 兜底：`tick` 在 `Combat` 到点（`deadline`）时仍调用 `finish_combat`，覆盖从未回报的座位；因此无论演示是否播完，下一回合都会按开战 + `recruit_seconds(turn + 1)` 起跑。
- 断线计为已回报：`Combat` 阶段断线时把该座位标记为参战座位已回报，若因此全员回报则当场结算。
- 战报在结算时才写入 `combatLog`：演示期间观众与参战者都看不到本场条目。
- 推到 `GameOver` 时逐座位下发 `GameOver`（排名从 1 起、种子为 16 位十六进制），不开始新的招募。
- `GameOver.ranking[]` 每项含 `place`、`playerId` 与可选 `survivedTurns`（0–20）、`damageDealt`；`selfStats` 只填接收者本人，含 16 个必填字段：`place`、`survivedTurns`、`finalHp`、`finalEra`、`remainingGold`、`damageDealt`、`damageTaken`、`buys`、`sells`、`rerolls`、`triples`、`combatWins`、`combatDraws`、`combatLosses`、`heroPowerUses`、`board`（最多 7 项，每项 `cardId` / `golden` / `attack` / `health`）。
- 统计来源：`buys` / `sells` / `rerolls` / `triples` 由 `MinionBought` / `MinionSold` / `ShopRolled` / `TripleFormed` 事件累计；`damageTaken` 由 `DamageTaken` 累计；`combatWins` / `combatDraws` / `combatLosses` 由 `CombatResolved` / `CombatDrawn` 累计；`heroPowerUses` 跨回合累计 `PlayerState.hero_uses` 的增量；`survivedTurns` 出局座位取淘汰事件回合、存活至结束取 `state.turn`；`finalHp` / `finalEra` / `remainingGold` / `damageDealt` / `board` 终局直接读 `PlayerState` 与棋盘。
- 弃权名次：引擎把弃权者视作「最近出局」插到 `ranking` 临时块最前；最终 `GameOver.ranking` 仍是第 1 名在前，弃权者排在后续出局者之前、存活者之后，名次等于弃权时的存活人数。
- 推进幂等：进入演示后阶段离开 `Recruit`，演示期间再次 `tick` 返回空；全部参战座位回报或兜底到点后阶段离开 `Combat`。
- 引擎 `resolve_combat` 会把阶段推到 `Settle`：`enter_combat` 随后把阶段拨回 `Combat` 并丢弃 `Settle` 阶段的 `PhaseChanged`，`finish_combat` 结算前再拨回 `Settle`，客户端在整场演示期间只看到 `Combat`。

## 8. 意图去重

- 去重键为 `(playerId, clientIntentId)`；表只增不删，随房间销毁。
- `Applied` 重发只回该座位当前视图的空事件 `StateUpdate`，不重复执行、不重复广播。
- `Rejected` 重发同一条 `Error`（同一错误码、`messageId` 与 `messageParams`），不重新判定。
- 下行 `ErrorMessage` 的玩家文案由 `messageId` + `messageParams` 决定：默认 `messageId` 为 `error.<CODE>`，细分场景用 `error.<CODE>.<detail>` 这类 id，参数按 `%s` 填充；客户端按文本表取用与格式化。`message` 只面向日志，不作为 UI 文案。
- 未知玩家直接回 `NOT_IN_ROOM`，不进入去重表；非招募阶段的意图会写入去重表，重复提交返回同一条 `INVALID_STAGE` 错误。
- 意图先做只读预检再修改状态：拒绝时 `GameState` 与随机流逐字节不变，客户端可原样重发。
- 需要单一显式目标的效果（`target` 选择器，或 `friendly:single` 这类 `Side` + `Pick::Single`）在存在合法目标而未指定或目标不合法时返回 `NO_VALID_TARGET`。池为空时按效果来源区分：主动技能返回该错误（卡牌文档规定此时技能不可用），随从战吼与效果牌放行、效果落空。合法池只含意图提交前已存在的随从：战吼只在 `PlayCard` 时结算，买入不入场也不涉及目标。
- 目标以 `TargetRef { zone, id }` 提交，`zone` 取 `hand` / `shop` / `board`。提交时先校验 `id` 是否存在于本人在该区域的实体中（三个区域共用实例编号空间，同一编号只可能属于一个区域）；区域不符返回 `NO_VALID_TARGET`，再进入效果的合法池校验。
- 协议层不做候选校验：客户端本应只提交视图下发的候选，但服务端仍以自身池为准重复校验，视图与结算同源（见 §7）。
- 目标预检与候选生成共用同一套 `check_targets`：视图的 `targetCandidates` 由引擎 `target_candidates` 逐个候选重跑预检得到，不另写筛选逻辑，避免视图与结算漂移。视图侧「需要目标但没有候选」时把操作标为 `NO_VALID_TARGET` 不可用，而不是把候选塞进 `check_*`。
- 预检覆盖打出效果牌（`OnPlayEffect` 与全体 `OnUse` 响应）、打出手牌随从的战吼与英雄主动技能；买入只校验金币、手牌空位与牌池余量。

## 9. 逐座位过滤与战斗回放

`filter::for_seat` 把引擎事件映射为发给指定座位的协议事件；`filter::combat_result` 把逐座位战斗报告映射为 `CombatResult`。规则如下：

| 事件 | 可见范围 | 说明 |
| --- | --- | --- |
| `MinionBought`、`MinionSold`、`ShopRolled`、`EraUpgraded`、`TripleFormed`、`CardDiscovered` | 只发操作者本人 | 隐藏信息 |
| `DamageTaken`、`PlayerEliminated` | 全房间可见 | 公开信息，`playerId`/`seat` 指向涉及座位 |
| `TurnStarted`、`PhaseChanged`、`GameOver` | 全房间可见 | 不指向单个玩家，`playerId` 为 `null` |
| `CombatResolved`、`CombatDrawn` | 不下发 | 战斗结果改走逐座位 `CombatResult`，避免重复；`CombatDrawn` 只用于观测 |

- `CombatResult` 的动作类型固定为 `attack|death|reborn|contagion|summon`；侧别相对接收该消息的座位：己方棋盘标 `own`、对手标 `opponent`。`ownBoard`/`opponentBoard` 是接收座位视角的两份战前快照。
- 每条动作带受影响随从的实例 id 与动作后的状态：`attack` 带 `sourceMinionId`/`targetMinionId` 与 `source`/`target`（各含 `minionId`、`attack`、`health`、`alive`）；`death`/`reborn`/`contagion` 只带 `target`；`summon` 带 `summon{side, position, minion}`，`minion` 是完整的随从对象。客户端按这些下发值逐条落地，不做数值推算。
- `sourceIndex`/`targetIndex` 是战前快照下标，`sourceSide`/`targetSide` 指明它属于哪一侧（`own`/`opponent`，相对接收座位）；召唤物等不在快照中的实例下标与侧别为 `null`，位置改由 `summon.position` 给出。
- `summon.position` 由引擎按**存活随从序列**给出：召唤后该随从在存活随从中的下标，已阵亡条目不计数，上界由 `PlayerView.boardSlots` 决定。`summon.minion.index` 与它同值，语义一致（客户端不读该字段，按 `position` 插入）。
- `PlayerView.boardSlots` 取自 `catalog.config.board_slots`（v0.1 为 7），放在视图顶层：重连只回 `Snapshot`，而视图每次刷新都带它，客户端不需要额外的续传逻辑。
- `PlayerView.enabledFactions` 字段名保留，取值为本局启用的 5 个玩法 id；`SeatView.enabled_playstyles` 由引擎按 `enabled_playstyles` 生成，`room-service::view::to_protocol` 做 `[String; 5]` 转换，8 个座位取值一致。
- `mirror` 原样取自战斗报告；幽灵战只向该座位发一条 `CombatResult`（报告里只有它自己的一条）。幽灵来自已出局者随机抽中的最终阵容，在引擎 `resolve_ghost` 里按幽灵自己的光环来源重算；实例编号只在必要时（与己方重号时）于本次战斗克隆内改号，与己方棋盘不相交，因此动作的实例编号在两侧可判。
- `opponentPlayerId` 在幽灵战指向幽灵归属座位（已出局者）的 `playerId`；幽灵归属者本人观战时不指向自己，已出局观战者、招募阶段或未配对时为 `null`。
- `combatLog` 来自 `SeatCombatReport`：真实战每个座位各有一份报告，只保留座位号较小的一份；幽灵战本来只有落单座位一条。战报回合号取结算时刻的 `state.turn`，追加后只保留最近 5 个回合（早于 `turn − 4` 的条目丢弃），按回合升序下发；`mirror=true` 表示该场为幽灵战。
- `summon.minion` 的攻击与生命取该动作的状态快照（含光环与战斗内加成），卡面字段（`cardId`/`golden`/`keywords`）来自动作本身；快照缺失时才退回卡面基础值。
- `View` 更新广播给全部 8 个座位：任何改变房间可见状态的操作都会推送 `StateUpdate`，因为对手的公开摘要（`boardCount`、`recruited`、阶段与回合）人人可见。事件仍按座位过滤：本人收到自己的私有事件，其他座位的 `events` 为空数组。

## 10. 断线重连

- 对局内断线：座位保留 30 秒（`RECONNECT_GRACE_MS`）。窗口内重连回一条全量 `Snapshot`；超过窗口只把座位标记为离线，不释放座位、不结束对局，也不向其他座位下发通知。
- `Combat` 阶段重连：快照之后追加补发该座位本场 `CombatResult`（报告保留到 `finish_combat` 为止），客户端据此按 `phaseDeadline` 续播。
- 对局已结束：房间已在对局结束时销毁，旧 `roomId` 重连只回 `ROOM_NOT_FOUND`（房间 task 已退出）或 `NOT_IN_ROOM`（task 尚未回收），不补发 `GameOver`，也不恢复房间。结算页断线后直接回大厅。
- 重连与 `RequestSnapshot` 共用全量快照路径；M1 **不重放历史**，`last_seq` 不参与判断，一律回全量快照，快照序号必然大于重连前最后一条。
- 大厅断线：保留座位一个 `RECONNECT_GRACE_MS`（30 秒）宽限，超时由房间 tick 释放并广播 `RoomUpdate`；宽限内 `Reconnect` 回同一座位与同一组候选。本局玩法在房间创建时已确定，重连不重抽。
- 连接代次：每次 `Attach`/`Reconnect` 带进程内唯一 `generation`；`Detach`/`Leave` 只在代次与当前登记一致时生效，避免新旧连接并存时旧连接迟到关闭误释放座位。
- M1 不做托管：`PlayerSummary.managed` 恒为 `false`。

## 11. 协议生成物

- `protocol/schema/v1/` 是唯一契约来源；`protocol/generated/protocol.rs` 由 `protocol-gen` 用 typify 生成并随源提交，`protocol/generated/protocol.gd` 由 `scripts/gen_protocol_gdscript.py` 生成并随源提交。
- 本次新增上行 `KickMember{targetPlayerId}`、`DissolveRoom{}` 与下行 `RoomClosed{reason: "kicked" | "dissolved"}`；`GameOver` 新增可选 `selfStats`，`ranking[]` 新增可选 `survivedTurns` / `damageDealt`。生成物与样例随源提交。
- `view.schema.json` 的 `enabledFactions` 描述为本局启用的 5 个玩法；字段名与长度不变，取值为 `mammal` 等玩法 id。
- 协议样例 `protocol/samples/v1/s2c/*.json` 的 `enabledFactions` 为玩法 id，`client/tests/fixture.gd` 同步。
- `make protocol-rs` 重新生成 Rust 类型；`make protocol-rs-check` 比较内存生成结果与磁盘内容，不一致即非零退出。
- `make protocol` 生成 GDScript 常量与协议字段表；`make protocol-check` 运行样例校验并比对两处生成物漂移。

## 12. `sim bot`

机器人客户端连接真实服务端，按只读视图决策打完一局：

```
sim bot --connect ws://HOST:PORT/ws --name NAME --hero CARD_ID
        [--room CODE] [--create] [--intent-seed N]
```

- `--connect`、`--name`、`--hero` 必填；`--room` 加入既有房间（与 `--create` 二选一，`--create` 时忽略 `--room`）；`--create` 建房并把房间码输出到 stdout 首行 `ROOM_CODE=<code>`；`--intent-seed` 默认 0，只用于在发现候选间做确定性选择。
- `--hero` 只是偏好：机器人收到本人 `RoomUpdate` 后从 `heroCandidates` 里选，指定的英雄不在候选内时退到候选中第一张；下发的候选为空时报错退出。
- 协议顺序固定：`AuthRequest` →（`CreateRoom` | `JoinRoom`）→ `SelectHeroCard` → `Ready` → 等 `GameStarted` → 循环按 brain 决策发送 `IntentMessage`，直到 `GameOver`。
- 每个意图使用确定性递增的 `clientIntentId`（`{name}-{counter}`）；客户端只对「自身视角有新变化」的视图做一次决策，避免重播视图导致重复意图。
- 收到 `NO_VALID_TARGET` 时按 `clientIntentId` 找回被拒意图并立即按同一视图重决策：定向出手沿服务端下发的候选（键为 `zone:id`）静默换下一个重发，遍历完仍无合法目标时把这张牌记入本回合阻塞集合并输出一行说明；`ChooseTarget` 把被拒候选记入阻塞集合并换下一个，全部候选都被拒时放弃该待选。整盘没有合法目标、本回合已放弃 3 张且可结束招募时直接 `EndRecruit`。候选本身来自服务端，正常情况下不会拒绝；出现拒绝按视图与结算偏差处理，不按错误上报。
- `bot/brain.rs` **只读** `PlayerView`（含服务端算好的 `affordances`）与静态卡表，不链接引擎判定、不读时钟与环境变量、不使用浮点；优先级固定为：消费 `pendingChoices`（发现/进化发 `ChooseDiscover`，目标待选取 `candidates` 中第一个未被拒绝的 `TargetRef`）→ 被拒出手达 3 张且可结束时 `EndRecruit` → 打最小下标且未被拒绝的可打手牌 → 买最贵可买槽位 → 升时代 → 刷新 → `EndRecruit`。出牌时若手牌视图 `requiresTarget` 为真，从该牌的 `targetCandidates` 预填第一个未被拒绝的目标；被拒后沿候选列表换下一个，候选遍历完才记入阻塞集合本回合跳过该牌。候选为空时不填（效果落空）。
- `sim/src/ai.rs` 的 `PendingChoice::Target` 分支与定向效果牌/英雄技能按 `Selector::Target { zone }` 依次尝试本人棋盘、手牌与商店的实例编号；视图驱动的新逻辑在 `bot/brain.rs`。复制手牌类效果的行为变化使 M0 金标准重新冻结（见[规则引擎与模拟器](../rule-engine-and-simulator.md) §9）。

## 13. 命令与验收证据

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

下表为 `make m1-check` 一次实际运行的结果（整体退出码 0）：

| 步骤 | 命令 | 结果摘要 | 退出码 |
| --- | --- | --- | --- |
| 协议 Rust 生成物 | `make protocol-rs-check` → `cargo run -q -p protocol-gen -- --check` | 无差异 | 0 |
| 协议生成物 | `make protocol-check` → `validate_protocol.py`、`gen_protocol_gdscript.py --check`、`gen_protocol_docs.py --check` | `协议样例校验通过`，GDScript 与字段表无漂移 | 0 |
| 卡牌数据 | `make data-check` → `gen_card_data.py --check`、`audit_effects.py` | `生成数据无漂移`、`审计通过` | 0 |
| 测试 | `make test` → `unittest discover`、`cargo test --workspace` | Python `Ran 130 tests ... OK`；workspace 全部 `test result: ok`，其中 `room-service` 集成测试含 `social`（12 个）、`social_e2e`（12 个）、`persistence`（5 个，无 `DATABASE_URL` 时跳过）、`e2e`（9 个，含 `eight_bots_finish_a_game`） | 0 |
| 静态检查 | `make lint` → `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings` | 无警告 | 0 |
| 确定性 | `make m0-determinism` → `cargo test -p sim --test sim_determinism replay_same_log_100_times_has_same_hash` | `1 passed`（约 26 s） | 0 |

持久化证据由 `make db-test` 单独提供：先 `make db-up` 启动 `postgres:16`，`tests/persistence.rs` 的 5 个用例在真实数据库上全部通过（账号身份恢复、社交数据跨重启恢复、过期数据不载入、重启后重新登录、固定时钟下邀请窗口不变）。

## 14. 与设计的偏差

1. 设计 §5.4 写 `view(state, seat, catalog) -> PlayerView`；因 `rule-engine` 不依赖 `protocol`，改为引擎返回自有 `SeatView`，由 `room-service::view::to_protocol` 做机械映射。规则判定仍全部在引擎。
2. 设计 §5.5 写「AI 决策改为同时接受 `PlayerView`」；为保住 M0 基线（`ai.rs` 的策略直接决定 1000 局基线数值），改为新增 view 驱动的 `sim/src/bot/brain.rs`，`ai.rs` 仍只读引擎状态、不接受 `PlayerView`（本需求只加了 `PendingChoice::Target` 的确定性分支与 `PlayCard.position: None`）。
3. 设计 §5.2 要求存 `tokenHash` 与 connection；实现中仍保存明文 token，但已建立 `ConnectionRegistry` 连接索引用于好友在线状态与房间外推送。令牌只存在于内存、有效期 24 小时、不落盘；`Session` 的 `Debug` 输出已把 `token` 标为 `<redacted>`，避免日志泄漏。
4. 设计把 PostgreSQL 列为账号、好友、房间元数据、意图日志与遥测的落点；本轮只落账号身份与社交数据（好友、申请、最近对局、房间邀请），房间、对局状态、意图日志与遥测仍在进程内或尚未实现。未配置 `DATABASE_URL` 时保留纯内存模式供本地开发并打印提示；配置后连不上数据库时进程直接失败，不静默降级。

## 15. 对局观察日志

- 进入 `GameOver` 时（正常结算与弃权两条路径）各打印一次单行 JSON 到 stderr；`GameTask.game_over_logged` 保证每局只打印一次。不引入指标端点，也不新增依赖。
- 字段：`service`、`event`（固定 `game_over`）、`roomId`、`seed`（16 位十六进制）、`turns`、`durationMs`（`started_at_ms` 到终局，按注入时钟计算）、`intentsBySeat`、`intentsTotal`、`rejectionsBySeat`、`rejectionsTotal`、`rejectionRatePermille`（向下取整，未有意向时为 0）、`survivors`、`winner`（座位号，排名为空时为 `null`）。
- 计数口径：首次提交的意图计 1（去重命中属于重传，不计）；任何一次拒绝都计 1，包括非招募阶段、参数非法与引擎拒绝。
- 内测期间按需从日志抄录观察值，不强制记录，也不作为验收项。

## 相关文档

- [规则引擎与模拟器](../rule-engine-and-simulator.md)
- [技术现状索引](../README.md)
- [服务端技术基线](../../../versions/v0.1/technical/architecture/server-stack.md)
- [通信协议](../../../versions/v0.1/technical/architecture/protocol.md)
