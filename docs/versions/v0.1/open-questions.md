# 待确认事项

本文只保留 v0.1 尚未确认的事项；只登记定不了的问题，不保留讨论过程、备选方案与倾向性说明。结论确认后写入对应文档，并从本文移除。

## 产品与平衡

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 随从统一 3 金币是否让高星级卡缺乏稀缺感 | 经济 | 首版保留统一价，观察 6 星卡出现时机 | [数值与经济](product/systems/economy-and-numbers.md) |
| 是否允许「卖血升级」 | 单局节奏 | 规则允许，观察是否成为唯一解 | [数值与经济](product/systems/economy-and-numbers.md) |
| 学者「血量上浮 30%」在 6/7 星是否过高 | 平衡 | 观察，必要时降到 20% | [平衡测算](product/content/balance-analysis.md) |
| 6 星卡中学者占 5 张是否会让终局过度偏向学者 | 平衡 | 首版先观察，必要时降 1–2 张为 5 星 | [卡牌规范与数据](product/content/card-system.md) |
| 7 星 12 张中学者占 3 张（费米、海森堡、薛定谔）是否偏高 | 平衡 | 观察终局学者占比；判据 7 已设为 ≤ 30% | [平衡测算](product/content/balance-analysis.md) |
| 古典段一次开放三个关键词是否让前期复杂度陡增 | 平衡 | 观察新手首局 | [关键词与机制](product/rules/keywords-and-mechanics.md) |

### M0 基线（1000 局，seed 1，8 AI）

| 问题 | 影响范围 | 实测与判据 | 来源 |
| --- | --- | --- | --- |
| 学者流派胜率超过 60% | 平衡 | 学者 281/1000（28.1%）未达 60%；但 **生物 272/1000（27.2%）**与学者同为得分最高的两个流派（每局 1 名冠军），与「单一流派不占优」的目标不符。全部流派：学者 281、生物 272、植物 186、机械 154、人类 107，病毒与通用 0。M0 的 AI 不感知流派，数值反映 AI 行为与卡池强度叠加，M1 前复核 | [版本路线与风险](../../version-roadmap.md)、[平衡测算](product/content/balance-analysis.md) |
| 平均回合数越界 | 节奏 | 19 回合；路线图目标单局 20–30 分钟、风险表以 > 35 分钟为观察信号。M0 只有 AI 对局，回合数不能换算真人时长，暂不判定为越界，列为基线 | [版本路线与风险](../../version-roadmap.md) |
| 升时代时点偏离目标区间 | 节奏 | 首次升时代平均在第 8 回合；目标「升满七段的实测回合落在 12–14」为升满段位而非首升，1000 局报告只统计首升（8），升满时点未统计，需报告补字段后再判 | [数值与经济](product/systems/economy-and-numbers.md) |
| 7 星池每张 5 份在 8 人局是否过少 | 平衡 | 三连平均 10 次/局；未出现抽空导致死槽（商店重抽星级用例覆盖），数量是否过少仍需真人局观察 | [数值与经济](product/systems/economy-and-numbers.md) |
| 第 20 回合时存活人数 | 节奏 | 到达第 20 回合的对局在第 20 回合开始平均存活 3 人；平均回合数 19，多数局在 20 回合前结束 | [版本路线与风险](../../version-roadmap.md) |

## 技术与版本

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 服务端部署编排与多实例切分维度 | 部署 | M2 联机内测前确定 | [服务端技术基线](technical/architecture/server-stack.md) |
| 客户端工程目录与构建导出流程未定 | 实现 | 搭建客户端工程前确定 | [客户端技术基线](technical/architecture/client-stack.md) |
| 客户端测试框架未定 | 实现 | 搭建客户端工程前确定 | [客户端技术基线](technical/architecture/client-stack.md) |
| GDScript 生成器是否值得长期维护 | 实现 | 先用最小生成物（常量与字段名）观察；维护成本高于收益时退回手写常量并由样例测试兜底 | [通信协议](technical/architecture/protocol.md) |
| 观战只读视图的字段裁剪 | 实现 | v0.1 不做观战，视图按可被第三方只读消费设计，裁剪规则待观战时确定 | [通信协议](technical/architecture/protocol.md) |
| 协议 crate 生成物位置 | 实现 | M0 只建了 `rule-engine` 与 `sim` 两个 crate，`protocol` crate 尚未建立；M1 统一为仓库根 `protocol/generated/protocol.rs` 生成、crate 依赖生成文件，并回写 [服务端技术基线](technical/architecture/server-stack.md) | [协议设计](../../superpowers/specs/2026-11-15-protocol-design.md) §8 |
| 单目标选择的意图参数 | 实现 | M0 引擎意图带 `target: Option<InstanceId>` 由 AI 自动选择；M1 给 `BuyMinion`、`PlayCard` 加可选 `targetMinionId`，同步 schema、样例、生成物与核心规则 | [核心规则](product/rules/core-rules.md) §3 |
| 发现的选择消息 | 实现 | 核心规则 §3 的九种意图没有「选择发现结果」；M0 用引擎内部 `ChooseDiscover`（越界返回 `INVALID_CHOICE`），M1 在协议中新增同义意图并同步 schema、样例、生成物与核心规则 | [核心规则](product/rules/core-rules.md) §3、§6 |
| `PlayerView` 字段不足 | 实现 | 当前 schema 不能表达对手阵容、淘汰名次、当前对手与托管状态；M1 按「先改 schema + 样例，再改两端」补齐，并与引擎视图映射同批落地 | [协议设计](../../superpowers/specs/2026-11-15-protocol-design.md) |
| 招募结束与遗留待选的先后 | 实现 | M0 允许仍有发现待选时提交 `EndRecruit`；M1 协议层再定是否拒绝该顺序 | [核心规则](product/rules/core-rules.md) §6 |

## 文档结构

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 仓库缺少根 `AGENTS.md` 的协作原则、runbook 位置声明与项目事实 | 仓库治理 | 按项目接入流程补建；缺用户规模与端到端测试入口，不能留空占位 | 仓库根目录 |
