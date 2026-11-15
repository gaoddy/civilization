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
| 学者流派胜率超过 60% | 平衡 | 两个 60% 口径分开看：① 路线图风险表「学者体系胜率 > 60%」——本次 1000 局 AI 对局只测了夺冠率，学者夺冠 281/1000（28.1%，`win_rate_permille_by_tribe.学者` = 281‰），未达 60%；生物 272（272‰）与之并列最高，病毒与通用 0。② [平衡测算](product/content/balance-analysis.md) 判据 4「流派进前四率 40%–60%」——本次**未统计进前四率**，报告没有该字段，不能据此判定。AI 不感知流派，夺冠率反映 AI 行为与卡池强度的叠加，M1 前需补进前四率并复核 | [版本路线与风险](../../version-roadmap.md)、[平衡测算](product/content/balance-analysis.md) |
| 平均回合数越界 | 节奏 | 19 回合（`avg_turns`）；路线图目标单局 20–30 分钟、风险表以 > 35 分钟为观察信号。M0 只有 AI 对局，回合数不能换算真人时长，暂不判定为越界，列为基线 | [版本路线与风险](../../version-roadmap.md) |
| 升时代时点偏离目标区间 | 节奏 | 目标「升满七段的实测回合落在 12–14」指升满段位；报告 `era_upgrade_turns_by_seat` / `avg_era_upgrade_turn` 只统计每座位**首次**升时代，**平均均为向下取整**（如座位 0 为 4500 / 1000 = 4.5，报告中显示 4）。实测：各座位平均首次升时代回合为 [4, 2, 4, 4, 4, 6, 4, 4]，全座位平均第 8 回合（4126 条记录，`era_upgrade_count`；1000 局中有 12 局无人升级，不计入平均）。升满时点未统计，需报告补字段后再对 12–14 判定 | [数值与经济](product/systems/economy-and-numbers.md) |
| 7 星池每张 5 份在 8 人局是否过少 | 平衡 | 三连平均 10 次/局（`triples_per_game`）；商店按星级无可用卡时的重抽路径有单元测试覆盖（`recruit::reroll_with_empty_star_retries_then_leaves_slot_empty`），但没有报告字段统计 1000 局中的空星级重抽与空槽，不能据此断言 1000 局无死槽；数量是否过少仍需真人局观察 | [数值与经济](product/systems/economy-and-numbers.md) |
| 第 20 回合时存活人数 | 节奏 | 两个计数定义不同：`games_reaching_turn_20` 统计「第 20 回合招募开始前仍有存活玩家的局」（含在第 20 回合前决出唯一幸存者、然后进入第 20 回合的局），`games_ended_on_turn_20` 统计「结束回合 = 20 的局」；本次两者都是 979 只是样本恰好没有此类提前结束局，属于巧合，不代表定义等价。979 局在第 20 回合招募开始前平均存活 `survivors_at_turn_20` = 3 人（向下取整，存活人数 2–6 人），另 21 局在第 19 回合或更早结束；`avg_turns` 19 为 1000 局平均。结束时存活人数 `avg_survivors_at_end` = 2（向下取整；1–5 人，平均 2.693）：未决出唯一幸存者且已到第 20 回合时，`resolve_settle` 按 生命 → 上战胜者 → 累计伤害 → 座位 排序后直接 `GameOver`，不淘汰剩余玩家，因此结束时仍有存活者 | [版本路线与风险](../../version-roadmap.md) |

## 技术与版本

> 已在 M1 解决并从下表移除：「单目标选择的意图参数」——`PlayCard` 已带可选 `targetMinionId`，schema、样例、生成物与核心规则同步（`BuyMinion` 买入一律入手牌，不需要目标）；「发现的选择消息」——`ChooseDiscover` 已是协议十种意图之一，服务层映射为引擎意图并消费 `PendingChoice`；「缺少目标时的服务端校验」——`NO_VALID_TARGET` 已由引擎在意图执行前返回，客户端与机器人补选路径端到端生效。

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 服务端部署编排与多实例切分维度 | 部署 | M2 联机内测前确定 | [服务端技术基线](technical/architecture/server-stack.md) |
| GDScript 生成器是否值得长期维护 | 实现 | 先用最小生成物（常量与字段名）观察；维护成本高于收益时退回手写常量并由样例测试兜底 | [通信协议](technical/architecture/protocol.md) |
| 观战只读视图的字段裁剪 | 实现 | v0.1 不做观战，视图按可被第三方只读消费设计，裁剪规则待观战时确定 | [通信协议](technical/architecture/protocol.md) |
| `TARGET_NOT_ALLOWED` 未校验敌我 | 实现 | 招募阶段客户端拿不到敌方实例编号（§4.4），正常流程不会提交敌方目标，该错误码因此从未构造；但 `instanceId` 全局递增，恶意客户端可猜测编号指向敌方随从，服务端目前不校验敌我、会把效果实际作用于该随从。是否补校验待定 | [核心规则](product/rules/core-rules.md) §4.4 |
| 协议 crate 生成物位置 | 实现 | M0 只建了 `rule-engine` 与 `sim` 两个 crate，`protocol` crate 尚未建立；M1 统一为仓库根 `protocol/generated/protocol.rs` 生成、crate 依赖生成文件，并回写 [服务端技术基线](technical/architecture/server-stack.md) | [协议设计](../../superpowers/specs/2026-11-15-protocol-design.md) §8 |
| `PlayerView` 字段不足 | 实现 | 当前 schema 不能表达对手阵容、淘汰名次、当前对手与托管状态；M1 按「先改 schema + 样例，再改两端」补齐，并与引擎视图映射同批落地 | [协议设计](../../superpowers/specs/2026-11-15-protocol-design.md) |
| 招募结束与遗留待选的先后 | 实现 | M0 允许仍有发现待选时提交 `EndRecruit`；M1 协议层再定是否拒绝该顺序 | [核心规则](product/rules/core-rules.md) §6 |
| M0 重放未比对事件序列 | 实现 | `make m0-determinism` 与 `sim replay` 只比对终局哈希与排名，未比对事件序列；规格的「确定性」要求事件序列一致，M1 前补事件序列摘要比对 | [规则引擎与模拟器](technical/rule-engine-and-simulator.md) |
| `survivors_at_turn_20` 采样时点 | 实现 | 实现中在 `drive` 循环内、第 20 回合 `resolve_settle` 返回后（检查 `state.turn == 20`）采样，即第 20 回合结算后的存活人数；文档原写「招募开始前」，与实现及 `games_reaching_turn_20` = `games_ended_on_turn_20` 的观测一致，但语义需确认（是否改为第 20 回合招募开始前或回合开始前采样） | [规则引擎与模拟器](technical/rule-engine-and-simulator.md) |

## 文档结构

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 仓库缺少根 `AGENTS.md` 的协作原则、runbook 位置声明与项目事实 | 仓库治理 | 按项目接入流程补建；缺用户规模与端到端测试入口，不能留空占位 | 仓库根目录 |
