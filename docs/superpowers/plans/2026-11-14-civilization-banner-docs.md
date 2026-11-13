# 《文明战旗》设计文档集 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 产出 00–10 共 11 篇中文设计文档 + 2 份 CSV 数据表，使《文明战旗》达到"可直接进入开发"的规格完整度。

**Architecture:** 以 `00-设计总纲.md` 为唯一规则基线，分册只细化不推翻。先写"规则 → 关键词 → 时代 → 数值"这条依赖链，再写"卡牌 → 英雄"内容层，最后写"架构 → 模式 → 规划"工程层。卡牌与英雄同时落一份 CSV，使文档可被机器校验（行数、字段、数值区间、重名）。

**Tech Stack:** Markdown（UTF-8，中文正文）+ CSV（UTF-8，表头英文键名）。不引入代码、不引入构建工具。**本项目当前不是 git 仓库，因此计划中不含提交步骤；若后续初始化仓库，以每个任务为一次提交。**

**Spec:** `/Users/gaoddy/Projects/Civilization/docs/00-设计总纲.md`

## Global Constraints

- 全部正文使用简体中文；代码、字段名、关键词英文名保持原样。
- 所有规则数值必须与 `00-设计总纲.md` 一致；发现总纲有误时，先改总纲并在分册中注明，不允许分册自带另一套数值。
- 关键词只有 10 个（成长、繁衍、战吼、亡语、复生、嘲讽、护盾、连击、传染、最优攻击），不得新增同义词。
- 两套等级不得混用：**商店等级 = 时代 1–5**，**随从星级 = 1–6**。任何文档写"等级"必须能一眼看出是哪一套。
- 学者硬规则：攻击力恒 0，任何攻击力加成都无效。
- 一切随机必须可播种复现；文档中不得出现"由主持人裁定""视情况而定"。
- 文件落点：`docs/` 下按编号命名；数据表放 `docs/data/`。
- 每篇文档开头必须有：标题、编号、依赖文档、状态（草案/已评审）；结尾必须有：待决问题（可为空）。

## Review Focus

1. **两套等级混用** —— 读者看到"等级 5"无法判断是时代还是星级；每处出现都必须带限定词。
2. **学者被隐性地赋予攻击力** —— 卡牌数值表若给学者填了攻击力，或成长效果写成 +1/+1，规则即破。
3. **伤害公式口径漂移** —— 08 分册的曲线模型若用"回合数对应的时代"代替"已升到的时代"，整局节奏与总纲冲突。
4. **牌池与流派轮换不一致** —— 05 的 120 张卡若包含未启用流派的计算方式错误，共享池份数会算错。
5. **素材卡重名/重复** —— 孔子、米开朗基罗、爱德华·詹纳在素材中各出现两次，CSV 必须去重且保留合并说明。

---

### Task 1: 分册 01 · 概念与定位

**Files:**
- Create: `docs/01-概念与定位.md`

**Interfaces:**
- Consumes: `00-设计总纲.md` §2 意图、§3 目标/非目标/反目标。
- Produces: 项目的一句话定义、目标用户、核心循环图（文本）、体验支柱、竞品对比；后续所有文档引用这里定义的产品气质。

- [ ] **Step 1: 写文档骨架**

必须包含章节：`## 一句话定义`、`## 目标用户`、`## 核心循环`、`## 体验支柱`、`## 与竞品的差异`、`## 世界观与基调`、`## 不做什么`、`## 待决问题`。

- [ ] **Step 2: 写正文**

- 一句话定义须同时出现三个关键词：酒馆战棋、文明时代、科学史。
- 核心循环用"招募 → 战斗 → 结算 → 升时代"四段文本流程图，标注每段的玩家决策。
- 体验支柱恰好 3 条，每条给出"如果这条被破坏，游戏会变成什么"。
- 竞品对比表列 3 行：炉石酒馆战棋、自走棋（Dota Underlords）、文明 6；每行写"我们相同 / 我们不同"。

- [ ] **Step 3: 校验**

Run: `grep -c '^## ' docs/01-概念与定位.md`
Expected: `8`（含待决问题）

Run: `grep -q '酒馆战棋' docs/01-概念与定位.md && grep -q '科学史' docs/01-概念与定位.md && echo ok`
Expected: `ok`

---

### Task 2: 分册 02 · 核心规则

**Files:**
- Create: `docs/02-核心规则.md`

**Interfaces:**
- Consumes: 总纲 §6.1–§6.5、§6.11、§6.12。
- Produces: 房间/阶段状态机、每个玩家意图的名称与前置条件、战斗结算伪代码、三连与发现流程。03/07 分册必须复用这里的状态名与意图名。

- [ ] **Step 1: 写状态机与意图清单**

状态名固定为：`Lobby`、`Recruit`、`Combat`、`Settle`、`GameOver`。玩家意图固定为：`BuyMinion`、`SellMinion`、`RerollShop`、`UpgradeEra`、`PlayCard`、`RearrangeBoard`、`FreezeShop`、`UseHeroPower`、`EndRecruit`。每个意图给出：前置条件、扣除资源、副作用、失败返回码（如 `NOT_ENOUGH_GOLD`）。

- [ ] **Step 2: 写战斗结算伪代码**

必须覆盖：指针交替推进、嘲讽筛选、最优攻击、护盾吸收、连击、反击、亡语与复生触发时机、传染结算。用编号列表写出固定结算顺序，顺序不得留空。

- [ ] **Step 3: 写阶段与异常规则**

必须覆盖：招募超时（自动 `EndRecruit`）、断线（AI 托管）、棋盘满、手牌满、牌池耗尽、单人对镜像幽灵、8 人淘汰与排名、第 20 回合强制结束。

- [ ] **Step 4: 校验**

Run: `grep -c '^### ' docs/02-核心规则.md`
Expected: 至少 `8`

Run: `for s in Lobby Recruit Combat Settle GameOver BuyMinion RerollShop UpgradeEra UseHeroPower; do grep -q "$s" docs/02-核心规则.md || echo "MISSING $s"; done`
Expected: 无输出

---

### Task 3: 分册 03 · 关键词与机制

**Files:**
- Create: `docs/03-关键词与机制.md`

**Interfaces:**
- Consumes: 总纲 §6.6、§6.8。
- Produces: 10 个关键词的精确判定、触发时机表、叠加与覆盖规则、效果文本模板、学者特例。Task 6 的卡牌文本只能使用这里定义的词与模板。

- [ ] **Step 1: 写关键词判定表**

表头固定为：`关键词 | 英文名 | 触发时机 | 作用对象 | 叠加规则 | 金色时变化 | 学者是否生效 | 文本模板`。10 行，一行一个，不得合并。

- [ ] **Step 2: 写结算顺序与状态规则**

必须覆盖：同一时机多个效果按"站位从左到右"结算；临时增益的清除时点；传染的层数与每回合扣血；成长在学者身上的实际结果（+0/+1）；护盾与复生的交互；金色随从的"效果触发两次"具体指哪些关键词。

- [ ] **Step 3: 写反例**

至少 6 个"错误写法 → 正确写法"对照，专治素材里的"加成"混用。例：`+2 攻击力和生命值` → `成长 2 次`；`让随从获得一次成长` 保留为合法写法。

- [ ] **Step 4: 校验**

Run: `grep -c '^| ' docs/03-关键词与机制.md`
Expected: 至少 `13`（表头 + 分隔 + 10 行）

Run: `for k in 成长 繁衍 战吼 亡语 复生 嘲讽 护盾 连击 传染 最优攻击; do grep -q "$k" docs/03-关键词与机制.md || echo "MISSING $k"; done`
Expected: 无输出

---

### Task 4: 分册 04 · 时代与场景

**Files:**
- Create: `docs/04-时代与场景.md`

**Interfaces:**
- Consumes: 总纲 §6.2、§6.3、§6.7、§6.9。
- Produces: 五时代的名称/商店等级/升级费用/解锁机制/卡池开放范围；8 个首版场景及其精确加成。Task 7（数值）与 Task 6（卡牌）引用此表。

- [ ] **Step 1: 写时代表**

表头固定：`时代 | 商店等级 | 升级费用 | 开放星级 | 该时代解锁机制 | 素材来源`。5 行。每时代给 1 条"独有机制"（例：史前=繁衍、现代=机械最优攻击），机制必须能在 03 的关键词表里找到落点，或明确标注为"新增机制，需在 03 补条目"。

- [ ] **Step 2: 写 8 个场景**

表头固定：`场景 | 来源素材 | 加成对象 | 精确效果 | 首版是否启用`。从素材 16 个场景中选 8 个（陆地、海洋、天空、高原、沙漠、丛林、草原、动物园），效果必须写成可结算规则（例："丛林：生物与植物随从进入战场时获得一次成长"），不得写"增强"。

- [ ] **Step 3: 校验**

Run: `grep -c '^| ' docs/04-时代与场景.md`
Expected: 至少 `18`

Run: `for e in 史前 原始 封建 现代 未来; do grep -q "$e" docs/04-时代与场景.md || echo "MISSING $e"; done`
Expected: 无输出

---

### Task 5: 分册 08 · 数值与经济

**Files:**
- Create: `docs/08-数值与经济.md`

**Interfaces:**
- Consumes: 总纲 §6.2、§6.3、§6.4、§6.10。
- Produces: 金币曲线、商店概率表、牌池份数表、卡牌星级数值区间表、单局节奏模型。Task 6（卡牌数据）必须用手册的区间表校验每张卡。

- [ ] **Step 1: 写经济与概率表**

必须包含：回合金币表（1–20 回合）、升级费用与每回合降价表、商店 6 槽按时代×星级的出现概率矩阵（5×6 或说明未开放格）、每张卡的牌池份数表（星级 1–6）、刷新与出售的收支模型。

- [ ] **Step 2: 写数值区间表**

表头固定：`星级 | 攻击力区间 | 生命值区间 | 参考白板 | 说明`。6 行，首版区间取：1 星 1–2/2–4，2 星 2–3/3–5，3 星 3–5/4–7，4 星 4–7/6–9，5 星 6–9/8–12，6 星 8–12/10–16。学者行单独说明"攻击力固定 0，生命值取区间上浮 30%"。

- [ ] **Step 3: 写节奏模型**

给出 12–20 回合的"预期血量/金币/时代"三线模型表，并说明第 20 回合强制结束的合理性（预期伤害累计 ≥ 40 的回合数）。

- [ ] **Step 4: 校验**

Run: `grep -c '^| ' docs/08-数值与经济.md`
Expected: 至少 `30`

Run: `grep -q '每回合降价' docs/08-数值与经济.md && grep -q '牌池份数' docs/08-数值与经济.md && echo ok`
Expected: `ok`

---

### Task 6: 分册 05 · 卡牌规范与数据（含量最大）

**Files:**
- Create: `docs/05-卡牌规范与数据.md`
- Create: `docs/data/卡牌数据.csv`

**Interfaces:**
- Consumes: 03 的关键词表与文本模板、04 的时代表、08 的星级区间表。
- Produces: 卡牌字段定义、120 张随从全量表、素材→新卡的映射与剔除清单。Task 7（英雄/战术牌）与后续开发直接读取 CSV。

- [ ] **Step 1: 写字段规范**

CSV 表头固定为：`id,name,tribe,subtype,level,era,attack,health,keywords,text,is_scholar,source`。逐字段写含义与取值域；`level` 是星级 1–6，`era` 是时代名（可为空表示全时代），`is_scholar` 为 `true` 时 `attack` 必须为 0。

- [ ] **Step 2: 写 120 行卡牌数据**

分配：史前 24、原始 24、封建 24、现代 30、未来 18（未来首版收敛）。每星级至少 12 张。流派占比：人类 24、机械 22、生物 24、植物 12、病毒 10、学者 28。**素材卡优先**：至少 80 张卡名直接来自素材；重复卡（孔子、米开朗基罗、爱德华·詹纳）合并为一张并在 `source` 列注明；空卡（木偶、木制机械傀儡）不入表，写入文档的"未来素材库"附表。

- [ ] **Step 3: 写映射与校验规则**

文档中列出：素材 140+ → 120 的取舍清单（保留 / 合并 / 剔除 / 改名），以及 5 条数据校验规则（唯一 id、学者攻击为 0、数值落在星级区间、关键词属于 10 词表、每时代张数达标）。

- [ ] **Step 4: 校验**

Run: `python3 -c "
import csv
r=list(csv.DictReader(open('docs/data/卡牌数据.csv',encoding='utf-8')))
print(len(r))
print(sum(1 for x in r if x['is_scholar']=='true' and int(x['attack'])!=0))
"`
Expected: 第一行 `120`；第二行 `0`

Run: `python3 -c "
import csv,collections
r=list(csv.DictReader(open('docs/data/卡牌数据.csv',encoding='utf-8')))
print(len({x['id'] for x in r}), len({x['name'] for x in r}))
print(collections.Counter(x['era'] for x in r))
"`
Expected: 两个集合长度均为 `120`；era 计数含 史前 24、原始 24、封建 24、现代 30、未来 18

---

### Task 7: 分册 06 · 英雄与技能

**Files:**
- Create: `docs/06-英雄与技能.md`
- Create: `docs/data/英雄数据.csv`

**Interfaces:**
- Consumes: 素材 10 位英雄 + 30 个技能、03 的模板、08 的数值区间。
- Produces: 10 位英雄的完整技能（费用、次数、效果、学派标签）、30 张战术牌的分配表。CSV 表头固定：`id,name,discipline,skill_name,skill_cost,skill_type,skill_text,series`。

- [ ] **Step 1: 写 10 位英雄**

覆盖素材全部 10 位：鲁迅、高德纳、阿尔伯特·爱因斯坦、居里夫人、尼古拉·特斯拉、法拉第、列奥纳多·达·芬奇、格雷戈尔·约翰·孟德尔、查尔斯·罗伯特·达尔文、玛丽·居里。素材中重复的"居里夫人/玛丽·居里"合并为一位（居里夫人），并在文档说明。每位给：学科、主动/被动、费用（0–2 金）、每回合可用次数、精确效果、平衡上限（"该技能单回合最多创造 X 点属性差"）。

- [ ] **Step 2: 写 30 张战术牌**

把素材 30 个"技能"拆成英雄技能（10 个）与战术牌（20 个），另补 10 张新战术牌凑满 30。每张给：星级、流派归属、费用、效果、比赛模式下的确定性替换写法。

- [ ] **Step 3: 校验**

Run: `python3 -c "
import csv
r=list(csv.DictReader(open('docs/data/英雄数据.csv',encoding='utf-8')))
print(len(r)); print([x['name'] for x in r])
"`
Expected: `10`，且列表含 `鲁迅`、`高德纳`、`居里夫人`、`爱因斯坦`

Run: `grep -q '玛丽·居里' docs/06-英雄与技能.md && echo ok`
Expected: `ok`（合并说明可被检索到）

---

### Task 8: 分册 07 · 联网与技术架构

**Files:**
- Create: `docs/07-联网与技术架构.md`

**Interfaces:**
- Consumes: 总纲 §8、02 的状态机与意图名。
- Produces: 服务端权威架构、事件协议草案、确定性随机方案、快照与重连、匹配、反作弊边界、规则引擎分层。技术选型明确标为"待定"，但接口约束必须可直接实现。

- [ ] **Step 1: 写架构与分层**

画文本架构图：客户端表现层 / 规则引擎（同构）/ 房间服务 / 匹配服务 / 持久化。写明"规则引擎不依赖网络与 UI"这一硬约束，以及同一引擎在服务端结算、客户端预演的复用方式。

- [ ] **Step 2: 写协议草案**

定义事件信封 `{seq, roomId, playerId, type, payload, ts}`；写出 5 条关键消息：`Intent`、`StateDelta`、`CombatResult`、`Snapshot`、`Reconnect`。给出一次完整回合的消息序列示例（招募→战斗→结算）。

- [ ] **Step 3: 写确定性、反作弊与容错**

必须覆盖：种子生成与存储、服务端校验点、隐藏信息隔离清单（哪些字段绝不下发）、断线重连的补发算法、AFK 托管触发条件、客户端篡改的检测面。

- [ ] **Step 4: 校验**

Run: `grep -c '^### ' docs/07-联网与技术架构.md`
Expected: 至少 `8`

Run: `for m in Intent StateDelta CombatResult Snapshot Reconnect; do grep -q "$m" docs/07-联网与技术架构.md || echo "MISSING $m"; done`
Expected: 无输出

---

### Task 9: 分册 09 · 模式与成就

**Files:**
- Create: `docs/09-模式与成就.md`

**Interfaces:**
- Consumes: 总纲 §6.9、§6.12、素材 6 个成就与 5 个模式。
- Produces: 会战/比赛为启用模式，大决战/坐镇为配置开关，剧情为非目标；6 个成就的触发条件、奖励与是否影响战力。

- [ ] **Step 1: 写模式表**

表头固定：`模式 | 状态 | 棋盘位 | 随机性 | 排名方式 | 首版是否上线`。5 行。比赛模式的"无随机"按总纲 §6.12 的两个开关描述。

- [ ] **Step 2: 写成就**

表头固定：`成就 | 触发条件（可判定） | 奖励 | 是否影响战力`。6 行：人类一小步、图灵时刻、有教无类、机器轰鸣、生物进化、遗传学。触发条件必须写成状态机可判定的事件（例："一名机械随从通过图灵判定并获得人类加成"）。

- [ ] **Step 3: 校验**

Run: `grep -c '^| ' docs/09-模式与成就.md`
Expected: 至少 `16`

Run: `for a in 人类一小步 图灵时刻 有教无类 机器轰鸣 生物进化 遗传学; do grep -q "$a" docs/09-模式与成就.md || echo "MISSING $a"; done`
Expected: 无输出

---

### Task 10: 分册 10 · 版本规划与风险

**Files:**
- Create: `docs/10-版本规划与风险.md`

**Interfaces:**
- Consumes: 总纲 §2 成功标准、§3 非目标、§7 内容规模。
- Produces: M0–M4 里程碑、每版本的卡池增量、风险清单与止损线。

- [ ] **Step 1: 写里程碑**

M0 规则可跑（无 UI，命令行跑完一局 8 人 AI）、M1 单机可玩、M2 联机内测、M3 首版上线、M4 扩展层（场景/陨落/坐镇）。每个里程碑给：交付物、退出标准、预计人周（标注为估算，非承诺）。

- [ ] **Step 2: 写风险表**

表头固定：`风险 | 触发信号 | 影响 | 缓解措施 | 止损线`。至少 8 条，必须覆盖：学者流派过强/过弱、PvP 匹配空房、平衡迭代拖期、规则口径返工、卡牌内容量不足、技术栈未定导致的架构返工、运气成分引发差评、营收不影响战力的承诺被破坏。

- [ ] **Step 3: 校验**

Run: `grep -c '^| ' docs/10-版本规划与风险.md`
Expected: 至少 `12`

Run: `for m in M0 M1 M2 M3 M4; do grep -q "$m" docs/10-版本规划与风险.md || echo "MISSING $m"; done`
Expected: 无输出

---

### Task 11: README 索引与全量一致性复核

**Files:**
- Create: `docs/README.md`
- Modify: `docs/00-设计总纲.md`（回填 §1 表格的"状态"列）

**Interfaces:**
- Consumes: 全部 11 篇文档与 2 份 CSV。
- Produces: 阅读顺序、文档状态总表、术语表入口。

- [ ] **Step 1: 写 README**

包含：项目一句话、阅读顺序（00 → 01 → 02 → 03 → 04 → 08 → 05 → 06 → 07 → 09 → 10）、每篇一句话摘要、数据表位置、术语表链接（指向 03）。

- [ ] **Step 2: 回填总纲状态列**

把 `00-设计总纲.md` §1 表格中所有"待写"改为"已写（v0.1）"。

- [ ] **Step 3: 全量一致性复核**

Run: `ls docs/*.md docs/data/*.csv | wc -l`
Expected: `14`（00–10 共 11 篇 + README + 2 份 CSV）

Run: `python3 -c "
import csv,re
cards=list(csv.DictReader(open('docs/data/卡牌数据.csv',encoding='utf-8')))
heroes=list(csv.DictReader(open('docs/data/英雄数据.csv',encoding='utf-8')))
print(len(cards),len(heroes))
bad=[c['name'] for c in cards if c['is_scholar']=='true' and int(c['attack'])!=0]
print('scholar_bad',len(bad))
"`
Expected: `120 10` 与 `scholar_bad 0`

Run: `grep -rn '待写' docs/ | wc -l`
Expected: `0`

- [ ] **Step 4: 交叉引用检查**

Run: `for f in docs/0*.md docs/1*.md; do head -5 "$f" | grep -q '依赖' || echo "NO-DEPS $f"; done`
Expected: 无输出

---

## Self-Review

- **Spec 覆盖**：总纲 §1 的 11 篇结构 → Task 1–11 一一对应；§6.1–§6.12 的锁定规则分布到 Task 2/3/4/5/9；§7 内容规模 → Task 6/7；§8 技术立场 → Task 8；§10 待确认问题 → Task 10 风险表与 01/09 待决问题。
- **步骤可判定性**：每个 Task 的校验命令都有确定期望值，不依赖主观判断。Task 6 的 120 行是唯一需要大量写作的任务，已单独拆出并给出机器校验。
- **类型/命名一致**：状态名、意图名、关键词名、CSV 表头在首次出现处定义，后续任务直接引用，未出现同义变体。
- **遗漏已补**：写作中发现"战术牌归属"只能在 06 与 05 之间二选一，已固定在 06。
- **比例**：计划本身约 300 行，产出约 11 篇文档，比例合理；未把文档正文写进计划。
