# 服务端技术选型 Rust 设计

> 状态：已确认

## 意图与原因

用户决定 v0.1 服务端使用 Rust。此前所有文档统一标注「服务端技术栈尚未确定」，兜底默认方案是 Node（`version-roadmap.md` §4 风险表与 §5 决策点均写死"若未选型则强制采用 Node"）。这条决定替换该兜底，让技术基线、待确认事项与版本路线保持一致。

本轮只出文档，不搭建服务端工程，不改动客户端选型。

## 目标

- 在 v0.1 技术基线中新增服务端技术基线，记录语言、运行时、网络、序列化、随机数、数据库、测试与部署形态。
- 固定服务端工程形态与依赖方向，使"规则引擎无副作用"由 crate 边界保证。
- 把 Rust 特有的确定性约束写进基线，避免实现阶段因容器顺序或浮点引入不可重放。
- 清除文档中「服务端技术栈未定」与 Node 兜底的全部表述。

## 非目标

- 不创建 Cargo workspace、不写 `Cargo.toml`、不引入任何依赖。
- 不决定具体 Rust 小版本（由 `rust-toolchain.toml` 在实际搭建时锁定）。
- 不设计数据库表结构、迁移文件与接口字段。
- 不调整 22–31 人周的排期估算。
- 不决定多实例部署的切分维度（M2 前再定）。

## 约束

- 规则引擎只部署在服务端，不读取时钟、环境变量、网络与全局随机；输入为 `state`、`intent`、`rng`，输出为新 `state` 与 `events`。
- 每房间单 task 串行处理意图，保证结算顺序确定。
- 协议为 WebSocket + JSON，事件信封与消息清单沿用 `network-architecture.md` 第 5 节。
- 确定性要求沿用 `network-architecture.md` 第 4 节与第 10 节。

## 方案

| 方案 | 内容 | 取舍 |
| --- | --- | --- |
| A | 只就地改写「服务端待定」相关句子 | 改动最小，但选型结论没有正式落点；`network-architecture.md` 明确定位为"不描述技术选型"，结论会散落在产品文档里 |
| B（选定） | 就地改写 + 新增 `technical/architecture/server-stack.md` | 与客户端基线对称，选型有独立承载文档；索引与待确认项同步 |
| C | 新建 `technical/server/` 目录拆多份文档 | 当前没有服务端代码，属于过度设计 |

### 备选语言

| 语言 | 优势 | 代价 |
| --- | --- | --- |
| Go | 每房间一个 goroutine 天然串行；WebSocket/JSON 生态成熟；编译与迭代快 | 无 sum type，`Intent`/`Event`/`Effect` 用 interface + type switch，漏分支编译期不报错 |
| Rust（选定） | enum + `match` 与 `reduce(state, intent, rng)` 形状一致，漏分支编译期报错；内存与并发安全免检 | 迭代慢（借用检查与编译时间），异步 WebSocket 需要拼装；22–31 人周未含学习与编译成本，M0 会变慢 |
| Node（原兜底） | 无 | 用户已排除 |

选定 Rust 的理由是规则引擎的效果分支是本项目最易写漏的地方，编译期穷尽检查的收益高于迭代速度损失；且用户具备 Rust 熟练度。代价是 M0 会更慢，本设计不改动既有估算数字。

## 设计

### 新增 `docs/versions/v0.1/technical/architecture/server-stack.md`

标注「方案未实施」，内容固定为：

1. 选型表：Rust stable + edition 2024（工具链版本由 `rust-toolchain.toml` 锁定）、tokio、axum WebSocket、serde + serde_json、自实现 xoshiro256++、PostgreSQL + sqlx、cargo test + proptest、单二进制 + Docker。
2. 明确不引入：ORM、Redis、消息队列、Criterion。
3. 工程形态：Cargo workspace，`rule-engine`（纯库）/ `protocol`（消息类型）/ `room-service`（可执行）；依赖方向 `room-service → protocol`、`room-service → rule-engine`，`rule-engine` 不依赖 I/O 库。
4. 确定性约束：禁用 `HashMap`/`HashSet` 迭代序、结算只用整数与整数权重、`#![forbid(unsafe_code)]`。
5. 运行边界：房间服务不做规则判断、每房间串行、隐藏信息在 `room-service` 过滤、持久化以意图日志 + 种子为准。
6. 未确定事项：工程目录落点、部署编排、表结构与迁移。

### 就地改写

- `design-overview.md` §8 引导句去掉「服务端技术栈待定」，指向服务端技术基线；§9 第一条删去「服务端语言与部署形态」，保留部署形态为待定。
- `network-architecture.md` 头部注记改为服务端与客户端均已确定。
- `technical/README.md`：补服务端选型结论与链接。
- `technical/architecture/README.md`：索引新增文档，去掉「服务端选型未定」。
- `open-questions.md`：删除「服务端技术栈未定：Node 或 Go」；新增工程目录落点与部署编排两条待确认。
- `version-roadmap.md`：§4 删除「服务端技术栈未定导致架构返工」风险行；§5 删除「服务端技术选型」决策点。
- `versions/v0.1/README.md`：能力清单增加服务端行。

## 留给实现者

- Rust 小版本、workspace 目录落点与拆分细度：搭建工程时确定。
- 数据库表结构、迁移文件与索引：进入 M1 前按数据模型设计。
- 部署编排与多实例切分维度：M2 联机内测前确定。
- 22–31 人周是否因 Rust 重估：M0 前按实际进度重估。
