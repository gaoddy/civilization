# 通信协议

> 状态：方案未实施
> 依赖：[联网与技术架构](network-architecture.md)、[核心规则](../../product/rules/core-rules.md) §3、[协议设计](../../../../superpowers/specs/2026-11-15-protocol-design.md)、[协议字段表](protocol-messages.md)

本文定义 v0.1 客户端与服务端的传输契约、消息规则与兼容演进；字段级清单由 schema 生成，见[协议字段表](protocol-messages.md)。唯一契约来源是 `protocol/schema/v1/`。

## 1. 连接生命周期

- 单条 WebSocket 承载认证、大厅、房间、对局与好友消息；HTTP 只保留 `/healthz`。
- 连接状态：`Connected` →（`AuthRequest`）→ `Authenticated` →（`Reconnect` 或创建/加入房间）→ `InRoom` → `Closed`。
- 未认证连接 10 秒内断开；`Connected` 只接受 `AuthRequest`；`Reconnect` 只在 `Authenticated` 被接受。
- 文本帧、UTF-8 JSON；单条 ≤ 256 KiB；不分片、不压缩。

## 2. 信封与序号

- 信封字段：`v`、`id`、`type`、`ts`、`payload` 必填；`roomId`、`playerId`、`replyTo` 可选；`seq` 仅下行必填。
- `v` 固定为 `1`；`ts` 为 RFC 3339 UTC；`type` 是唯一路由键，未知类型忽略并记录，不断开连接。
- `seq` 是逐玩家流，从 1 开始单调递增；被隐藏信息过滤的事件不占序号，客户端只检测单调性。
- 客户端丢弃 `seq ≤ 当前值` 的消息；连接存活但连续 30 秒无下行消息且阶段截止已过时，发 `RequestSnapshot`。`Snapshot` 不递增 `seq`。
- `replyTo` 只用于直接回复：`AuthResult` 回 `AuthRequest`，`Error` 回触发错误的消息。

## 3. 消息与方向

- 上行 11 个类型、下行 11 个类型，清单见[协议字段表](protocol-messages.md)与设计文档 §3。
- 规则只在服务端运行：客户端不加载规则引擎、不做本地预演、不推断规则结论。
- 战斗只播放 `CombatResult`；播放速度不影响结算。

## 4. 可见状态与可操作提示

- 每次结算后服务端下发该玩家可见的完整 `PlayerView`，客户端整片替换，不做补丁合并。
- `PlayerView` 只含自己的完整状态与其他玩家的公开信息；对手手牌、金币、商店、刷新次数、意图队列与种子绝不下发。
- 能否购买、能否打出、能否升级等结论由服务端在 `affordances` 中给出，客户端不得自行推导。
- `events` 只服务动画与音效，不作为状态来源。

## 5. 意图与幂等

- `Intent` 载荷为 `{clientIntentId, intent}`，`intent.kind` 对应[核心规则](../../product/rules/core-rules.md) §3 的十种意图。
- 同一 `clientIntentId` 只处理一次；重复到达返回原结果，不重复结算。
- 去重表只存在于房间进程内，按房间生命周期保留，不持久化；重连后重发未收到回复的意图仍返回原结果。
- 意图被拒绝时返回 `Error{code, clientIntentId}`；客户端没有可回滚的本地状态。

## 6. 鉴权、心跳与重连

- `AuthRequest{phone, code}` 登录，`AuthRequest{token}` 免验证码；`AuthResult` 返回 `playerId`、`token`、`expiresAt`。
- 令牌为不透明随机串，服务端只存哈希，有效期 30 天；同一账号最多 2 条活跃连接，超出关闭最早一条并返回 `SESSION_REPLACED`。
- 心跳使用 WebSocket ping 帧：服务端每 15 秒发送，45 秒无 pong 关闭连接（1001）；不定义应用层心跳消息。
- 重连：先认证，再 `Reconnect{roomId, lastSeq}`，服务端直接返回 `Snapshot`，不补发历史事件。
- 断线 30 秒进入托管，规则见[联网与技术架构](network-architecture.md) §6.3。

## 7. 背压、限流与消息上限

| 项 | 规则 |
| --- | --- |
| 单条消息 | ≤ 256 KiB；超限回 `BAD_PAYLOAD` 并以 1009 关闭 |
| 发送队列 | 每连接 256 条或 2 MiB，超限以 1013 关闭 |
| 意图速率 | 每秒 20 条、突发 40 条；超出丢弃并回 `RATE_LIMITED` |
| 认证速率 | 每 IP 每分钟 10 次，超出回 `AUTH_RATE_LIMITED` |
| 连接数 | 每账号 2 条、每 IP 20 条 |

## 8. 契约来源与生成

- 唯一来源：`protocol/schema/v1/*.schema.json`（JSON Schema draft 2020-12）与 `messages.json`。
- 生成物：`protocol/generated/protocol.gd`（GDScript 常量）与[协议字段表](protocol-messages.md)；生成物随源提交。
- 命令：`make protocol` 重新生成；`make protocol-check` 校验样例并检测生成物漂移。
- 客户端不做完整运行时校验，只检查 `type`、`v` 与关键字段存在性；字段级契约由服务端与样例校验保证。

## 9. 版本演进与兼容

- v0.1 发布前属于开发期：允许直接改动 `v: 1` 的字段与类型，两端同批提交，不实现 v2 路由。
- 同一 `v` 内只允许新增可选字段与新增 `type`；客户端必须忽略未知字段与未知类型。
- 上线后的改名、删除、改语义或改类型走 `v: 2`；错误码只增不改。

## 10. 错误码

- 规则错误码与[核心规则](../../product/rules/core-rules.md) §3 的返回码一致；协议错误码覆盖认证、载荷、限流、房间与内部错误。
- 完整枚举见 `protocol/schema/v1/errors.schema.json` 与[协议字段表](protocol-messages.md)，客户端 `match` 必须穷尽。

## 11. 验证

| 测试 | 内容 |
| --- | --- |
| 样例校验 | `protocol/samples/v1/` 全部通过 schema 校验，且每个消息类型都有样例 |
| 生成一致性 | `make protocol-check` 无差异 |
| 乱序与重复 | `seq ≤ 当前` 的消息被丢弃；重复 `clientIntentId` 返回原结果 |
| 重连一致 | 任意事件点重连后，客户端视图与服务端视图逐字段一致 |

## 相关文档

- [协议字段表](protocol-messages.md)
- [联网与技术架构](network-architecture.md)
- [协议设计](../../../../superpowers/specs/2026-11-15-protocol-design.md)
- [核心规则](../../product/rules/core-rules.md)
