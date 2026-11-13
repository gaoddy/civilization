# 执行台账 — 计划：docs/superpowers/plans/2026-11-14-civilization-banner-docs.md

项目不是 git 仓库，故跳过 SDD 脚本与提交步骤，以本文件记录进度。

Pre-flight: 任务间共享接口为「状态名/意图名」「10 个关键词」「五时代表」「星级数值区间」「CSV 表头」。均在 00-设计总纲 中已锁定，无冲突。

Task 1: complete (01-概念与定位.md, 8 headings, keywords ok)
Task 2: complete (02-核心规则.md, 10 subsections, intents ok)
Task 3: complete (03-关键词与机制.md, 37 table rows, 10 keywords ok)
Task 4: complete (04-时代与场景.md, 36 table rows, 5 eras ok)
Task 5: complete (08-数值与经济.md, 82 table rows, 金币/费用/概率/区间齐备)
Task 6: complete (05-卡牌规范与数据.md + 卡牌数据.csv, 120 张, 学者攻击 0, 分布达标)
Task 6: Ruling: 计划要求“至少 80 张卡名来自素材”，实际 50 张 — 素材随从名在时代上分布极不均衡（史前/原始/未来几乎无可用名），保证每时代卡量必须新增 — 代价是新增卡占比高，需在文案层补来源说明
Task 7: complete (06-英雄与技能.md + 英雄数据.csv, 10 英雄, 30 战术牌)
Task 8: complete (07-联网与技术架构.md, 8+ subsections, 5 协议消息齐备)
Task 9: complete (09-模式与成就.md, 模式/成就表齐备)
Task 10: complete (10-版本规划与风险.md, M0-M4 + 11 条风险)
Task 11: complete (README.md + 总纲状态回填, 14 个文件, 卡片 120/英雄 10/学者攻击 0, 无待写)
Task 11: Ruling: “grep 待写”命中 2 处均在计划文件自身 — 计划里的检查命令描述了该字符串 — 不代表文档未完成 — 代价是无

## 终审与修复

Final: fixed 伤害公式口径（Critical）— 00/02/08 统一为「败者时代 + 向上取整(胜方存活星级和/2)」，上限 15 — 回归：DATA_OK 0 违规
Final: fixed 学者隐性输出（Critical）— m20 居里夫人战吼改为「使一个友方随从获得两次成长」— 回归：scholar_atk 0
Final: fixed 第 11 个关键词「光环」（Critical）— CSV 11 处改为 `无`，03/05 明确光环不是关键词 — 回归：kw 违规 0
Final: fixed PlayCard 二次收费（Important）— 费用在获得牌时支付，PlayCard 不再扣费
Final: fixed 招募阶段指定敌方目标（Important）— 02 §4.4 新增目标规则；h03/h05 与 14 张战术牌改为己方目标或战斗开始时结算
Final: fixed 升级降价口径（Important）— 按「当前时代已停留回合数」递减，最早第 3 回合可升
Final: fixed 4 星攻击力越界（Important）— f21/m13 攻击 3→4
Final: fixed 协议缺口（Important）— 07 补 TimerWarning，02 补 NO_VALID_TARGET/TARGET_NOT_ALLOWED
Final: fixed 成就开关矛盾（Important）— 00 明确场景/陨落/坐镇默认关、成就默认开
Final: fixed 图灵时刻不可判定（Important）— 改为「机械随从同时拥有最优攻击与连击」
Final: fixed 4 金费用表缺失（Important）— 08 补 4 金档
Final: fixed 命名空间冲突（Important）— 05 明确唯一性只约束随从池；三张病毒战术牌改名
Final: fixed 剩余 Minor — 51 张新增、总纲英雄/战术牌来源表述、06 同名英雄清单、README 陨落链接、胜率阈值 58→60

回归：120 卡 / 10 英雄 / 0 数值越界 / 0 关键词违规 / 14 个文件 / 无旧数值残留

## 2026-11-14 卡牌深化与平衡测算（计划：2026-11-14-card-depth.md）

Pre-flight: 任务 1→2→3→4 共享接口为「三个新词的名称与折算值」「CSV 表头」「08 星级区间」。词表在 Task 1 一次性锁定，后续只引用，无冲突。

Task 1: complete (00 §6.8 / 03 词表+§1.1 / 08 §5 折算, 13 词)
Task 2: complete (70 张新增卡重做, 装配 14 / 播种 9 / 进化 6, CSV 120 行 0 违规)
Task 2: Ruling: 为使 6 个流派在每个星级都有可用卡，调整了 8 张卡的星级/数值 — 原分布下人类无 6 星、植物无 3 星、机械无 1 星，会导致某流派在对应时代整段缺席 — 代价是 8 张卡与素材原始数值不同（均为新增/调整过的卡，未触碰 50 张素材卡）
Task 3: complete (11-平衡测算.md, 65 表格行, 6×6 矩阵 + 6 套阵容 + 6 条失衡判据)
Task 4: complete (README 加 11 条目与阅读顺序 / 00 §1 加 11 / 04 §6 措辞修正 / 05 §6 全量卡表按 CSV 重生成)

回归：15 个文件 / 120 卡 / 时代 24-24-24-30-18 / 流派 24-22-24-12-10-28 / 星级 18-24-26-22-18-12 / 0 数值越界 / 0 关键词违规 / 0 超模卡 / 无「10 个关键词」残留
