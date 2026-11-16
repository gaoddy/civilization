# 英雄候选分配与选英雄页 · 技术方案

> 状态：已实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 内容 | 英雄卡从 13 张扩到 32 张，新增 13 个技能；只用现有 DSL 原语 |
| 协议 | `RoomUpdate.players[]` 新增可选 `heroCandidates` |
| 房间服务 | `PlayerSlot` 保存候选与断线时刻；建房/入房时分配候选；选英雄校验候选；大厅断线改为宽限期 |
| 客户端 | 选英雄页重写为「他人状态 + 本人候选卡 + 确认」；删除房间码与离开入口 |
| 机器人 | `sim bot` 与无头验收客户端改为从本人候选里选英雄 |

不涉及：邀请与分享房间码、对局内英雄展示、自动匹配与天梯、发现/进化等既有规则。

## 2. 内容

- 19 张新英雄卡（`docs/versions/v0.1/product/content/cards/*.md`）与 13 个新技能（`skills/*.md`），全部只用已白名单的触发与原语，未新增 `Op` / `Trigger`。
- `scripts/validate_docs.py` 的分布常量与英雄数量断言同步到 `随从 203 / 效果 36 / 技能 131 / 英雄卡 32`。

## 3. 协议

### 3.1 `RoomUpdate.players[].heroCandidates`

```json
"heroCandidates": {
  "type": ["array", "null"],
  "items": { "type": "string" },
  "description": "本座位可选的英雄卡 id；只在发给本人的 RoomUpdate 中下发"
}
```

- 不加入 `required`，旧客户端忽略未知字段即可；未下发时按 `null` 处理。
- 只有本人那一项填充，其余座位恒为 `null`。因此大厅广播必须**逐座位序列化**，不能复用同一份 payload；`RoomRegistry::room_update_for(room_id, viewer)` 承担这一职责，`broadcast_room_update` 与 `send_room_update_to` 都改走它。

## 4. 房间服务

### 4.1 候选分配

- `PlayerSlot` 新增 `hero_candidates: Vec<String>` 与 `candidate_segment: Option<usize>`。
- 分配算法：把卡表里所有 `is_hero` 的 id 升序取出，用 `seed_for_code(房间码)` 做确定性 Fisher–Yates 洗牌（线性同余推进，不引入随机源），再按段切分，每段 `CANDIDATE_COUNT = 4` 张。
- 段号取「当前座位里未被占用的最小段」：玩家离开后段号随之空出，新玩家能拿回完整的一组，候选池不会被永久占满。
- 建房时立即给房主分配，入房时给新玩家分配；同一玩家重复加入幂等，不重新分配。

### 4.2 选英雄校验

顺序固定为：房间存在 → 阶段为 `Lobby` → 玩家在房间内 → 卡存在且 `is_hero` → 座位未 `ready` → 卡在本人候选内。

- 候选外：新增 `RoomError::NotCandidate`，映射 `INVALID_CHOICE`，文案「不在可选英雄范围内」。
- 已确认后再选：`WrongPhase` → `INVALID_STAGE`，且不改变已选英雄。

### 4.3 大厅断线宽限

- `Detach` 不再立即 `leave`：标记 `connected = false` 并记录 `disconnected_at_ms`，广播大厅视图。
- 房间 task 的 tick 调用 `RoomRegistry::tick_disconnects(now)`，对 `now - disconnected_at_ms >= RECONNECT_GRACE_MS`（30 秒，与对局内同值）的座位执行原 `leave` 并广播。
- `attach` / `reconnect` 在大厅分支调用 `mark_reconnected`；座位已被释放时回 `NOT_IN_ROOM`。
- `Leave` 命令仍立即释放座位，不走宽限。

## 5. 客户端

- `GameStore` 新增 `my_hero_candidates()` / `my_ready()` / `my_hero_id()`，只搬运 `room_players` 里本人那一项。
- `client/scenes/room.gd` 整体重写：`OtherPlayers`（名字 + 确认状态胶囊）、`HeroCandidates`（`HFlowContainer`，160×200 候选卡）、`ConfirmButton`、`StatusLabel`、`SkillLayer`（技能浮层）。
- 候选卡为页面私有结构：部落色封面 `ColorRect` + 居中首字水印 + 底部居中名字；选中态 2px `color-accent` 描边。
- 确认一次发送 `SelectHeroCard` 与 `Ready{ready:true}`；确认后按钮显示「已确认」并禁用，候选卡解除点击绑定。
- 删除房间码标签、离开房间按钮与英雄下拉框。

## 6. 机器人与验收

- `sim bot` 的 `--hero` 降级为偏好：收到本人 `RoomUpdate` 后从 `heroCandidates` 选，不在候选内退到第一张；候选为空时报错退出。
- `client/tests/headless_game.gd` 同样从本人候选选英雄，`M1B_HERO` 仅为偏好。

## 7. 验证

- `python3 scripts/validate_docs.py`：数量与分布校验通过。
- `make protocol-rs && make protocol && make protocol-check`：生成物无漂移。
- `cd server && cargo test -p room-service`：候选互不重复、候选回收、候选外拒绝、确认后不可改选、断线宽限与超时释放均覆盖。
- `make client-smoke`：候选卡渲染、选中态、未连接禁用、已确认锁定、无房间码与离开入口。

## 相关文档

- [选英雄页](../../../current/product/ui/pages/room.md)
- [房间服务与联网](../../../current/technical/room-service.md)
