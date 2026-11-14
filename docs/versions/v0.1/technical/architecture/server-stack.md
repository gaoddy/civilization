# 服务端技术基线

> 状态：方案未实施
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
| 数据库 | PostgreSQL + sqlx | 账号、好友、房间元数据、意图日志与遥测；迁移用 `sqlx migrate` |
| 测试 | cargo test + proptest | 确定性重放测试与属性测试，见[联网与技术架构](network-architecture.md)第 10 节 |
| 部署 | 单二进制 + Docker | 规则引擎与房间服务同进程发布；初期单实例 |

明确不引入：ORM（Diesel、SeaORM）、缓存（Redis）、消息队列、Criterion 基准测试。理由分别是：sqlx 已覆盖查询与迁移；首版房间规模不需要外部缓存；每房间串行处理不需要跨进程队列；性能不是本项目瓶颈，基准测试无决策价值。

## 2. 工程形态

服务端为 Cargo workspace，规则引擎与房间服务通过 crate 边界隔离：

```
server/
  Cargo.toml            # workspace 与依赖版本锁定
  rust-toolchain.toml   # 工具链版本
  crates/
    rule-engine/        # 纯库：状态、意图校验、效果结算、战斗、数据表加载
    protocol/           # 事件信封与消息类型，serde 派生
    room-service/       # 可执行：会话、阶段计时、广播、重连、持久化
```

依赖方向固定为 `room-service → protocol`、`room-service → rule-engine`；`rule-engine` 不依赖 `protocol`、tokio、sqlx 或任何 I/O 库。这样[联网与技术架构](network-architecture.md) §2.1 的"引擎无副作用"由编译期依赖图保证，不靠人工约束。

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
- 持久化以意图日志 + 种子为准，快照只用于进程崩溃恢复；恢复路径见[联网与技术架构](network-architecture.md)第 9 节。

## 5. 未确定事项

- 服务端工程目录的具体落点与 workspace 拆分细度：搭建工程时确定，见[待确认事项](../../open-questions.md)。
- 部署编排（容器编排方式、单实例到多实例的切分维度）：M2 联机内测前确定。
- 数据库表结构与迁移文件：进入 M1 前按数据模型另行设计。

## 相关文档

- [联网与技术架构](network-architecture.md)
- [客户端技术基线](client-stack.md)
- [设计总纲](../../product/overview/design-overview.md)
- [待确认事项](../../open-questions.md)
