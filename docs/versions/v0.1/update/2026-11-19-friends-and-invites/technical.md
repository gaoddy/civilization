# 好友与邀请：技术方案

> 状态：已实施
> 归属版本：v0.1
> 产品：[product.md](product.md)

## 1. 边界

- 服务端新增 `room-service/src/social.rs`：好友关系、申请、最近对局与房间邀请的规则与缓存，写操作经 `db.rs` 落 PostgreSQL。
- 服务端新增 `room-service/src/account.rs`：手机号 → 稳定账号（`playerId`、玩家编号、展示名），同一手机号重新登录恢复同一账号。
- 服务端新增 `room-service/src/db.rs` 与 `server/migrations/0001_baseline.sql`：连接池、基线迁移与唯一 SQL 落点；业务模块不拼 SQL。
- `room-service/src/auth.rs` 增加 6 位玩家编号与账号前置解析；`net.rs` 增加连接登记与好友消息路由；`room.rs` 只增加按玩家查房间的查询，不改房间状态机。
- 协议在 `protocol/schema/v1/` 增量定义，Rust 与 GDScript 生成物随源提交。
- 客户端新增社交状态与大厅好友面板、房间邀请入口、独立 `InvitePopup` 组件。
- 持久化范围：账号身份、好友关系、好友申请与发送记录、最近对局、房间邀请；会话（token）仍是进程内状态，重启后用同手机号重新登录。

## 2. 玩家编号

- `Session` 增加 `player_code: String`；编号由 `account` 表分配：Postgres 序列 `player_code_seq` 从 `100000` 起循环，`player_code` 唯一约束兜底（90 万个编号用尽后写入失败并返回 `INTERNAL_ERROR`，v0.1 不预期触达）。
- 同一手机号重新登录复用同一 `player_id` 与编号；编号不对其它账号复用。
- `AuthResult` 增加必填 `playerCode`；`playerId` 仍是不透明内部标识。
- 好友搜索、展示与邀请都使用 `playerCode`，协议不要求客户端持有手机号。
- 默认展示名 `玩家N` 的序号由 `account.ordinal`（持久化自增列）生成，重启不回退。

## 3. 社交存储

`SocialStore` 由 `AppState.social: Arc<Mutex<SocialStore>>` 持有，单把锁；内存是数据库的缓存，写操作先落库、成功后再改内存，写入失败返回 `INTERNAL_ERROR` 且不改内存。未配置 `DATABASE_URL` 时退化为纯内存模式（本地开发与既有测试）。

| 数据 | 结构 | 上限/有效期 |
| --- | --- | --- |
| 资料 | `player_id → { player_code, display_name }` | 来自 `account` 表，登录时载入并进程内缓存 |
| 好友 | `player_id → BTreeSet<player_id>` | 每人 100 |
| 待处理申请 | `player_id → BTreeMap<from, { sent_at_ms }>` | 每人 20，7 天失效 |
| 已发申请 | `player_id → BTreeMap<to, { sent_at_ms, reject_at: Vec<u64> }>` | 拒绝后 24 小时最多 3 次 |
| 最近对局 | `player_id → Vec<player_id>` | 最近 20 局，去重、新近在前 |
| 邀请 | `invite_id → { from, to, room_id, room_code, expires_at_ms }` | 60 秒 |

规则落点：

- 不能加自己、已是好友、已有任一方向待处理申请都拒绝。
- 同意后双方好友集合同时插入，删除双方申请记录；拒绝只删除并记录拒绝时刻。
- 删除好友双向立即解除。
- `record_match` 在开局成功时把同局其他玩家并入每个座位的最近对局列表。
- 所有读取入口先清理过期申请与邀请；M2 不跑后台清理任务。

## 4. 在线状态

`FriendStatus` 三值：

- `inGame`：玩家在某个房间且阶段为 `Recruit` / `Combat`；
- `online`：连接登记表里存在该玩家，且不在对局中；
- `offline`：其余。

好友列表排序：`online` → `inGame` → `offline`，同状态按 `display_name` 首字，再按 `player_id`。

## 5. 连接登记与下行

- `AppState.connections: Arc<Mutex<BTreeMap<player_id, ConnectionEntry>>>`；`ConnectionEntry` 含连接代次、控制通道发送端、下行发送端与会话级 `SeqFloor`。
- 连接 task 在读循环增加 `tokio::select!` 分支消费控制通道；控制消息仍走该连接自己的 `SeatChannel` 与 `SeqFloor`，因此同一连接只有一个直接写者。
- 社交下行统一经 `send_social(state, player_id, kind, payload)`：
  - 玩家在房间中：投递 `RoomMessage::Push { player_id, kind, payload }`，由房间 task 用该座位的下行发送端与 `SeqFloor` 发出；
  - 不在房间：投递连接控制通道。
- 连接登记在认证成功后写入，连接关闭时按代次摘除；摘除后向好友推送状态变化。
- 认证成功后向本人推送一次 `FriendUpdate`；好友关系、申请、最近对局或在线状态变化后，向受影响玩家推送新的 `FriendUpdate`。

## 6. 协议增量

上行：

| 类型 | 载荷 | 说明 |
| --- | --- | --- |
| `FriendRequest` | `{ toPlayerCode }` | 按 6 位编号发送申请 |
| `FriendRespond` | `{ fromPlayerId, accept }` | 同意/拒绝申请 |
| `FriendRemove` | `{ friendId }` | 删除好友 |
| `InviteToRoom` | `{ friendId }` | 邀请好友加入自己所在房间 |
| `RespondInvite` | `{ inviteId, accept }` | 接受/拒绝邀请 |

下行：

| 类型 | 载荷 | 说明 |
| --- | --- | --- |
| `FriendUpdate` | `{ friends, incoming, outgoing, recent }` | 全量社交视图；字段项见 schema |
| `FriendInvite` | `{ inviteId, fromPlayerId, fromPlayerCode, fromDisplayName, roomCode, expiresAt }` | 发给被邀请人 |
| `FriendInviteResult` | `{ inviteId, accepted, reason }` | 发给邀请人 |
| `InviteAccepted` | `{ inviteId, roomCode }` | 发给被邀请人，客户端随后发 `JoinRoom` |

- `FriendUpdate.friends[]`：`{ playerId, playerCode, displayName, status }`，`status` 为 `online|inGame|offline`。
- `FriendUpdate.incoming[]` / `outgoing[]`：`{ playerId, playerCode, displayName, sentAt }`。
- `FriendUpdate.recent[]`：`{ playerId, playerCode, displayName, relation }`，`relation` 为 `none|friend|incoming|outgoing`。
- 邀请接受后由客户端发既有 `JoinRoom`，服务端不跨连接改房间状态；满房与已开局仍由 `JoinRoom` 的既有错误返回。
- 新增错误码：`FRIEND_NOT_FOUND`、`FRIEND_ALREADY`、`FRIEND_REQUEST_PENDING`、`FRIEND_LIST_FULL`、`FRIEND_REQUEST_FULL`、`FRIEND_REQUEST_LIMIT`、`NOT_FRIENDS`、`FRIEND_OFFLINE`、`FRIEND_BUSY`、`INVITE_NOT_FOUND`、`INVITE_EXPIRED`。

## 7. 邀请时序

1. 邀请人 `InviteToRoom{friendId}`。服务端校验好友、被邀请人在线、邀请人在大厅房间且未满，生成 60 秒邀请，向被邀请人推 `FriendInvite`，向邀请人回成功（无额外消息，或推送 `FriendUpdate` 不变）。
2. 被邀请人 `RespondInvite{inviteId, accept}`。
3. 接受：校验未过期且房间仍可加入，回 `InviteAccepted{inviteId, roomCode}`，并向邀请人推 `FriendInviteResult{accepted:true}`；客户端收到后发 `JoinRoom{code}`。
4. 拒绝：删除邀请，向邀请人推 `FriendInviteResult{accepted:false, reason}`。
5. 过期：读取时清理，不主动推送；客户端按 `expiresAt` 自行禁用。

邀请失败原因使用稳定错误码，邀请人看到的文案由 `error.<CODE>` 文本表提供。

## 8. 客户端

- `GameStore` 增加 `player_code`、`friends`、`friend_requests_in`、`friend_requests_out`、`recent_players`、`incoming_invites` 与 `social_changed`、`invite_received`、`invite_accepted` 信号；只搬运服务端字段。
- 大厅：右侧好友面板，含好友列表（状态、删除）、待处理申请（同意/拒绝）、按 6 位编号添加、最近对局添加入口；顶部显示本人 6 位编号。
- 房间：新增「邀请好友」入口，列出当前在线且不在对局中的好友，点击发 `InviteToRoom`；结果显示在状态行。
- 邀请弹层：独立组件 `InvitePopup`（`client/scenes/invite_popup.gd`），自己订阅 `social_changed`、按 `pending_invites` 渲染并发送 `RespondInvite`；大厅挂载它并只负责状态文案。收到 `InviteAccepted` 后由 `GameStore` 发 `JoinRoom`。
- 文本表 `client/text/zh_CN.csv` 增加社交文案与新增错误码文案；协议常量由 `make protocol` 重新生成。

## 9. 兼容与限制

- 协议只增不改；`FriendRequest` 的 `toPlayerId` 改为 `toPlayerCode`（该消息此前未被实现，无在线兼容负担）。
- 单实例运行：数据库已可用，但多实例下的推送仍依赖单进程连接登记；部署编排在 M2 前单独确定。
- 新增依赖 `sqlx`（`runtime-tokio-rustls` + `postgres` + `chrono` + `migrate`）；本地数据库用 `docker compose` 的 `postgres:16`。
- 会话不落盘：进程重启后 token 失效，客户端需用同手机号重新登录；账号身份与好友数据不丢。

## 10. 验证

- `make protocol-rs-check`、`make protocol-check`：生成物无漂移，样例通过（退出 0）。
- `make test`：新增 `room-service/tests/social.rs`（12 个），覆盖编号分配、申请/同意/拒绝/删除、上限与过期、最近对局、在线状态排序、邀请成功/拒绝/过期/非好友/离线/满房；`tests/social_e2e.rs`（12 个）逐条覆盖 WebSocket 邀请分支；`Ran 130 tests ... OK` 与 workspace 全部通过（退出 0）。
- `make db-test`（先 `make db-up`）：`tests/persistence.rs` 在真实 PostgreSQL 上 5/5 通过，验证账号身份恢复、好友/申请/最近对局/邀请跨重启恢复、过期数据不载入、WebSocket 重启后重新登录（退出 0）。
- `make lint`：`cargo fmt --check` 与 `cargo clippy -D warnings`，无警告（退出 0）。
- `make client-check`、`make client-smoke`：协议/数据/文本表无漂移，新增好友面板与 `InvitePopup` 冒烟场景通过（退出 0）。
- `make m1-check`：既有 M1 验收不回归（退出 0）。

## 11. 实现偏差（2026-11-19）

- 邀请弹层已拆为独立组件 `InvitePopup`，但只在大厅挂载；房间页仍只提供邀请入口与状态行结果，不展示收到的邀请（被邀请人按校验不会同时处于房间中）。
- 拒绝邀请时给邀请人与被邀请人各发一条 `FriendInviteResult{accepted:false}`，被邀请人据此收起弹窗，避免客户端本地兜底。
- 重连成功后补发一次 `FriendUpdate`，走房间通道与快照共用序号；认证时的首份社交视图可能落在已断开的旧连接上。
- 端到端用例落在 `tests/social_e2e.rs`：好友申请/同意/删除，邀请成功、非好友、未在房间、离线、对方在房、满房、已开局、接受方开局后/满房/房间已散、过期与未知邀请、拒绝后不可复用。
- 冒烟用例为 `client/tests/ui_smoke_social.*`。
- 启动顺序为「先建库连接并迁移 → 载入账号与社交 → 绑定端口」；数据库不可用时进程直接失败，不静默降级（仅未配置 `DATABASE_URL` 时使用内存模式并打印提示）。

## 12. 持久化与账号身份（2026-11-19 追加）

### 表

| 表 | 主键 | 内容 | 约束与索引 |
| --- | --- | --- | --- |
| `account` | `account_id` | 手机号、玩家编号、展示名、持久化序号、创建/更新时间 | `phone` / `player_code` 唯一，编号 6 位数字检查，序号自增唯一 |
| `friendship` | `(player_id, friend_id)` | 双向好友关系 | 两端外键、禁止自关联；`friend_id` 索引 |
| `friend_request` | `(from_account_id, to_account_id)` | 待处理申请与发送时刻 | 禁止自关联；收件方 + 时间索引 |
| `friend_request_attempt` | `friend_request_attempt_id` | 24 小时发送次数记录 | 申请人 + 收件方 + 时间索引 |
| `recent_opponent` | `(player_id, opponent_id)` | 最近对局对手与时间 | 禁止自关联；`player_id` + 时间索引 |
| `room_invite` | `invite_id` | 房间码、双方、过期时刻 | 收件方索引、过期时刻索引 |

- 迁移形式：项目处于开发阶段，直接维护基线 `server/migrations/0001_baseline.sql`；表文档见 `docs/current/technical/room-service/models/`。
- 时间列一律 `TIMESTAMPTZ(3)`，值由服务端 `Clock` 传入，不使用 `now()`。

### 写穿与载入

- `account.rs`：登录先查账号缓存，未命中再按手机号查库/建号；`AccountStore::load` 在启动时全量载入账号。
- `social.rs`：申请、处理申请、删除好友、记录对局、创建/响应邀请全部先落库再改内存；跨表写入（申请 + 发送记录、好友 + 申请 + 发送记录）在同一事务内完成。
- `SocialStore::load` 启动时重建好友、申请、发送记录、最近对局与邀请，并清理过期申请/记录/邀请。
- 邀请 id 改为 `inv_<uuid>`，跨重启与多实例不依赖进程内自增序号。

### 测试

- `tests/persistence.rs` 为每个用例创建独立 schema（`CREATE SCHEMA` + `search_path`），在真实 PostgreSQL 上验证重启恢复；未设置 `TEST_DATABASE_URL` / `DATABASE_URL` 时打印跳过原因并返回。
- `make db-up` / `db-test` / `db-reset` 封装 Docker 起库、跑持久化测试与重建库。
