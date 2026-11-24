# 房间等待区与对局内选英雄（技术基线）

> 状态：已实现
> 版本：v0.2 增量
> 产品基线：[房间等待区与对局内选英雄](../product/systems/room-waiting-and-hero-pick.md)
> 已实现文档：[房间与邀请](../../../current/product/systems/rooms-and-invites.md)、[协议字段](../../../current/technical/architecture/protocol-messages.md)

本文记录 v0.2 房间等待区与选英雄阶段的架构选择与落点。

## 1. 架构选择

| 选择 | 结论 |
| --- | --- |
| 选英雄阶段 | `room-service::GameTask` 的前置 `HeroPick` 内部阶段；选完后调用 `Engine::start_game`，不改规则引擎 |
| 机器人 AI | 新建共享 crate `server/crates/bot-ai`，`sim` 与 `room-service` 共用 |
| 机器人座位 | `PlayerSlot` 增加 `is_bot`；房主开始时按计划补满空位 |

## 2. 协议

- 新增 c2s：`StartGame`、`SetBotFill`。
- `SelectHeroCard` 改为选英雄阶段使用。
- 新增 s2c：`HeroPickStarted`，逐座位下发本人候选、截止时刻与各座位是否已选。
- `RoomUpdate` 增加 `seat`、`botFill`，移除 `heroCandidates`、`heroCardId`。
- 生成物随源提交：`protocol/generated/protocol.rs`、`protocol/generated/protocol.gd`、`client/generated/protocol.gd`、协议字段文档。

## 3. 服务端

- `room.rs`：准备/取消准备、补位开关；`prepare_start` 只读规划，`commit_start` 写入机器人与阶段，构造失败不留半成品；候选在开始阶段统一分配。
- `game.rs`：选英雄阶段、超时确定性选定、选英雄阶段断线/弃权、机器人逐回合驱动与战斗自动回报。
- `net.rs`：新命令、开局交接、匹配房同样进入选英雄阶段。
- 无数据库迁移。

## 4. 客户端

- `GameStore` 新增 `hero_pick_started` 信号与选英雄状态；房间页据此导航到对局页，对局页在 `_ready` 读取 `GameStore.hero_pick` 显示覆盖层。
- 房间页重写为 8 座位等待区，准备、开始、补位开关、离开确认与房主管理。
- 对局页新增选英雄覆盖层，复用现有候选卡与详情组件；进入第 1 回合后隐藏。
- 文案表新增房间与选英雄条目，删除大厅选英雄时代的旧条目。

## 相关文档

- [房间与邀请](../../../current/product/systems/rooms-and-invites.md)
- [协议字段](../../../current/technical/architecture/protocol-messages.md)
