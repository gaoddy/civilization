# v0.1 客户端与服务端通信协议设计

> 状态：待确认
> 依赖：[联网与技术架构](../../versions/v0.1/technical/architecture/network-architecture.md)、[服务端技术基线](../../versions/v0.1/technical/architecture/server-stack.md)、[客户端技术基线](../../versions/v0.1/technical/architecture/client-stack.md)、[核心规则](../../versions/v0.1/product/rules/core-rules.md) §3
> 决策方式：已在对话中选定「全量可见视图」与「独立 IDL 生成两端」，本文只记录落地方案。

## 意图与原因

M1 单房间联调需要客户端与服务端按同一份契约对接。现有 [联网与技术架构](../../versions/v0.1/technical/architecture/network-architecture.md) 锁定了协议骨架（WebSocket + JSON、信封、消息清单、重连、隐藏信息），但没有字段级契约、契约来源、版本字段、鉴权与心跳规则。本文补齐这些，使两端可以各自实现并通过同一批样例验证。

## 目标

- 一份独立于语言的 IDL 作为唯一契约来源，生成 Rust 类型、GDScript 常量与字段表。
- 服务端每次结算后下发该玩家可见的完整状态视图，客户端整片替换，不做补丁合并。
- 握手鉴权、心跳、重连、背压、限流、错误码与消息版本规则可直接实现。
- 客户端不推导规则：能否买、能否三连等提示全部由服务端字段给出。

## 非目标

- 不搭建服务端或客户端工程，不写业务代码。
- 不设计数据库表、好友与邀请的具体交互、观战界面。
- 不引入二进制编码、消息压缩或分片；v0.1 保持 JSON 文本帧。
- 不改变规则本身与意图语义，只定义传输契约。

## 本轮交付范围

- 交付：`protocol/` 下的 JSON Schema、样例、生成脚本与校验脚本；`make protocol` 与 `make protocol-check`；`docs/versions/v0.1/technical/architecture/protocol.md`；相关基线文档改写。
- 不交付：`server/` 与 `client/` 工程、Cargo workspace、Godot 工程，以及写进工程目录的生成物。
- 接入点：工程建立后，`make protocol` 把生成物输出到对应工程目录；在那之前，生成物只落在仓库根的 `protocol/generated/`，Rust 类型生成在服务端 workspace 出现后启用。

## 反目标

- 客户端出现任何"本地先改状态、等服务器纠正"的乐观更新。
- 同一份契约在 Rust 与 GDScript 各手写一份。
- 靠补丁合并维持状态：出现一次乱序或丢消息就进入无法自愈的非法状态。

## 约束

- 规则只在服务端运行，客户端不加载、不执行规则引擎（联网架构硬约束 2、3）。
- 每房间单 task 串行结算；同一玩家的消息由同一 task 发出，顺序稳定。
- 序列化只用 serde + serde_json；随机数确定性要求不进入协议层。
- 隐藏信息清单（对手手牌、金币、商店、种子、意图队列）绝不下发。
- 生成物随源契约提交；契约以 IDL 为唯一来源，不手写与 IDL 不一致的字段文档。
- 兼容演进：开发期两端同批更新；上线后同一 `v` 内只增字段，不改语义。

## 方案

### 决策一：状态同步形态（选定 A）

| 方案 | 内容 | 取舍 |
| --- | --- | --- |
| A（选定） | 每次结算向每位玩家推送该玩家可见的完整状态视图，带单调 `seq` | 状态小、逻辑薄；重连与乱序天然自愈；代价是重复带宽，8 人局可忽略 |
| B | 只推变化字段，客户端合并补丁 | 省带宽；需维护 diff、合并与乱序重放，出错即静默非法状态，与"客户端不推导规则"冲突 |

### 决策二：契约单一来源（选定 C）

| 方案 | 内容 | 取舍 |
| --- | --- | --- |
| A | Rust `protocol` crate 为来源，导出 schema 再生成 GDScript | 链路短；但契约被服务端语言绑死，客户端生成物永远是二等公民 |
| B | 两端手写 + 样例测试 | 最轻；字段改名靠人眼，契约漂移不可检测 |
| C（选定） | 手写 JSON Schema（draft 2020-12）为 IDL，生成 Rust 类型、GDScript 常量与字段表 | 语言中立、可校验样例；多一层生成链路与构建目标，需要 CI 检查生成物无差异 |

C 的额外成本集中在 GDScript 侧：无成熟代码生成器，只生成常量与值对象，不做完整运行时校验；字段级校验以 Rust 端为准，客户端用同一批 JSON 样例做解析测试。

## 设计

### 1. 传输与连接

- 单条 WebSocket 承载全部消息：认证、大厅、房间、对局、好友。HTTP 只保留 `/healthz`，不引入 REST。
- 文本帧，UTF-8 JSON。单条消息 ≤ 256 KiB，不支持分片与压缩（`permessage-deflate` 关闭）。
- 连接状态：`Connected` →（`AuthRequest`）→ `Authenticated` →（`Reconnect` 或创建/加入房间）→ `InRoom` → `Closed`。
- 未认证连接 10 秒内未完成认证即断开；`Connected` 状态只接受 `AuthRequest`；`Reconnect` 只在 `Authenticated` 状态被接受，且不再重复携带凭证。

### 2. 信封

```json
{
  "v": 1,
  "id": "m_01J8Z3K4",
  "type": "StateUpdate",
  "seq": 1024,
  "ts": "2026-11-15T10:00:00Z",
  "roomId": "r_8f2a",
  "playerId": "p_03",
  "replyTo": "ci_7f3a",
  "payload": {}
}
```

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `v` | 是 | 协议主版本，整数；v0.1 固定为 1 |
| `id` | 是 | 发送方生成的唯一消息 id，用于日志与排障 |
| `type` | 是 | 唯一路由键；未知 `type` 必须忽略并记录，不断开连接 |
| `seq` | 仅 S→C | 服务端发往该玩家的消息序号，按连接单调递增，从 1 开始 |
| `ts` | 是 | RFC 3339 UTC 字符串 |
| `roomId` | 否 | 未进房间时为 `null` |
| `playerId` | 否 | 认证前为 `null` |
| `replyTo` | 否 | 回复某条上行消息时填其 `id` |
| `payload` | 是 | 由 `type` 决定的结构，按 IDL 校验 |

- `seq` 是逐玩家流，不是房间全局流：被隐藏信息过滤掉的事件不占该玩家的序号，客户端只需检测单调性，不检测连续性。
- 客户端收到 `seq ≤ 当前 seq` 的消息直接丢弃；连接存活但连续 30 秒无任何下行消息、且当前阶段的上一个 `phaseDeadline` 已过时，发 `RequestSnapshot`。`Snapshot` 不递增 `seq`，只携带当前值。
- `replyTo` 只在直接回复某条上行消息时填写：`AuthResult` 回 `AuthRequest`，`Error` 回触发错误的消息；`StateUpdate`、`Snapshot`、`PhaseChanged` 等推送不填。

### 3. 消息清单 v1

| 方向 | `type` | 用途 | 阶段 |
| --- | --- | --- | --- |
| C→S | `AuthRequest` | 手机号 + 验证码登录；断线重连时用 `token` 免验证码 | M1 |
| S→C | `AuthResult` | 返回 `playerId`、`token`、`expiresAt` | M1 |
| C→S | `CreateRoom` | 创建房间，返回 6 位房间码 | M1 |
| C→S | `JoinRoom` | 按房间码加入 | M1 |
| C→S | `LeaveRoom` | 离开房间 | M1 |
| S→C | `RoomUpdate` | 房间成员、准备状态、房主、房间码 | M1 |
| C→S | `Ready` | 准备/取消准备 | M1 |
| C→S | `SelectHeroCard` | 开局选英雄卡 | M1 |
| S→C | `GameStarted` | 对局开始，附初始 `PlayerView` | M1 |
| C→S | `Intent` | 招募阶段意图，转发给规则引擎 | M1 |
| C→S | `SkipAnimation` | 战斗播放跳过，仅表现层 | M1 |
| C→S | `RequestSnapshot` | 序号异常或主动校准 | M1 |
| C→S | `Reconnect` | 携带 `roomId`、`lastSeq`；连接已认证，不再带令牌 | M1 |
| S→C | `Snapshot` | 全量可见视图，重连与校准使用 | M1 |
| S→C | `StateUpdate` | 完整可见视图 + 本次结算产生的事件列表 | M1 |
| S→C | `PhaseChanged` | 阶段、回合与截止时间；用于计时与动画 | M1 |
| S→C | `TimerWarning` | 招募剩余 10 秒 | M1 |
| S→C | `CombatResult` | 本玩家这一对的完整战斗回放数据 | M1 |
| S→C | `GameOver` | 排名、淘汰顺序与本局种子（会战模式公开） | M1 |
| S→C | `Error` | 意图或请求被拒绝，含稳定错误码 | M1 |
| C→S/S→C | `FriendRequest` / `FriendUpdate` | 好友申请与列表 | M2 |

- `StateDelta` 更名为 `StateUpdate`：载荷是完整视图，原名称会误导实现者做补丁合并。本次同步改写联网架构 §5.2 与 §5.3。
- `PhaseChanged` 与 `StateUpdate` 有信息重叠：前者只用于计时与过场动画，客户端以 `StateUpdate.view.phase` 为真相。

### 4. 可见状态视图

`StateUpdate.payload`：

```json
{
  "view": { "...": "PlayerView" },
  "events": [{ "kind": "ShopRolled", "...": "..." }]
}
```

`PlayerView` 只在服务端构造，客户端只读：

| 字段 | 说明 |
| --- | --- |
| `roomId` / `phase` / `turn` / `phaseDeadline` | 房间与阶段；截止时间为 RFC 3339 |
| `self` | 自己的完整状态：`hp`、`era`、`gold`、`heroCardId`、`heroPower`、`shop`、`hand`、`board`、`rerollCost`、`upgradeCost`、`recruited` |
| `players` | 其他玩家的公开信息：`playerId`、`seat`、`displayName`、`hp`、`era`、`alive`、`boardCount`、`heroCardId`、`recruited` |
| `enabledFactions` | 本局启用的 5 个流派 |
| `affordances` | 服务端算好的可操作提示，见下 |

`affordances` 明确禁止客户端自行推导：

```json
{
  "shopSlots": [{"index": 0, "buyable": true, "disabledReason": null}, "..."],
  "handCards": [{"index": 0, "playable": true}],
  "boardMinions": [{"index": 0, "sellable": true}],
  "canReroll": true,
  "canUpgradeEra": false,
  "upgradeDisabledReason": "NOT_ENOUGH_GOLD",
  "canUseHeroPower": true,
  "canEndRecruit": true
}
```

- `events` 是本次结算产生的领域事件，只服务动画与音效，不作为状态来源：`MinionBought`、`MinionSold`、`ShopRolled`、`EraUpgraded`、`TripleFormed`、`CardDiscovered`、`DamageTaken`、`PlayerEliminated` 等。
- 视图不包含对手手牌、金币、商店、刷新次数、意图队列与对局种子。

### 5. 意图载荷

`Intent.payload` 用内部标签区分意图名，与 [核心规则](../../versions/v0.1/product/rules/core-rules.md) §3 一一对应：

```json
{
  "clientIntentId": "ci_7f3a",
  "intent": {"kind": "BuyMinion", "slot": 2}
}
```

| `kind` | 参数 |
| --- | --- |
| `BuyMinion` | `slot` |
| `SellMinion` | `minionId` |
| `RerollShop` | 无 |
| `UpgradeEra` | 无 |
| `PlayCard` | `handIndex` |
| `RearrangeBoard` | `fromIndex`、`toIndex` |
| `FreezeShop` | `slot` 或 `null` 表示全部 |
| `UseHeroPower` | `skillId`、`targetMinionId?` |
| `EndRecruit` | 无 |

- 同一 `clientIntentId` 只处理一次；重复到达时服务端回原结果（成功则不重复结算，失败则重复 `Error`）。
- 去重表只存在于房间进程内，按房间生命周期保留；对局结束、房间销毁即释放，不持久化。重连后重发未收到回复的意图，仍按原结果返回。
- 意图被拒绝时返回 `Error{code, clientIntentId}`；客户端无本地状态可回滚，仅结束该次操作的即时反馈。

### 6. 鉴权、心跳与重连

- 认证：`AuthRequest{phone, code}` → `AuthResult{playerId, token, expiresAt}`；重连改用 `AuthRequest{token}`。`token` 为不透明随机串，服务端只存哈希，有效期 30 天。
- 同一账号最多 2 条活跃连接；超过时服务端关闭最早的一条并附 `Error{code: SESSION_REPLACED}`。
- 心跳：服务端每 15 秒发 WebSocket ping 帧，45 秒未收到 pong 关闭连接（1001）。不定义应用层心跳消息。
- 重连：客户端先 `AuthRequest{token}` 完成认证，再 `Reconnect{roomId, lastSeq}` → `Snapshot`。因为每次都是全量视图，不补发历史事件；客户端清空本地状态后载入快照。
- 断线 30 秒进入托管，规则见联网架构 §6.3，协议层不新增消息。

### 7. 背压、限流与消息上限

| 项 | 规则 |
| --- | --- |
| 单条消息 | ≤ 256 KiB；超限回 `Error{BAD_PAYLOAD}` 并关闭连接 1009 |
| 发送队列 | 每连接 256 条或 2 MiB，超限关闭连接 1013；客户端按指数退避重连 |
| 意图速率 | 每秒 20 条、突发 40 条；超出丢弃该条并回 `Error{RATE_LIMITED}`，不累计封禁 |
| 认证速率 | 每 IP 每分钟 10 次 `AuthRequest`，超出回 `AUTH_RATE_LIMITED` |
| 连接数 | 每账号 2 条，每 IP 20 条 |

### 8. 契约来源与生成

- IDL 目录：仓库根 `protocol/`
  - `schema/v1/*.schema.json`：JSON Schema draft 2020-12，按消息与共享类型分文件。
  - `schema/v1/messages.json`：`type` → 上行/下行 schema 的映射表。
  - `samples/v1/*.json`：合法样例，两端测试共用。
- 生成物（本轮随源提交，放在 `protocol/generated/`）：
  - `protocol/generated/protocol.gd`：由 `scripts/gen_protocol_gdscript.py` 生成，含消息类型、意图名、错误码、阶段与事件名的常量，以及按 schema 生成的 `PlayerView` 字段名常量。客户端工程建立后由 `make protocol` 输出到 `client/scripts/protocol/`。
  - `docs/versions/v0.1/technical/architecture/protocol-messages.md`：由 `scripts/gen_protocol_docs.py` 从 schema 生成的字段表；人工维护的 `protocol.md` 只写规则与链接，不复制字段。
  - `protocol/generated/protocol.rs`：由 `typify` 从 schema 生成；服务端 Cargo workspace 建立后启用并提交，本轮只保留生成命令与说明，不产出该文件。
- 构建入口：`make protocol` 重新生成全部生成物；`make protocol-check` 重新生成后比较工作区，差异即失败。
- 客户端不实现完整运行时 schema 校验：解析样例与真实消息时只检查 `type`、`v` 与关键字段存在性；字段级契约由 Rust 端与样例校验保证。

### 9. 版本演进

- `v` 为协议主版本，v0.1 固定 `1`。
- v0.1 正式发布前属于开发期：允许直接改动 `v: 1` 的字段与类型，两端同批提交；不实现 `v: 2` 路由与双序列化器。
- 同一 `v` 内只允许新增可选字段与新增 `type`；客户端必须忽略未知字段与未知 `type`。
- 上线后的改名、删除、改语义或改变字段类型走 `v: 2`，服务端按 `v` 路由序列化器；不做同一连接内的版本协商。
- 错误码只增不改；码值语义变更视为破坏性变更。

### 10. 错误码

| 类别 | 码 | 说明 |
| --- | --- | --- |
| 规则（来源：核心规则 §3） | `NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY`、`NO_SPACE`、`INVALID_STAGE`、`NO_SUCH_MINION`、`ALREADY_MAX_ERA`、`CARD_NOT_IN_HAND`、`HERO_POWER_LIMIT`、`NO_VALID_TARGET`、`TARGET_NOT_ALLOWED`、`INVALID_CHOICE` | 意图被拒绝 |
| 协议 | `AUTH_FAILED`、`TOKEN_EXPIRED`、`AUTH_RATE_LIMITED`、`SESSION_REPLACED` | 认证 |
| 协议 | `BAD_PAYLOAD`、`UNKNOWN_TYPE`、`RATE_LIMITED`、`NOT_IN_ROOM`、`ROOM_NOT_FOUND`、`ROOM_FULL`、`SEQUENCE_STALE`、`INTERNAL_ERROR` | 请求与传输 |

- 错误码枚举写入 `protocol/schema/v1/errors.schema.json`，Rust 端用 `match` 穷尽；新增错误码必须同时更新该文件与核心规则的返回码表。

### 11. 测试与验证

| 测试 | 内容 |
| --- | --- |
| 样例校验 | `protocol/samples/v1/` 全部通过 JSON Schema 校验 |
| Rust 反序列化 | 全部样例可解析；未知字段忽略；未知 `type` 忽略 |
| GDScript 解析 | 同一批样例可解析并构建视图与可操作提示 |
| 生成一致性 | `make protocol-check` 无差异 |
| 乱序与重复 | `seq ≤ 当前` 的消息被丢弃；序号停滞触发 `RequestSnapshot` |
| 重连一致 | 在任意事件点断线重连，客户端视图与服务端视图逐字段一致 |
| 错误码穷尽 | 每个 `Error.code` 都有对应的处理分支，无未处理枚举值 |

## 需要同步改写的文档

- [联网与技术架构](../../versions/v0.1/technical/architecture/network-architecture.md)：§5.1 信封补 `v`、`id`、`replyTo` 与 `ts` 格式；§5.2 `StateDelta` 改为 `StateUpdate` 并改语义；§5.3 序列示例同步；§5.4 补重复意图的结果说明；§6.2 重连改为直接返回 `Snapshot` 不补发增量。
- [服务端技术基线](../../versions/v0.1/technical/architecture/server-stack.md)：工程形态补 `protocol/` IDL 目录与生成链路。
- [客户端技术基线](../../versions/v0.1/technical/architecture/client-stack.md)：补 GDScript 生成物位置与"无完整运行时校验"的边界。
- [待确认事项](../../versions/v0.1/open-questions.md)：新增或收敛令牌有效期、IDL 目录落点、GDScript 生成器维护成本三项。
- 新增 `docs/versions/v0.1/technical/architecture/protocol.md`（人工规则）与 `protocol-messages.md`（生成字段表）。

## 留给实现者

- JSON Schema 的文件切分粒度与命名；生成器的具体实现与 CI 接入方式。
- `PlayerView` 内领域对象（随从、技能、商店槽位）的完整字段清单，按 [卡牌规范与数据](../../versions/v0.1/product/content/card-system.md) 与前端渲染需要确定。
- `CombatResult` 的动作序列字段结构，需覆盖 [核心规则](../../versions/v0.1/product/rules/core-rules.md) §5 的每种动作类型。
- 好友与邀请消息的字段结构，M2 前补齐。
- 令牌存储、短信服务商与验证码有效期属服务端实现，不在协议内。

## 未决事项

| 问题 | 影响 | 处理 |
| --- | --- | --- |
| GDScript 生成器是否值得长期维护 | 生成链路复杂度 | 本轮先做最小生成物（常量 + 字段名），不做 `$ref` 解析与完整校验；若维护成本高于收益，退回手写常量并由样例测试兜底 |
| 是否需要 `protocol-messages.md` 自动生成 | 文档与契约一致性 | 实现时先手写字段表，生成器稳定后再自动化 |
| 观战只读视图的字段裁剪 | v0.1 不做观战，但结构需预留 | 视图按"可被第三方只读消费"设计，字段裁剪规则待观战时确定 |
