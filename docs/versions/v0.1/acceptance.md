# v0.1 验收记录

> 状态：已记录（M2 断线重连一致性；iOS 适配自动化部分，真机项待执行）
> 关联：M2 联机内测、iOS 移动端适配（破例纳入 v0.1）

本文记录 v0.1 的验收口径与自动化验收结果，不描述产品规则或实现方案。规则见[核心规则](product/rules/core-rules.md)，实现见[当前实现状态](../../current/README.md)。

## 1. 验收口径

- M2 不设真人完成率验收。断线重连后状态一致由自动化用例保证（见 §2）。
- M3 不做单独验收记录：桌面端完整对局、崩溃率、意图拒绝率与单局时长不记入本文，也不作为 v0.1 的退出条件。
- iOS 适配的验收形态为真机可安装的开发签名包，分自动化与真机两部分记录（见 §3）。

## 2. 断线重连一致性

自动化用例覆盖「重连快照与当前视图逐字段一致」与「战斗阶段重连补发本场 `CombatResult`」，不依赖真人操作：

```bash
cd server && cargo test -p room-service --test reconnect
```

| 日期 | 用例数 | 结果 | 备注 |
| --- | --- | --- | --- |
| 2026-11-20 | 11 | 全部通过 | `cargo test -p room-service --test reconnect`；同时由 `make m1-check` 的 `cargo test --workspace` 覆盖 |

## 3. iOS 移动端适配

适配规则见[移动端适配](../../current/product/ui/mobile-adaptation.md)，实现见[客户端技术现状](../../current/technical/client.md) §9。验收分自动化与真机两部分。

### 3.1 自动化（可复跑）

```bash
make client-check
make client-smoke
make m1b-e2e
```

| 日期 | 项 | 结果 | 备注 |
| --- | --- | --- | --- |
| 2026-11-20 | `make client-check` | 退出码 0 | 无脚本解析错误 |
| 2026-11-20 | `make client-smoke` | 13 个场景全部 `UI_SMOKE_OK` | 含 `ui_smoke_touch`、`ui_smoke_layout`、`ui_smoke_platform`；无 `SCRIPT ERROR` |
| 2026-11-20 | 设备基准与命中换算 | 通过 | 桌面 / iPhone（960×540）/ iPad（1280×720）三档断言：44pt→60/41、32pt→44/30；安全区与键盘像素换算 |
| 2026-11-20 | 小屏布局 | 通过 | 960×540 下六个页面关键节点不越界；图鉴目录、详情与 4 列网格 |
| 2026-11-20 | 触控交互 | 通过 | 长按 450ms 显示详情、移动取消、多指与阻塞态不触发；手牌悬停残留复位 |
| 2026-11-20 | `make m1b-e2e` | 退出码 0 | 客户端到服务端联调未受影响 |
| 2026-11-20 | iOS 工程编译 | `BUILD SUCCEEDED`（`CODE_SIGNING_ALLOWED=NO`） | 导出 Xcode 工程可编译；正式签名包见 §3.2 |

### 3.2 真机项（待执行）

以下项目需要 Apple 开发者账号完成签名并有可用真机，当前未执行：

| 项 | 判定 |
| --- | --- |
| `make client-export-ios` 产出签名 `.ipa` | 命令退出码 0，`output/ios/Civilization-ios.ipa` 可安装 |
| iPhone 真机安装并启动 | 横屏进入登录页，无闪退 |
| iPad 真机安装并启动 | 1280×720 基准，图鉴 5 列 |
| 全触控完成一局 | 8 人房间招募、战斗、结算走完，不依赖鼠标 |
| 安全区实测 | 刘海/灵动岛与 Home 指示条不遮挡按钮与文本 |
| 左横与右横 | 两个方向均可操作 |

## 相关文档

- [v0.1 版本入口](README.md)
- [未确认事项](open-questions.md)
- [房间服务与联网](../../current/technical/room-service/README.md)
