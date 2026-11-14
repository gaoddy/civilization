# 待确认事项

本文汇总 v0.1 尚未确认的产品、技术、内容与文档结构事项；结论确认后写入对应文档，并从本文移除。

## 产品与内容

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 8 人房间人数不足时，AI 补位是否计入排名与 MMR | 匹配与赛季设计 | 默认计入排名，标注「AI 对局」不计 MMR | [设计总纲](product/overview/design-overview.md) §7 |
| 首版是否需要新手引导与 AI 教学局 | 首版工作量约 15% | 默认需要，做 3 回合引导 | [设计总纲](product/overview/design-overview.md) §7 |
| 中文卡名与英文卡名的对应是否首版就要 | 数据库结构 | 默认首版只做中文，字段留空 | [设计总纲](product/overview/design-overview.md) §7 |
| 赛事与观战是否首版 | 服务端设计 | 默认不做，快照按可观战设计 | [设计总纲](product/overview/design-overview.md) §7 |
| 素材中「教育」定位是否显性化 | UI 与内容运营 | 默认卡牌详情页附一句真实史实，不做课程化 | [设计总纲](product/overview/design-overview.md) §7 |
| 是否在客户端内置「这张卡是谁」的科普浮层 | UI 与内容 | 倾向做，只放一句史实 | [概念与定位](product/overview/concept-and-positioning.md) |
| 「跨时代同桌」是否需要叙事解释 | 文案与世界观 | 倾向用一句话设定带过 | [概念与定位](product/overview/concept-and-positioning.md) |
| 镜像幽灵是否保留随从身上的临时增益 | 战斗规则 | 倾向不保留，只保留永久属性 | [核心规则](product/rules/core-rules.md) |
| 冻结商店在升时代后是否仍然有效 | 商店规则 | 倾向有效，重抽后槽位保持冻结 | [核心规则](product/rules/core-rules.md) |
| 传染造成的死亡是否算「被该随从击杀」 | 击杀类效果 | 倾向算；首版没有击杀类效果，暂不实现 | [关键词与机制](product/rules/keywords-and-mechanics.md) |
| 金色随从「战吼触发两次」是否包含召唤类效果 | 卡牌文本 | 倾向是，需要逐卡确认 | [关键词与机制](product/rules/keywords-and-mechanics.md) |
| 未来社会 5 星与 6 星池的分配比例 | 商店概率 | 首版按 60/40，上线后用遥测校准 | [时代与场景](product/rules/eras-and-scenarios.md) |
| 场景是否在排位模式中完全随机 | 模式规则 | 倾向排位随机、比赛固定为「陆地」 | [时代与场景](product/rules/eras-and-scenarios.md) |
| 「新增」卡占比 58% 是否需要在文案层补科学史出处 | 文案与内容 | 倾向每张新增卡写一句来源说明 | [卡牌规范与数据](product/content/card-system.md) |
| 6 星卡中学者占 7 张是否会让终局过度偏向学者 | 平衡 | 首版先观察，必要时降 1–2 张为 5 星 | [卡牌规范与数据](product/content/card-system.md) |
| 英雄与随从同名是否会造成混淆 | UI | 倾向保留，UI 用边框区分 | [英雄与技能](product/content/hero-system.md) |
| 达尔文的「发现」是否应该在首版加入 | 平衡与确定性 | 倾向保留，限制每回合 1 次 | [英雄与技能](product/content/hero-system.md) |
| 随从统一 3 金币是否让高星级卡缺乏稀缺感 | 经济 | 首版保留统一价，观察 6 星卡出现时机 | [数值与经济](product/systems/economy-and-numbers.md) |
| 是否允许「卖血升级」 | 单局节奏 | 规则允许，观察是否成为唯一解 | [数值与经济](product/systems/economy-and-numbers.md) |
| 成就是否应该在比赛模式中关闭 | 模式规则 | 倾向关闭 | [模式与成就](product/systems/modes-and-achievements.md) |
| 大决战模式的 10 个棋盘位是否需要独立商店概率表 | 模式规则 | 倾向复用会战概率，只改棋盘位与伤害公式 | [模式与成就](product/systems/modes-and-achievements.md) |
| 英雄坐镇是否需要与「陨落」绑定 | 扩展层 | 设计上成立；首版两者都关闭，暂不实现 | [模式与成就](product/systems/modes-and-achievements.md) |
| 平衡推演未考虑 8 人局的牌池争抢 | 平衡 | 需要实测校准 | [平衡测算](product/content/balance-analysis.md) |
| 学者「血量上浮 30%」在 6 星是否过高 | 平衡 | 观察，必要时降到 20% | [平衡测算](product/content/balance-analysis.md) |
| 病毒是否需要对召唤物生效 | 平衡 | 取决于实测里病毒是否稳定垫底 | [平衡测算](product/content/balance-analysis.md) |

## 技术与版本

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 技术栈未定：客户端 Web（TypeScript + Canvas/DOM），服务端 Node 或 Go | 架构与实现 | 由用户选型；选型前只做与语言无关的规则引擎 | [联网与技术架构](technical/architecture/network-architecture.md) |
| 是否需要「观战延迟 30 秒」 | 服务端设计 | 首版不做观战，暂不决定 | [联网与技术架构](technical/architecture/network-architecture.md) |
| 对局状态持久化到数据库，还是只存意图日志与种子 | 持久化 | 倾向只存意图日志与种子，快照只做进程内恢复 | [联网与技术架构](technical/architecture/network-architecture.md) |
| 首版是否必须做移动端 | UI 与工作量 | 倾向不做，布局预留 16:9 与竖屏 | [版本规划与风险](../../version-roadmap.md) |
| 是否引入「赛季」与卡牌轮换 | 版本规划 | 倾向第二版再引入，首版全卡常驻 | [版本规划与风险](../../version-roadmap.md) |
| 人周估算是否需要按「是否有全职美术」重算 | 排期 | 倾向在 M1 结束后重新估算 | [版本规划与风险](../../version-roadmap.md) |

## 文档结构

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 仓库缺少根 `AGENTS.md` 的协作原则、runbook 位置声明与项目事实 | 仓库治理 | 按项目接入流程补建；用户规模等产品数据需先确认，不能留空占位 | 仓库根目录 |
