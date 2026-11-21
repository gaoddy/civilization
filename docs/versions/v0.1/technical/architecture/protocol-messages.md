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
| `分支3.steamTicket` | string | 是 | Steam Web API 票据十六进制串 |
| `分支3.displayName` | string | 否 | Steam 个人资料名；仅作展示，服务端截断为 20 字符 |

### BindPhone

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `phone` | string | 是 | 待绑定的手机号 |
| `code` | string | 是 | 短信验证码 |

### CombatPlayed

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### CreateRoom

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### CreateRoomInvite

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### DissolveRoom

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### FriendRemove

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `friendId` | string | 是 |  |

### FriendRequest

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `toPlayerCode` | string | 是 | 6 位玩家编号 |

### FriendRespond

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `fromPlayerId` | string | 是 | 申请人的 playerId |
| `accept` | boolean | 是 |  |

### Intent

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `clientIntentId` | string | 是 | 客户端生成，用于幂等 |
| `intent` | Intent | 是 |  |

### InviteToRoom

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `friendId` | string | 是 |  |

### JoinRoom

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `code` | string | 是 | 6 位房间码 |

### KickMember

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `targetPlayerId` | string | 是 | 要被移出房间的玩家 playerId |

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

### RedeemRoomInvite

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `token` | string | 是 | 一次性邀请 token |

### RequestAccount

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### RequestPhoneCode

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `phone` | string | 是 | 待绑定的手机号 |

### RequestSnapshot

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### RespondInvite

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `inviteId` | string | 是 |  |
| `accept` | boolean | 是 |  |

### SelectHeroCard

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `cardId` | string | 是 |  |

## 下行消息（S→C）

### AccountView

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `playerId` | string | 是 | 内部不透明标识，不向玩家展示 |
| `playerCode` | string | 是 | 6 位玩家编号 |
| `displayName` | string | 是 |  |
| `steamBound` | boolean | 是 | 账号是否绑定 Steam 身份 |
| `phoneBound` | boolean | 是 | 账号是否绑定手机号 |
| `maskedPhone` | string | null | 否 | 前 3 位与后 4 位保留，其余用 *；未绑定为 null |

### AchievementState

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `unlocked` | array<object> | 是 |  |
| `unlocked[].achievementId` | string | 是 |  |
| `unlocked[].steamApiName` | string | 是 |  |

### AchievementUnlocked

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `achievementId` | string | 是 |  |
| `steamApiName` | string | 是 |  |

### AuthResult

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `playerId` | string | 是 | 内部不透明标识，不向玩家展示 |
| `playerCode` | string | 是 | 6 位玩家编号，用于好友搜索与展示 |
| `token` | string | 是 |  |
| `expiresAt` | string | 是 |  |

### CombatResult

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `startedAt` | string | 是 | 本场演示开始时刻（服务端时钟，RFC3339）；重连补发时保持同值 |
| `ownBoard` | array<BoardMinion> | 是 |  |
| `opponentBoard` | array<BoardMinion> | 是 |  |
| `actions` | array<object> | 是 |  |
| `actions[].kind` | string | 是 | 动作类型，具体取值由规则引擎实现确定 |
| `actions[].cardId` | string | null | 否 | 动作直接涉及的卡牌 id（如零件回收获得的牌）；没有卡牌载荷的动作省略 |
| `actions[].sourceIndex` | integer | null | 否 |  |
| `actions[].targetIndex` | integer | null | 否 |  |
| `actions[].damage` | integer | null | 否 |  |
| `actions[].sourceSide` | enum | 否 | sourceIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有来源下标 |
| `actions[].targetSide` | enum | 否 | targetIndex 所指棋盘，相对接收该消息的座位；null 表示本条动作没有目标下标 |
| `actions[].sourceMinionId` | string | null | 否 | 动作来源随从实例 id；死亡、复生、召唤等无来源动作为 null |
| `actions[].targetMinionId` | string | null | 否 | 动作目标随从实例 id；死亡、复生、召唤等无目标动作为 null |
| `actions[].source` | CombatMinionState | 否 |  |
| `actions[].target` | CombatMinionState | 否 |  |
| `actions[].summon` | object | 否 |  |
| `actions[].summon.side` | enum | 是 | 相对接收该消息的座位 |
| `actions[].summon.position` | integer | 是 | 召唤后该随从在存活随从序列中的插入下标；上界由 PlayerView.boardSlots 决定 |
| `actions[].summon.minion` | BoardMinion | 是 |  |
| `damage` | integer | 是 | 本场战斗结算给败者的伤害（含幽灵战）；平局与幽灵战落单座位获胜时为 0 |
| `winnerId` | string | null | 是 | 本场胜者；玩家自己的 playerId 表示胜 |
| `mirror` | boolean | 否 | true 表示本场为幽灵战，对手是已出局者的幽灵阵容 |

### Error

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `code` | ErrorCode | 是 | 错误码 |
| `messageId` | string | 是 | 面向玩家的文本 id，默认 error.<CODE> |
| `messageParams` | array<string> | 否 | messageId 的 %s 参数 |
| `message` | string | 是 | 面向日志的错误说明，不作为 UI 文案 |
| `clientIntentId` | string | 否 | 被拒绝意图的 clientIntentId |
| `retryable` | boolean | 是 | 是否可原样重试 |

### FriendInvite

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `inviteId` | string | 是 |  |
| `fromPlayerId` | string | 是 |  |
| `fromPlayerCode` | string | 是 |  |
| `fromDisplayName` | string | 是 |  |
| `roomCode` | string | 是 |  |
| `expiresAt` | string | 是 |  |

### FriendInviteResult

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `inviteId` | string | 是 |  |
| `accepted` | boolean | 是 |  |
| `reason` | string | null | 否 | 拒绝或失败原因的错误码 |

### FriendUpdate

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `friends` | array<object> | 是 |  |
| `friends[].playerId` | string | 是 |  |
| `friends[].playerCode` | string | 是 | 6 位玩家编号 |
| `friends[].displayName` | string | 是 |  |
| `friends[].status` | enum | 是 |  |
| `incoming` | array<FriendRequestItem> | 是 |  |
| `outgoing` | array<object> | 是 |  |
| `outgoing[].playerId` | string | 是 |  |
| `outgoing[].playerCode` | string | 是 |  |
| `outgoing[].displayName` | string | 是 |  |
| `outgoing[].sentAt` | string | 是 |  |
| `recent` | array<object> | 是 |  |
| `recent[].playerId` | string | 是 |  |
| `recent[].playerCode` | string | 是 |  |
| `recent[].displayName` | string | 是 |  |
| `recent[].relation` | enum | 是 |  |

### GameOver

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `ranking` | array<object> | 是 |  |
| `ranking[].playerId` | string | 是 |  |
| `ranking[].place` | integer | 是 |  |
| `ranking[].survivedTurns` | integer | 否 | 该座位存活到的回合；旧客户端缺省按 0 处理 |
| `ranking[].damageDealt` | integer | 否 | 该座位本局造成的总伤害；旧客户端缺省按 0 处理 |
| `seed` | string | 是 | 会战模式对局结束后公开的种子 |
| `selfStats` | SelfStats | 否 | 接收者本人的本局统计；只填接收者所在座位 |

### GameStarted

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `view` | PlayerView | 是 |  |

### InviteAccepted

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `inviteId` | string | 是 |  |
| `roomCode` | string | 是 |  |

### PhaseChanged

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `phase` | Phase | 是 |  |
| `turn` | integer | 是 |  |
| `deadline` | string | 是 |  |

### PhoneCodeSent

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `expiresAt` | string | 是 | 验证码失效时刻 |

### RoomClosed

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `reason` | enum | 是 | kicked：被房主移出；dissolved：房间已解散 |

### RoomInviteCode

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `token` | string | 是 | 一次性邀请 token |
| `roomCode` | string | 是 | 6 位房间码 |
| `expiresAt` | string | 是 | token 失效时刻，创建后 60 秒 |

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
| `players[].heroCandidates` | array | null | 否 | 本座位可选的英雄卡 id；只在发给本人的 RoomUpdate 中下发 |

### Snapshot

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `view` | PlayerView | 是 |  |
| `seq` | integer | 否 | 快照对应的座位消息序号；与信封 seq 一致，M1 新增可选字段 |

### StateUpdate

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `view` | PlayerView | 是 |  |
| `events` | array<GameEvent> | 是 |  |

### TimerWarning

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `secondsLeft` | integer | 是 |  |

## 共享类型（messages.schema.json）

### CombatMinionState

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `minionId` | string | 是 | 随从实例 id |
| `attack` | integer | 是 |  |
| `health` | integer | 是 |  |
| `alive` | boolean | 是 |  |

### FriendRequestItem

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `playerId` | string | 是 |  |
| `playerCode` | string | 是 |  |
| `displayName` | string | 是 |  |
| `sentAt` | string | 是 |  |

### SelfStats

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `place` | integer | 是 |  |
| `survivedTurns` | integer | 是 |  |
| `finalHp` | integer | 是 |  |
| `finalEra` | integer | 是 |  |
| `remainingGold` | integer | 是 |  |
| `damageDealt` | integer | 是 |  |
| `damageTaken` | integer | 是 |  |
| `buys` | integer | 是 |  |
| `sells` | integer | 是 |  |
| `rerolls` | integer | 是 |  |
| `triples` | integer | 是 |  |
| `combatWins` | integer | 是 |  |
| `combatDraws` | integer | 是 |  |
| `combatLosses` | integer | 是 |  |
| `heroPowerUses` | integer | 是 |  |
| `board` | array<object> | 是 | 终局棋盘，按站位顺序；最多 7 项 |
| `board[].cardId` | string | 是 |  |
| `board[].golden` | boolean | 是 |  |
| `board[].attack` | integer | 是 |  |
| `board[].health` | integer | 是 |  |

## 共享类型

### TargetZone

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### TargetRef

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `zone` | TargetZone | 是 |  |
| `id` | string | 是 | 目标实例编号；三个区域共用同一编号空间 |

### Phase

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### ShopSlot

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 |  |
| `instanceId` | string | null | 是 | 卡牌实例编号；空槽位为 null |
| `cardId` | string | null | 是 | 空槽位为 null |
| `frozen` | boolean | 是 |  |

### HandCard

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 |  |
| `instanceId` | string | 是 | 卡牌实例编号；买入、上手与三连合成沿用同一编号 |
| `cardId` | string | 是 |  |
| `golden` | boolean | 是 | 随从牌为金色三连结果；效果牌恒为 false |
| `attack` | integer | null | 是 | 随从牌的当前攻击力（已含成长等全部层数）；效果牌为 null |
| `health` | integer | null | 是 | 随从牌的当前生命值；效果牌为 null |
| `rewardLevel` | integer | null | 是 | 「三连奖励」锁定的发现星级；其它牌为 null |
| `playable` | boolean | 是 | 服务端算好的可打出提示 |
| `requiresTarget` | boolean | 是 | 该牌是否需要玩家指定单一目标 |
| `targetCandidates` | array<TargetRef> | 是 | 该牌的合法目标；随从牌为空数组 |
| `disabledReason` | string | null | 是 | 不可操作时的错误码 |

### BoardMinion

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `index` | integer | 是 | 该随从在当前存活随从序列中的下标；上界由 PlayerView.boardSlots 决定 |
| `minionId` | string | 是 | 随从实例 id |
| `cardId` | string | 是 |  |
| `golden` | boolean | 是 |  |
| `attack` | integer | 是 |  |
| `health` | integer | 是 |  |
| `keywords` | array<string> | 是 | 当前生效的关键词，取值为英文 id |
| `stealth` | boolean | 是 | 当前是否仍处于潜行：携带 stealth 关键词且尚未被打破 |
| `sellable` | boolean | 是 |  |
| `stats` | MinionStats | 否 | 攻防分层构成；旧客户端可忽略 |

### MinionStatBreakdown

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `initial` | integer | 是 | 基础值 |
| `permanent` | integer | 是 | 成长与永久增量之和 |
| `turn` | integer | 是 | 本回合临时增量，可为负 |
| `aura` | integer | 是 | 光环派生增量，可为负 |
| `damage` | integer | 是 | 层和与当前值之差 |
| `current` | integer | 是 | 当前实际值 |

### MinionStats

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `attack` | MinionStatBreakdown | 是 |  |
| `health` | MinionStatBreakdown | 是 |  |

### CombatLogEntry

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `turn` | integer | 是 |  |
| `participantIds` | array<string> | 是 | 参战玩家 id；幽灵战只有对战方一个元素（幽灵不是玩家） |
| `ghostId` | string | null | 是 | 幽灵战为幽灵归属者的玩家 id；真实战为 null |
| `winnerId` | string | null | 是 | 平局无胜者时为 null |
| `loserId` | string | null | 是 | 幽灵战失利时为该玩家；幽灵不受伤，所以幽灵输时也为 null |
| `damage` | integer | 是 |  |
| `mirror` | boolean | 是 | true 表示对战已出局者的幽灵阵容 |

### PlayerSummary

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `playerId` | string | 是 |  |
| `seat` | integer | 是 |  |
| `displayName` | string | 是 |  |
| `hp` | integer | 是 |  |
| `era` | integer | 是 |  |
| `rank` | integer | null | 是 | 当前名次；对局结束前为实时名次，结束后等于 placement |
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
| `canUnfreeze` | boolean | 是 | 冻结按钮是否为「解冻」态：本回合冻结过商店且仍有冻结槽位；跨回合保留的冻结不算 |
| `canUpgradeEra` | boolean | 是 |  |
| `upgradeDisabledReason` | string | null | 是 |  |
| `canEndRecruit` | boolean | 是 |  |

### HeroPower

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `skillId` | string | 是 |  |
| `usesLeft` | integer | 是 |  |
| `requiresTarget` | boolean | 是 | 该主动技能是否需要玩家指定单一目标 |
| `targetCandidates` | array<TargetRef> | 是 | 该主动技能的合法目标 |
| `canUse` | boolean | 是 |  |
| `disabledReason` | string | null | 是 | 不可使用该技能时的错误码 |

### GameEvent

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `kind` | enum | 是 |  |
| `playerId` | string | 否 | 事件涉及的玩家，可省略表示自己 |
| `seat` | integer | 否 | 事件涉及的座位；客户端可经 PlayerView.players 映射到 playerId |
| `cardId` | string | 否 |  |
| `minionId` | string | 否 |  |
| `amount` | integer | 否 | 伤害等数值 |
| `place` | integer | null | 否 | PlayerEliminated 事件对该座位判定的名次；其他事件为 null |
| `turn` | integer | 否 | TurnStarted/PhaseChanged 事件对应的回合号 |
| `phase` | Phase | 否 | PhaseChanged 事件进入的阶段 |
| `ranking` | array<integer> | 否 | GameOver 事件的座位排名，从第 1 名起 |

### PendingChoice

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `(根)` | oneOf | 是 | 下列分支任选其一 |
| `分支1.kind` | object | 是 | 固定为 discover |
| `分支1.candidates` | array<string> | 是 | 三选一的候选 cardId |
| `分支2.kind` | object | 是 | 固定为 evolve |
| `分支2.options` | array<string> | 是 | 进化选项，取值为英文 id |
| `分支3.kind` | object | 是 | 固定为 target |
| `分支3.sourceMinionId` | string | 是 | 触发该待选的随从 |
| `分支3.candidates` | array<TargetRef> | 是 | 战吼待选的合法目标 |

### PlayerView

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `roomId` | string | 是 |  |
| `phase` | Phase | 是 |  |
| `turn` | integer | 是 |  |
| `phaseDeadline` | string | 是 |  |
| `boardSlots` | integer | 是 | 本方棋盘随从位上限，v0.1 为 7 |
| `hpCap` | integer | 是 | 生命上限，v0.1 为 40 |
| `combatLog` | array<CombatLogEntry> | 是 | 本局最近 5 个回合的全部战斗，按回合升序；全对局公开 |
| `opponentPlayerId` | string | null | 否 | 当前战斗对手；招募阶段或未配对时为 null |
| `self` | object | 是 |  |
| `self.playerId` | string | 是 |  |
| `self.seat` | integer | 是 |  |
| `self.hp` | integer | 是 |  |
| `self.era` | integer | 是 |  |
| `self.gold` | integer | 是 |  |
| `self.heroCardId` | string | null | 是 |  |
| `self.heroPowers` | array<HeroPower> | 是 | 英雄卡上的全部主动技能，按英雄卡 skills 顺序排列；没有主动技能时为空 |
| `self.shop` | array<ShopSlot> | 是 |  |
| `self.hand` | array<HandCard> | 是 |  |
| `self.board` | array<BoardMinion> | 是 |  |
| `self.rerollCost` | integer | 是 |  |
| `self.upgradeCost` | integer | null | 是 | 满时代为 null |
| `self.recruited` | boolean | 是 |  |
| `self.pendingChoices` | array<PendingChoice> | 是 | 本座位待处理的发现或进化选择，按先到先处理排序 |
| `self.affordances` | Affordances | 是 |  |
| `players` | array<PlayerSummary> | 是 |  |
| `enabledFactions` | array<string> | 是 | 本局启用的 5 个玩法 |

### Intent

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `(根)` | oneOf | 是 | 下列分支任选其一 |
| `分支1.kind` | object | 是 |  |
| `分支1.slot` | integer | 是 | 商店槽位 |
| `分支2.kind` | object | 是 |  |
| `分支2.minionId` | string | 是 | 要出售的随从实例 id |
| `分支3.kind` | object | 是 |  |
| `分支4.kind` | object | 是 |  |
| `分支5.kind` | object | 是 |  |
| `分支5.handIndex` | integer | 是 | 手牌位置 |
| `分支5.position` | integer | 否 | 插入站位；缺省追加到最右 |
| `分支5.target` | TargetRef | 否 | 效果牌或战吼需要目标时填 |
| `分支6.kind` | object | 是 |  |
| `分支6.fromIndex` | integer | 是 |  |
| `分支6.toIndex` | integer | 是 |  |
| `分支7.kind` | object | 是 |  |
| `分支7.slot` | integer | null | 否 | null 表示全部槽位 |
| `分支8.kind` | object | 是 |  |
| `分支8.slot` | integer | null | 否 | null 表示全部槽位 |
| `分支9.kind` | object | 是 |  |
| `分支9.skillId` | string | 是 |  |
| `分支9.target` | TargetRef | 否 | 需要目标的技能才填 |
| `分支10.kind` | object | 是 |  |
| `分支11.kind` | object | 是 |  |
| `分支11.choice` | integer | 是 | 发现为候选下标，进化为选项下标 |
| `分支12.kind` | object | 是 |  |
| `分支12.target` | TargetRef | 是 | 目标 |
| `分支13.kind` | object | 是 |  |

### ErrorCode

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |

### ErrorMessage

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `code` | ErrorCode | 是 | 错误码 |
| `messageId` | string | 是 | 面向玩家的文本 id，默认 error.<CODE> |
| `messageParams` | array<string> | 否 | messageId 的 %s 参数 |
| `message` | string | 是 | 面向日志的错误说明，不作为 UI 文案 |
| `clientIntentId` | string | 否 | 被拒绝意图的 clientIntentId |
| `retryable` | boolean | 是 | 是否可原样重试 |
