# v0.1 验收记录

> 状态：已记录（M2 断线重连一致性）
> 关联：M2 联机内测

本文记录 v0.1 的验收口径与自动化验收结果，不描述产品规则或实现方案。规则见[核心规则](product/rules/core-rules.md)，实现见[当前实现状态](../../current/README.md)。

## 1. 验收口径

- M2 不设真人完成率验收。断线重连后状态一致由自动化用例保证（见 §2）。
- M3 不做单独验收记录：桌面端完整对局、崩溃率、意图拒绝率与单局时长不记入本文，也不作为 v0.1 的退出条件。

## 2. 断线重连一致性

自动化用例覆盖「重连快照与当前视图逐字段一致」与「战斗阶段重连补发本场 `CombatResult`」，不依赖真人操作：

```bash
cd server && cargo test -p room-service --test reconnect
```

| 日期 | 用例数 | 结果 | 备注 |
| --- | --- | --- | --- |
| 2026-11-20 | 11 | 全部通过 | `cargo test -p room-service --test reconnect`；同时由 `make m1-check` 的 `cargo test --workspace` 覆盖 |

## 相关文档

- [v0.1 版本入口](README.md)
- [未确认事项](open-questions.md)
- [房间服务与联网](../../current/technical/room-service/README.md)
