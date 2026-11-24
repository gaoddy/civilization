# 天梯与匹配技术方案

> 状态：已实现（v0.2 自动匹配、天梯积分、排行榜与个人主页）
> 关联：[天梯与匹配](../product/systems/ranked-and-matchmaking.md)、[服务端技术基线](../../../current/technical/architecture/server-stack.md)、[协议总览](../../../current/technical/architecture/protocol.md)、[客户端技术基线](../../../current/technical/architecture/client-stack.md)

本文记录 v0.2 自动匹配与天梯的落点、契约、数据模型与验证方式；产品规则以[天梯与匹配](../product/systems/ranked-and-matchmaking.md)为准，本文不重复数值。

## 1. 边界

- 只在会战模式的自动匹配局生效；房间局不计分，行为与 v0.1 一致。
- 单实例内存匹配队列；不做跨实例、跨区与赛季。
- 不做 AI 托管、断线特殊处理与主动退出结算。

## 2. 协议

新增 C→S：

| type | 载荷 | 说明 |
| --- | --- | --- |
| `JoinQueue` | `{}` | 已登录且不在房间时入队；已在房间返回错误码 |
| `LeaveQueue` | `{}` | 取消排队；不在队列时幂等 |
| `RequestLeaderboard` | `{}` | 取全服榜前 100 |
| `RequestProfile` | `{playerId?}` | 缺省为本人；非本人仅好友可看最近 10 局 |

新增 S→C：

| type | 载荷 | 说明 |
| --- | --- | --- |
| `QueueState` | `{waitingMs, waitingPlayers}` | 入队时下发本人快照；此后队列人数变化时下发 |
| `MatchFound` | `{roomId, seat}` | 成局后逐人下发，客户端随后走既有房间/开局消息 |
| `Leaderboard` | `{entries: [{rank, displayName, tier, rating}]}` | 实时查询 |
| `PlayerProfile` | `{playerId, displayName, rating, tier, highestTier, recent: [...]}` | 本人与好友可见最近 10 局 |

扩展既有消息：

- `AccountView` 增加 `rating`、`tier`、`highestTier`。
- `GameOver` 增加 `ratingDelta`、`ratingAfter`、`tierBefore`、`tierAfter`；房间局四项为 `null`。

错误码：`QUEUE_ALREADY`、`QUEUE_IN_ROOM`、`QUEUE_NOT_JOINED`。非好友查看个人主页只隐藏最近战绩，不返回错误。

## 3. 服务端

### 3.1 匹配队列

- 新模块 `server/crates/room-service/src/queue.rs`：`MatchQueue` 保存 `player_id → QueueEntry { session, joined_at_ms, recent_opponents }`。
- 由既有服务端 tick 驱动，约每秒尝试一次成局。
- 队列满 8 人即可成局，积分差不是成局条件；选人时优先排除最近 3 局对手，本人等待满 60 秒后放宽。
- 选人确定性：以等待最久（再按 `player_id`）的玩家为锚点，候选取积分差从小到大、再按入队时间与 `player_id` 排序；先贪心，凑不满时对同一锚点做确定性回溯，存在可行组合即能成局。
- 入队玩家断线或进入房间时移出队列；取消排队不扣分。

### 3.2 匹配局建房

`RoomRegistry` 新增 `create_match(sessions: [Session; 8], catalog) -> Room`：

- 服务端直接建 8 人满座房间，跳过大厅准备流程；座位按入队顺序分配。
- 房间新增 `ranked: bool` 字段；房间局为 `false`，匹配局为 `true`。
- 建房后立即给 8 人分配英雄候选，并复用 `start_heroes` + `commit_start` 推进到 `Recruit`；对局驱动沿用现有 `GameTask`。
- `create_match` 不要求房主；房间 `host_id` 记为入队第一人，但不暴露房主操作（无邀请、无踢人、无解散）。
- `RoomRegistry::create` 的既有行为不变。

### 3.3 积分结算

- 纯算法放 `server/crates/rule-engine/src/rating.rs`：输入 8 人开局积分与名次，输出每人四舍五入后的整数变化；不补差。
- 个人总分下限在持久化层应用：`after = max(0, before + delta)`，`ratingDelta` 记录实际入账值。
- 只对 `ranked == true` 的对局结算；`GameOver` 前写入，失败时整局不计分并记录错误日志。

### 3.4 段位配置

- 配置项：K、最近对手放宽时长、段位区间、宗师名次上限。
- 服务端启动时从环境变量 `RANKED_CONFIG` 读取 JSON；未设置时使用代码内默认值（与产品文档一致）。
- 解析失败直接启动失败，不降级到默认值。
- 配置随 Helm values 注入；调整后需要重新发布使新配置生效。段位不落库，每次按当前配置与积分计算；历史最高段位按历史最高积分与当前配置计算，宗师还要求账号历史上进入过宗师榜快照。

### 3.5 排行与个人主页

- 全服榜：`account_rating` 按 `rating DESC, updated_at ASC, account_id ASC` 取前 100。
- 宗师判定：每日 00:00（服务器时区）写入 `master_snapshot` 前 500 名；判定使用最近一次快照。
- 个人主页：最近 10 局来自 `ranked_result`，前 4 名比例按其中 `placement <= 4` 计算；本人与好友可见。

## 4. 数据模型

开发阶段直接修改 `server/migrations/0001_baseline.sql`，新增：

| 表 | 关键列 | 用途 |
| --- | --- | --- |
| `account_rating` | `account_id` 主键、`rating`、`highest_rating`、`updated_at` | 当前积分、历史最高积分与同分排序时间 |
| `ranked_match` | `match_id` 主键、`room_id`、`finished_at`、`seed` | 天梯对局 |
| `ranked_result` | `(match_id, account_id)` 主键、`placement`、`rating_before`、`rating_delta`、`rating_after` | 每人结算与最近 10 局 |
| `master_snapshot` | `(snapshot_day, account_id)` 主键、`rank` | 宗师榜每日快照 |

- `rating` 与 `highest_rating` 有 `>= 0` 约束；`placement` 限定 1–8。
- 新账号首次结算时插入 `account_rating`，初始 1000。
- 内存存储模式（未配置 `DATABASE_URL`）下积分只保存在进程内，重启丢失；仅用于本地开发。

## 5. 客户端

- 大厅增加「匹配」入口与排队浮层：显示等待时长、已凑人数与取消；未登录置灰。
- 收到 `MatchFound` 后按现有房间/开局消息进入对局。
- 大厅显示当前段位与积分；结算页在段位变化时显示晋级或降级提示。
- 新增排行榜页与个人主页：`UINavigation` 登记路径；排行榜只展示昵称、段位与积分，个人主页按可见性展示最近 10 局。
- 客户端不做积分与段位计算，只消费服务端字段。

## 6. 可观测性

- 日志记录成局等待时长、积分跨度、参与人数；结算记录 `match_id`、`rating_delta` 总和、被下限截断的人数。
- 排行榜查询与宗师快照失败记错误日志，不影响对局结算。

## 7. 验证

- 规则引擎单测覆盖产品文档 §3.3 两个示例与总分下限示例。
- 队列单测覆盖积分差不阻止成局、7/8 人边界、最近对手排除与放宽、回溯保底与确定性选人。
- 房间服务测试覆盖 8 人成局、房间局不计分、断线重连后座位与状态一致。
- 持久化测试覆盖积分写入、最近 10 局、排行榜排序与宗师快照。
- 客户端 `make client-check` 与相关冒烟场景；排行榜与个人主页在手机档位下不裁切。
- 端到端：8 个真实 WebSocket 客户端匹配成局并开启对局（`tests/matchmaking.rs`）；整局结算由规则引擎单测与持久化测试覆盖。尚无「8 客户端打完整局并断言积分结算」的端到端用例。

## 8. 未做

- 赛季、积分重置、段位奖励、好友榜、组队排队、举报与处罚、观战。
- AI 托管、断线特殊处理、主动退出结算。
- 多实例匹配与跨区匹配。
