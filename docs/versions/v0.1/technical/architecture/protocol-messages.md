# 协议字段表 v1

> 本文件由 `scripts/gen_protocol_docs.py` 从 `protocol/schema/v1/` 生成，不要手改。
> 规则与消息语义见 [protocol.md](protocol.md)。

## 上行消息（C→S）

### AuthRequest

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `(根)` | oneOf | 是 | 下列分支任选其一 |
| `分支1.phone` | string | 是 | 手机号 |
| `分支1.code` | string | 是 | 短信验证码 |
| `分支1.displayName` | string | 否 | 缺省由服务端生成「玩家N」 |
| `分支2.token` | string | 是 | 上次登录返回的令牌 |
| `分支2.displayName` | string | 否 | 缺省由服务端生成「玩家N」 |

### CreateRoom

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### FriendRequest

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `toPlayerId` | string | 是 |  |

### Intent

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `clientIntentId` | string | 是 | 客户端生成，用于幂等 |
| `intent` | Intent | 是 |  |

### JoinRoom

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `code` | string | 是 | 6 位房间码 |

### LeaveRoom

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### Ready

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `ready` | boolean | 是 |  |

### Reconnect

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `roomId` | string | 是 |  |
| `lastSeq` | integer | 是 | 客户端最后收到的 seq |

### RequestSnapshot

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### SelectHeroCard

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `cardId` | string | 是 |  |

### SkipAnimation

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

## 下行消息（S→C）

### AuthResult

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `playerId` | string | 是 |  |
| `token` | string | 是 |  |
| `expiresAt` | string | 是 |  |

### CombatResult

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `ownBoard` | array<BoardMinion> | 是 |  |
| `opponentBoard` | array<BoardMinion> | 是 |  |
| `actions` | array<object> | 是 |  |
| `actions[].kind` | string | 是 | 动作类型，具体取值由规则引擎实现确定 |
| `actions[].sourceIndex` | integer | null | 否 |  |
| `actions[].targetIndex` | integer | null | 否 |  |
| `actions[].damage` | integer | null | 否 |  |
| `actions[].sourceSide` | enum | 否 | sourceIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有来源下标 |
| `actions[].targetSide` | enum | 否 | targetIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有目标下标 |
| `damage` | integer | 是 | 本场战斗败者受到的伤害 |
| `winnerId` | string | null | 是 | 本场胜者；玩家自己的 playerId 表示胜 |
| `mirror` | boolean | 否 | true 表示本场为镜像幽灵战（对手是自身阵容的镜像），此时 winnerId 为本人或 null、damage 为 0 |

### Error

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `code` | ErrorCode | 是 | 错误码 |
| `message` | string | 是 | 面向日志的错误说明，不作为 UI 文案 |
| `clientIntentId` | string | 否 | 被拒绝意图的 clientIntentId |
| `retryable` | boolean | 是 | 是否可原样重试 |

### FriendUpdate

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `friends` | array<object> | 是 |  |
| `friends[].playerId` | string | 是 |  |
| `friends[].displayName` | string | 是 |  |
| `friends[].online` | boolean | 是 |  |

### GameOver

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `ranking` | array<object> | 是 |  |
| `ranking[].playerId` | string | 是 |  |
| `ranking[].place` | integer | 是 |  |
| `seed` | string | 是 | 会战模式对局结束后公开的种子 |

### GameStarted

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `view` | PlayerView | 是 |  |

### PhaseChanged

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `phase` | Phase | 是 |  |
| `turn` | integer | 是 |  |
| `deadline` | string | 是 |  |

### RoomUpdate

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `roomId` | string | 是 |  |
| `code` | string | 是 | 6 位房间码 |
| `hostId` | string | 是 |  |
| `phase` | Phase | 是 |  |
| `players` | array<object> | 是 |  |
| `players[].playerId` | string | 是 |  |
| `players[].displayName` | string | 是 |  |
| `players[].ready` | boolean | 是 |  |
| `players[].isHost` | boolean | 是 |  |
| `players[].heroCardId` | string | null | 是 |  |

### Snapshot

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `view` | PlayerView | 是 |  |

### StateUpdate

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `view` | PlayerView | 是 |  |
| `events` | array<GameEvent> | 是 |  |

### TimerWarning

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `secondsLeft` | integer | 是 |  |

## 共享类型

### Phase

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### ShopSlot

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 |  |
| `cardId` | string | null | 是 | 空槽位为 null |
| `frozen` | boolean | 是 |  |

### HandCard

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 |  |
| `cardId` | string | 是 |  |
| `playable` | boolean | 是 | 服务端算好的可打出提示 |
| `disabledReason` | string | null | 是 | 不可操作时的错误码 |

### BoardMinion

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 |  |
| `minionId` | string | 是 | 随从实例 id |
| `cardId` | string | 是 |  |
| `golden` | boolean | 是 |  |
| `attack` | integer | 是 |  |
| `health` | integer | 是 |  |
| `keywords` | array<string> | 是 | 当前生效的关键词 |
| `sellable` | boolean | 是 |  |

### PlayerSummary

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `playerId` | string | 是 |  |
| `seat` | integer | 是 |  |
| `displayName` | string | 是 |  |
| `hp` | integer | 是 |  |
| `era` | integer | 是 |  |
| `alive` | boolean | 是 |  |
| `boardCount` | integer | 是 |  |
| `heroCardId` | string | null | 是 | 开局选择前为 null |
| `recruited` | boolean | 是 | 本回合是否已结束招募 |
| `placement` | integer | null | 否 | 最终名次，对局结束前为 null |
| `managed` | boolean | 是 | 当前是否由 AI 托管 |

### ShopAffordance

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 |  |
| `buyable` | boolean | 是 |  |
| `disabledReason` | string | null | 是 |  |

### HandAffordance

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 |  |
| `playable` | boolean | 是 |  |
| `disabledReason` | string | null | 是 |  |

### BoardAffordance

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 |  |
| `sellable` | boolean | 是 |  |
| `disabledReason` | string | null | 是 |  |

### Affordances

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `shopSlots` | array<ShopAffordance> | 是 |  |
| `handCards` | array<HandAffordance> | 是 |  |
| `boardMinions` | array<BoardAffordance> | 是 |  |
| `canReroll` | boolean | 是 |  |
| `canUpgradeEra` | boolean | 是 |  |
| `upgradeDisabledReason` | string | null | 是 |  |
| `canUseHeroPower` | boolean | 是 |  |
| `heroPowerDisabledReason` | string | null | 否 | 不可使用英雄技能时的错误码 |
| `canEndRecruit` | boolean | 是 |  |

### GameEvent

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `kind` | enum | 是 |  |
| `playerId` | string | 否 | 事件涉及的玩家，可省略表示自己 |
| `cardId` | string | 否 |  |
| `minionId` | string | 否 |  |
| `amount` | integer | 否 | 伤害等数值 |

### PendingChoice

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `(根)` | oneOf | 是 | 下列分支任选其一 |
| `分支1.kind` | object | 是 | 固定为 discover |
| `分支1.candidates` | array<string> | 是 | 三选一的候选 cardId |
| `分支2.kind` | object | 是 | 固定为 evolve |
| `分支2.options` | array<string> | 是 | 进化选项 |

### PlayerView

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `roomId` | string | 是 |  |
| `phase` | Phase | 是 |  |
| `turn` | integer | 是 |  |
| `phaseDeadline` | string | 是 |  |
| `opponentPlayerId` | string | null | 否 | 当前战斗对手；招募阶段或未配对时为 null |
| `self` | object | 是 |  |
| `self.playerId` | string | 是 |  |
| `self.seat` | integer | 是 |  |
| `self.hp` | integer | 是 |  |
| `self.era` | integer | 是 |  |
| `self.gold` | integer | 是 |  |
| `self.heroCardId` | string | null | 是 |  |
| `self.heroPower` | object | 是 |  |
| `self.heroPower.skillId` | string | null | 是 |  |
| `self.heroPower.usesLeft` | integer | 是 |  |
| `self.shop` | array<ShopSlot> | 是 |  |
| `self.hand` | array<HandCard> | 是 |  |
| `self.board` | array<BoardMinion> | 是 |  |
| `self.rerollCost` | integer | 是 |  |
| `self.upgradeCost` | integer | null | 是 | 满时代为 null |
| `self.recruited` | boolean | 是 |  |
| `self.pendingChoices` | array<PendingChoice> | 是 | 本座位待处理的发现或进化选择，按先到先处理排序 |
| `self.affordances` | Affordances | 是 |  |
| `players` | array<PlayerSummary> | 是 |  |
| `enabledFactions` | array<string> | 是 | 本局启用的 5 个流派 |

### Intent

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `(根)` | oneOf | 是 | 下列分支任选其一 |
| `分支1.kind` | object | 是 |  |
| `分支1.slot` | integer | 是 | 商店槽位 |
| `分支1.targetMinionId` | string | 否 | 战吼或效果需要目标时填 |
| `分支2.kind` | object | 是 |  |
| `分支2.minionId` | string | 是 | 要出售的随从实例 id |
| `分支3.kind` | object | 是 |  |
| `分支4.kind` | object | 是 |  |
| `分支5.kind` | object | 是 |  |
| `分支5.handIndex` | integer | 是 | 手牌位置 |
| `分支5.targetMinionId` | string | 否 | 战吼或效果需要目标时填 |
| `分支6.kind` | object | 是 |  |
| `分支6.fromIndex` | integer | 是 |  |
| `分支6.toIndex` | integer | 是 |  |
| `分支7.kind` | object | 是 |  |
| `分支7.slot` | integer | null | 否 | null 表示全部槽位 |
| `分支8.kind` | object | 是 |  |
| `分支8.skillId` | string | 是 |  |
| `分支8.targetMinionId` | string | 否 | 需要目标的技能才填 |
| `分支9.kind` | object | 是 |  |
| `分支10.kind` | object | 是 |  |
| `分支10.choice` | integer | 是 | 发现为候选下标，进化为选项下标 |

### ErrorCode

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### ErrorMessage

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `code` | ErrorCode | 是 | 错误码 |
| `message` | string | 是 | 面向日志的错误说明，不作为 UI 文案 |
| `clientIntentId` | string | 否 | 被拒绝意图的 clientIntentId |
| `retryable` | boolean | 是 | 是否可原样重试 |
