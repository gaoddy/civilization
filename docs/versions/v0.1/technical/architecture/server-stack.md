# 服务端技术基线

> 状态：M0 已实施（`rule-engine`、`sim`）；M1a 已实施（`protocol`、`room-service`，账号与社交数据已落 PostgreSQL）
> 依赖：[设计总纲](../../product/overview/design-overview.md)（§8）、[联网与技术架构](network-architecture.md)

本文记录 v0.1 服务端已确定的技术选型、工程形态与确定性约束；不描述客户端选型、接口字段与具体实现。未确定事项见[待确认事项](../../open-questions.md)。

## 1. 选型

| 项 | 选择 | 说明 |
| --- | --- | --- |
| 语言 | Rust | stable，edition 2024；工具链版本写入 `rust-toolchain.toml` 并提交 |
| 异步运行时 | tokio | 每个房间一个 task，意图在该 task 内串行处理 |
| 网络 | axum（WebSocket） | 只暴露 WebSocket 与健康检查，不引入 REST 框架 |
| 序列化 | serde + serde_json | 协议保持 JSON，见[联网与技术架构](network-architecture.md)第 5 节 |
| 随机数 | 自实现 xoshiro256++ | 不使用 `rand` 的默认算法：其实现不承诺跨版本稳定，重放会失效 |
| 数据库 | PostgreSQL + sqlx | 已落库：账号身份、好友关系、好友申请与发送记录、最近对局、房间邀请；迁移用 `sqlx::migrate!` 执行 `server/migrations/` |
| 测试 | cargo test + proptest | 确定性重放测试与属性测试，见[联网与技术架构](network-architecture.md)第 10 节 |
| 部署 | 单二进制 + Helm chart | 规则引擎与房间服务同进程发布；部署到 platform 共享 Kubernetes，单实例 |

明确不引入：ORM（Diesel、SeaORM）、缓存（Redis）、消息队列、Criterion 基准测试。理由分别是：sqlx 已覆盖查询与迁移；首版房间规模不需要外部缓存；每房间串行处理不需要跨进程队列；性能不是本项目瓶颈，基准测试无决策价值。

## 2. 工程形态

服务端为 Cargo workspace，规则引擎与房间服务通过 crate 边界隔离。当前已建 `rule-engine`（纯库）、`sim`（CLI 模拟器与确定性 AI）、`protocol`（生成的消息与视图类型）、`protocol-gen`（协议生成器）与 `room-service`（WebSocket 房间服务），当前状态见[房间服务与联网](../../../../current/technical/room-service/README.md)：

```
server/
  Cargo.toml            # workspace 与依赖版本锁定
  rust-toolchain.toml   # 工具链版本
  crates/
    rule-engine/        # 纯库：状态、意图校验、效果结算、战斗、数据表加载、逐座位视图
    protocol/           # 事件信封与消息类型，serde 派生
    protocol-gen/       # 由 schema 生成 protocol/generated/protocol.rs
    room-service/       # 可执行：会话、阶段计时、广播、重连
    sim/                # 可执行：模拟器与机器人客户端
```

依赖方向固定为 `room-service → protocol`、`room-service → rule-engine`；`rule-engine` 不依赖 `protocol`、tokio、sqlx 或任何 I/O 库。这样[联网与技术架构](network-architecture.md) §2.1 的"引擎无副作用"由编译期依赖图保证，不靠人工约束。

协议契约的唯一来源是仓库根 `protocol/schema/v1/`（JSON Schema draft 2020-12），不手写与 schema 不一致的 Rust 类型。`protocol` crate 用 `typify` 从 schema 生成 `protocol/generated/protocol.rs` 并随源提交；GDScript 常量生成到 `protocol/generated/protocol.gd`，同样随源提交。生成入口为 `make protocol-rs` / `make protocol`，漂移检测为 `make protocol-rs-check` / `make protocol-check`，详见[通信协议](protocol.md) §8。

## 3. 确定性约束

协议与规则要求见[联网与技术架构](network-architecture.md)第 4 节与第 10 节。Rust 实现额外固定以下三条：

1. **容器顺序**：结算路径禁止使用 `HashMap` / `HashSet` 的迭代结果；需要映射时用 `BTreeMap`，需要集合时用有序 `Vec`。
2. **数值类型**：结算只用整数；概率用整数权重实现，禁止浮点参与判定与比较。
3. **未定义行为**：`rule-engine` crate 顶部启用 `#![forbid(unsafe_code)]`。

随机数流按标签派生（`shop`、`combat`、`discover`、`matchmaking`、`effects`），标签与状态一起构成随机调用的唯一键；引擎只接受注入的随机数状态，不读取系统时间或全局随机源。

## 4. 运行边界

- 房间服务负责会话、阶段计时、意图排序、广播、掉线与托管，不做任何规则判断；它只调用规则引擎并把结果广播出去。
- 每房间单 task 串行处理意图；同一房间的意图进入顺序即结算顺序，不并发结算。
- 隐藏信息按[联网与技术架构](network-architecture.md)第 7.1 节的字段清单过滤后才下发，过滤发生在 `room-service`，不进入规则引擎。
- 当前实现不持久化房间、意图日志与快照：房间与对局状态只在进程内，重启即丢失；已持久化的只有账号与社交数据（好友、申请、最近对局、房间邀请），写入方式见[房间服务与联网](../../../../current/technical/room-service/README.md) §6.1。意图日志 + 种子的崩溃恢复路径见[联网与技术架构](network-architecture.md)第 9 节，尚未实现。

## 5. 未确定事项

- 服务端工程目录的具体落点与 workspace 拆分细度：搭建工程时确定，见[待确认事项](../../open-questions.md)。
- 多实例切分维度：v0.1 固定单实例，房间状态在进程内；扩展到多副本前需要拆分房间归属并改为独立迁移 Job。部署形态与环境见[部署与接入](../../../../current/technical/deployment/README.md)。
- 房间、对局状态与意图日志的持久化与恢复：M2 前确定。

## 相关文档

- [联网与技术架构](network-architecture.md)
- [客户端技术基线](client-stack.md)
- [设计总纲](../../product/overview/design-overview.md)
- [待确认事项](../../open-questions.md)
