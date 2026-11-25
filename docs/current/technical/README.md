# 技术现状

本文是已实现技术现状的入口，列出文档与对应代码位置；不重复文档内容。

| 文档 | 内容 | 代码位置 |
| --- | --- | --- |
| [架构约束](architecture/README.md) | 联网与技术架构、通信协议与协议字段表、客户端与服务端技术基线；遗留项见 [v0.1 待确认事项](../../versions/v0.1/open-questions.md) | `server/crates/protocol`、`protocol/`、`client/` |
| [规则引擎与模拟器](rule-engine-and-simulator.md) | `rule-engine` 与 `sim` 两个 crate 的边界、卡牌数据生成、构建与验证命令、报告字段口径、规则用例清单 | `server/`、`scripts/`、`Makefile` |
| [房间服务与联网](room-service/README.md) | `room-service` 与 `protocol` 的边界、`/ws` 接入与信封、下行序号与去重、逐座位过滤、计时与断线、好友与邀请、PostgreSQL 持久化、`sim bot` 用法、`make check` 验收证据 | `server/crates/room-service`、`server/crates/protocol`、`server/migrations`、`protocol/`、`Makefile` |
| [Godot 客户端](client/README.md) | Godot 工程结构、`GameStore`/`Net`/`CardDB` 职责、六场景、生成物与运行方式、客户端验收、桌面端导出（`make client-export`）；含纹理登记册、跨设备一致性与卡片封面美术生成 | `client/`、`Makefile`、`scripts/validate_textures.py` |
| [AI 机器人与训练](ai-bot-training.md) | `ai-core`/`ai-model`/`ai-train`/`bot-ai` 职责、C0–C3 阶段与门槛、训练与评测命令、产物落点、`BOT_POLICY` 接入与回退、重训流程 | `server/crates/ai-core`、`server/crates/ai-model`、`server/crates/ai-train`、`server/crates/bot-ai`、`make/ai.mk` |
| [Steam 接入](steam-integration.md) | 渠道抽象、Steam 登录与邀请、账号与绑定、成就契约、导出/签名/公证/上传入口 | `client/autoload/platform.gd`、`client/platform/`、`server/crates/room-service/src/{steam,phone_code,account,room,net}.rs`、`deploy/steam` |
| [部署与接入](deployment/README.md) | platform Kubernetes 接入、Helm 清单、镜像与发布入口、健康检查、客户端连接方式 | `deploy/helm`、`server/Dockerfile`、`.dockerignore`、`Makefile` |

客户端为 Godot 4 + GDScript，服务端为 Rust；工程与部署事项已确定，遗留事项见 [v0.1 待确认事项](../../versions/v0.1/open-questions.md)。

v0.1 的 M0–M3 能力均已实现：规则引擎与模拟器、房间服务与联机（含断线重连）、Godot 客户端与 macOS 桌面端导出。M2 不设真人完成率验收，断线重连一致性由自动化用例覆盖；M3 不做单独验收记录。结果见 [v0.1 验收记录](../../versions/v0.1/acceptance.md)。
