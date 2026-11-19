# 当前实现状态

本文是 `docs/current/` 的入口，列出代码已实现的能力及其文档；只描述现状，不复制版本规划。

| 文档 | 内容 |
| --- | --- |
| [technical/](technical/README.md) | 已实现的技术现状：工程结构、引擎边界、数据生成、命令、报告字段与验证方式；房间服务与联网、协议生成物、`sim bot` |
| [product/ui/](product/ui/README.md) | 已实现的客户端 UI：视觉令牌、通用交互、页面与共享组件 |

| 里程碑 | 状态 | 验证结果 | 实现文档 |
| --- | --- | --- | --- |
| v0.1 M0 规则可跑 | 已实现 | 七类验证标准由验证矩阵命令与报告字段支撑，见[规则引擎与模拟器](technical/rule-engine-and-simulator.md) §5；数值基线的口径问题登记在 [v0.1 平衡问题](../versions/v0.1/product/content/balance-questions.md) | [规则引擎与模拟器](technical/rule-engine-and-simulator.md) |
| v0.1 M1a 房间服务 | 已实现 | `make m1-check` 整体退出 0，证据见[房间服务与联网](technical/room-service/README.md) §13 | [房间服务与联网](technical/room-service/README.md) |
| v0.1 M1b Godot 客户端 | 已实现 | `make client-check`、`make client-smoke` 与 `make m1b-e2e` 均退出 0，见[Godot 客户端](technical/client.md) §4 | [Godot 客户端](technical/client.md) |
| v0.1 M2 联机内测 | 已实现 | 断线重连一致性由 `room-service/tests/reconnect.rs` 用例覆盖；不设真人完成率验收，口径见 [v0.1 验收记录](../versions/v0.1/acceptance.md) §2 | [房间服务与联网](technical/room-service/README.md) §10 |
| v0.1 M3 桌面端上线 | 已实现 | macOS 桌面端可完成完整对局；iOS 端已实现横屏、分设备缩放基准、触控与安全区，开发签名真机包待 Apple 账号登录后产出；不设单独验收记录，也不记录崩溃率、意图拒绝率与单局时长 | [Godot 客户端](technical/client.md) §8、§9 |

v0.1 剩余四项能力（赛后数据回顾、房间生命周期、首局界面引导、macOS 桌面端导出）已实现，现状见 [v0.1 版本说明](../versions/v0.1/README.md) 与下列实现文档。
