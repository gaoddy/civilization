# 技术现状

本文是已实现技术现状的入口，列出文档与对应代码位置；不重复文档内容。

| 文档 | 内容 | 代码位置 |
| --- | --- | --- |
| [规则引擎与模拟器](rule-engine-and-simulator.md) | `rule-engine` 与 `sim` 两个 crate 的边界、卡牌数据生成、构建与验证命令、报告字段口径、规则用例清单 | `server/`、`scripts/`、`Makefile` |
| [房间服务与联网](room-service/README.md) | `room-service` 与 `protocol` 的边界、`/ws` 接入与信封、下行序号与去重、逐座位过滤、计时与断线、好友与邀请、PostgreSQL 持久化、`sim bot` 用法、`make m1-check` 验收证据 | `server/crates/room-service`、`server/crates/protocol`、`server/migrations`、`protocol/`、`Makefile` |
| [Godot 客户端](client.md) | Godot 工程结构、`GameStore`/`Net`/`CardDB` 职责、六场景、生成物与运行方式、客户端验收、桌面端导出（`make client-export`） | `client/`、`Makefile` |
| [部署与接入](deployment/README.md) | platform Kubernetes 接入、Helm 清单、镜像与发布入口、健康检查、客户端连接方式 | `deploy/helm`、`server/Dockerfile`、`.dockerignore`、`Makefile` |

v0.1 已实现到 M1b，并补齐赛后回顾、房间生命周期、首局引导与 macOS 桌面端导出；M2 起的里程碑尚未实现。
