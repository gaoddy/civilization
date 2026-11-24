# 内容素材库存

> 状态：参考材料（不进入卡池）
> 性质：内容设计参考材料，不进入已实现卡池，不代表已排期。本文件是卡牌素材设计的唯一库存文档，原先分散的卡牌设计、诺奖名册与设计角度已全部并入本文件。
> 口径：`docs/current/product/content/`（432 张卡 + 32 张英雄）、规则引擎代码白名单（2026-11-25）；诺奖原始名单来自同目录 [nobel-laureates.md](nobel-laureates.md) 与 [nobel-laureates-zh.md](nobel-laureates-zh.md)，抓取于 2026-11-25。

## 0. 本文件怎么读

| 部分 | 内容 | 规模 |
| --- | --- | --- |
| 第一部分 · 卡牌设计 | 内容素材库素材的精选卡牌设计：首批 26、扩展 94、诺奖效果牌 32；含引擎支持矩阵、9 项新机制、39 个模板与 69 个技能 | 152 张卡 + 7 个衍生物 |
| 第二部分 · 诺奖全量名册 | 1901–2026 全部诺奖得主与机构的一人一卡覆盖：科学三奖做随从、文学/和平/经济做效果牌；含 36 个奖项效果池与 15 个名册模板 | 1028 位得主/机构（1036 条记录去重） |
| 第三部分 · 诺奖设计角度 | 现有卡池覆盖分析、机制原型、人物与概念选题、风险边界与首批建议 | 设计分析，不单独出卡 |

三部分的关系：第一部分是**精选机制**，给高辨识度概念配专属技能；第二部分是**全量覆盖**，保证每个得主至少一张卡或一个技能；第三部分是**选题依据与取舍记录**。三者不互相替代，实施时按批次取用。

本文件不描述排期，也不替代版本规划；所有数值为草案，落地时以 `python3 scripts/validate_docs.py` 与 `make data-check` 为准。按设计阶段约定，本文件不对第二部分全量名册做强度、卡池占比与技能密度校验。

## 第一部分 · 卡牌设计（精选 152 张）

### 1. 结论

1. **三批共 152 张卡 + 7 个非池衍生物**：首批 26 张（20 随从 + 4 效果 + 2 英雄），扩展 94 张（66 随从 + 22 效果 + 6 英雄），诺奖效果牌 32 张（全部为效果牌）。首批对齐[版本路线](../../../../version-roadmap.md) §3 的「+20 随从、+2 英雄」增量；扩展批次是素材池的完整开掘，供后续版本按需取用，不代表一次性上线；诺奖批次只补效果牌，把诺奖素材里可结算的科学概念集中做成一套横向工具牌。
2. **不新增玩法**：全部卡牌落在现有 13 个玩法内。「灵长类」「量子」「医学」用多 `play:` 归属、`taxon` 与 `subtype` 表达，不引入第 14 个玩法。扩展批次按现有卡池厚度反向配平：66 张随从里哺乳纲 0 张，鸟纲、昆虫纲、鱼纲、真菌界、细菌域、病毒各 6–7 张。
3. **不新增常驻关键词**：新机制只用「参数、触发器、原语、技能模板」扩展；[关键词判定表](../../rules/keywords-and-mechanics.md) §1 的 16 个关键词不变。诺奖批次同样只复用这 16 个词，动态赋予的护盾与潜行走 `GrantKeyword`。
4. **可直接落地的 77 张**：复用现有技能或模板即可写成实体文档（支持状态 A）；其余只需新增技能模板或实例（B，45 张）。
5. **需要引擎改动的 30 张**：集中在 9 项新机制——概率判定、出售触发与金币、递增成长、清除传染、献祭、属性互换、沉默、冻结、购买触发。诺奖批次贡献 7 张 C 类，全部落在上述机制内，不引入第 10 项。每项都按「可复用模板」设计，不做单卡子系统。
6. **推荐顺序**：引擎 9 项 → 39 个新模板 → 69 个新技能 → 152 张卡 → 数据生成与模拟。任何一步都不先于上一步合入；扩展与诺奖批次都可以按玩法或学科切片分批落地。

### 2. 设计边界

#### 2.1 必须满足的数据规则

以下规则来自[卡牌规范与数据](../card-system.md) §1、§2 与[数值与经济](../../systems/economy-and-numbers.md) §5，本文所有卡都按此自检：

| 约束 | 口径 |
| --- | --- |
| 星级与身材 | 1 星 1–3 / 2–5，2 星 2–4 / 3–6，3 星 3–6 / 4–8，4 星 4–8 / 6–10，5 星 6–10 / 8–13，6 星 8–12 / 10–16，7 星 10–15 / 14–20 |
| 攻击力 0 随从 | 生命值取同星级生命上限 ×1.3，允许 ±1；豁免身材分布约束 |
| 超模上限 | `攻击力 + 生命值 + 关键词折算 ≤ 同星级白板总和 + 4` |
| 技能数量 | 1–2 星最多 1 个；3–4 星最多 2 个；5–7 星最多 3 个，其中登记关键词不超过 2 个 |
| 卡池分布 | 同「玩法 + 星级 + 攻击力 + 生命值」最多 2 张（攻 0 豁免）；同「玩法 + 星级 + 效果技能集」最多 1 张；只带关键词的卡按「玩法 + 星级 + 身材 + 完整技能列表」最多 1 张；攻 0 卡按「玩法 + 星级 + 完整技能列表」最多 1 张 |
| 专属关键词 | `分解` 仅真菌界/细菌域；`装配` 仅机械；`播种` 仅植物界；`进化` 仅生物卡；`剧毒`/`减伤`/`潜行` 受玩法与最小星级约束 |
| 效果牌 | `cost` 0–4；4 费只允许 6–7 星；同一星级效果牌不超过该星级全部卡牌的 30%；必须有非空 `skills` |
| 英雄卡 | 只保留 `id`、`name`、`categories`、`introduction`、`skills`；必须与同 id 同名随从卡同池并存；技能只能是主动或被动，不得登记战场关键词 |
| 同名 | 进池随从、进池效果牌、英雄池三者各自唯一；跨池允许同 id 同名 |

#### 2.2 确定性

- 所有随机必须走引擎的种子随机流；本文的「概率」机制按 `Stream::Effects` 每次判定推进一次，同一种子与同一操作序列必须重放出同一结果。
- 不使用「每局随机攻击力」这类在建卡阶段就写入随机数值的写法；随机只发生在结算时。
- 新增机制不得读取时钟、文件或网络；`Ramp` 只读对局内的回合计数。

#### 2.3 引擎判定口径

本文的「已支持/需新增」按 2026-11-25 的代码白名单判定，不按文档口径：

- 触发白名单在 `server/crates/rule-engine/src/effects/parse.rs` 的 `TRIGGERS`，共 18 个；
- 原语白名单在同一文件的 `OPS`，共 23 个；[关键词与机制](../../rules/keywords-and-mechanics.md) §9.3 只列了其中 15 个，`Mimic`、`Devour`、`Detonate`、`Convert`、`Revive`、`Recall`、`Salvage`、`Latent` 未在该表出现，实际已可解析。
- 招募路径与战斗路径的原语可达性不同：`Detonate`、`Convert`、`Revive`、`Recall`、`Salvage` 在招募路径直接返回空结果，只能挂在战斗触发（如 `OnDeath`、`OnCombatStart`）上；`Decompose` 同理只发生在战斗内。
- 本文没有使用上述招募不可达原语，避免出现「卡面写了、结算无声」的卡。

### 3. 素材选取

#### 3.1 纳入首批的素材

| 素材 | 素材库给出的再评估方向 | 本文决定 | 对应卡 |
| --- | --- | --- | --- |
| 珍·古德 | 与黑猩猩体系联动 | 做成灵长目光环，串联灵长类子流派 | 珍·古德（随从 + 英雄） |
| 黑猩猩「白胡子」大卫 | 战吼钓白蚁不可结算，改为召唤白蚁 | 战吼召唤两个白蚁衍生物，跨哺乳纲/昆虫纲 | 黑猩猩「白胡子」大卫 |
| 猩猩阿兹 | 依赖数学系加成，作为跨流派桥梁 | 哺乳纲 + 机械双归属，只要控制机械随从就给自己加攻 | 猩猩阿兹 |
| 猩猩因达 | 同上 | 哺乳纲 + 机械双归属，战吼给哺乳纲成长 | 猩猩因达 |
| 猴子泰特拉一世 | 召唤链过长，简化为召唤一个 2/2 | 亡语召唤一个 2/2 猴子衍生物 | 猴子泰特拉一世 |
| 布里切特三世 | 猴子部队体系核心 | 机械随从入场时自身加攻，并带装配 | 布里切特三世 |
| 黑猩猩奥利佛 | 调酒增益不可结算，改为成长 | 战吼给灵长目两次成长，带嘲讽 | 黑猩猩奥利佛 |
| 小猴子海利恩 | 回手效果与战棋骨架冲突，改为出售返还 2 金币 | 新增出售触发与金币原语，做成「免费租用」 | 小猴子海利恩 |
| 薛定谔的猫 | 随机攻击力与确定性冲突，改为战斗开始 50% 加攻并用种子固定 | 概率参数化，走种子随机流 | 薛定谔的猫 |
| 薛定谔气体模型 | 机制为空，作为量子流派种子卡 | 概率群体成长的效果牌 | 薛定谔气体模型 |
| 费波那契 | 斐波那契成长曲线难平衡 | 线性化递增成长，设上限 | 费波那契、斐波那契数列 |
| 手工耿 | 召唤物已有同名卡，作为彩蛋英雄卡 | 做成英雄卡 + 同名随从，主动技能给机械加身材 | 手工耿（随从 + 英雄） |
| 莎士比亚 / 荷马 / 贝多芬 / 梵高 / 卢梭 | 机制为空，作为文学家/艺术家体系锚点 | 本轮只取梵高，其余已有卡或留待后续 | 梵高 |
| 凯恩斯 / 卡耐基 / 福特 | 经济效果过强或无数值，需要专门经济流派 | 只取凯恩斯，先做单卡金币来源并设上限 | 凯恩斯 |
| 莱特兄弟 / 贝尔 / 诺贝尔 | 与其他机械卡重叠，合并为机械光环 | 贝尔做群体成长，诺贝尔做机械光环 | 贝尔、诺贝尔 |
| 伦琴 / 普朗克 / 海森堡 / 麦克斯韦 / 拉瓦锡 / 开普勒 / 哥白尼 / 欧拉 / 费米 | 数量太多，按学科分批上线 | 本轮只取普朗克、麦克斯韦；海森堡、拉瓦锡、费米已有卡 | 普朗克、麦克斯韦 |
| 弗洛伊德 / 李斯特 / 莫顿 / 列文虎克 / 弗莱明 | 机制为空，医学/心理学子标签 | 本轮取李斯特、列文虎克；弗莱明已有卡 | 约瑟夫·李斯特、列文虎克 |
| 爱德华·詹纳 | 机制为空，疫苗体系 | 疫苗体系第一张：清除传染并短时免疫 | 爱德华·詹纳、疫苗 |
| 诺奖概念素材 | 机制原型：PCR、CRISPR、转座子、朊病毒等 | 首批只做「创造性破坏」一张经济效果牌；诺奖批次把可结算的概念集中做成 §7.9 的 32 张效果牌 | 创造性破坏、§7.9 |

扩展批次的 66 张随从按「物种 + 科学概念」选取，逐卡出处见 §7.5；它们不依赖素材库名单，目的是把 13 个玩法里偏薄的卡池补到同一量级，同时避开已有 332 张随从的名字。22 张效果牌覆盖 1–7 星与 0–4 费，逐卡见 §7.6。§7.9 的 32 张诺奖效果牌只取本库存「诺奖设计角度」部分里已有机制原型或可用现有原语表达的概念，不新增子系统；学科集中在生理学或医学、化学、物理学三类。

#### 3.2 本轮不纳入

| 素材 | 原因 | 后续建议 |
| --- | --- | --- |
| 黑猩猩尼姆 | 经济效果与凯恩斯重叠，首批名额有限 | 与经济批次一起做 |
| 科学怪物 | 「只活一回合」需要生命周期机制 | 与 `Expire`/短命衍生物一起做，见 §5.6 |
| 木偶 | 1 星填充卡，卡池 1 星已足够 | 后续低星补量时做 |
| 卡耐基 / 福特 | 经济流派需要多卡联动 | 等经济批次成型 |
| 弗洛伊德 / 莫顿 | 心理学与麻醉缺少可结算机制 | 与「无法攻击」「镇静」类机制一起做 |
| 开普勒 / 欧拉 / 哥白尼 | 学科人物数量多，避免同星同玩法撞车 | 按学科分批 |

### 4. 引擎支持矩阵

#### 4.1 触发时机

| 触发 | 阶段 | 本批是否使用 | 说明 |
| --- | --- | --- | --- |
| `OnPlay` | 招募 · 入场 | 是 | 随从战吼 |
| `OnPlayEffect` | 招募 · 打出效果牌 | 是 | 效果牌入口 |
| `OnActivate` | 招募 · 主动技能 | 是 | 英雄主动技能 |
| `OnTurnStart` | 招募 · 回合开始 | 是 | 回合递增成长、金币 |
| `OnCombatStart` | 战斗 · 快照后 | 是 | 概率判定 |
| `OnDeath` | 战斗 · 死亡结算 | 是 | 亡语召唤 |
| `OnAllySummon` | 招募/战斗 | 是 | 入场计数 |
| `OnAllyGrowth` | 招募/战斗 | 是 | 成长计数 |
| `OnSell` | 招募 · 出售 | 是（新增） | 出售返还金币 |
| `OnBuy` | 招募 · 购买 | 是（新增） | 购买返金币 |
| `Aura` | 持续求值 | 是 | 灵长类光环、机械光环 |
| `ConditionalAura` | 持续求值 | 是 | 按是否控制机械加攻 |
| `OnDealDamage` / `OnKill` / `OnAllyKill` / `OnFriendlyDeath` / `OnEnemyDeath` / `OnAllyEvolve` / `OnUse` / `OnFirstRefresh` | — | 否 | 本批未使用，机制已存在 |

#### 4.2 动作原语

| 原语 | 招募路径 | 战斗路径 | 本批使用 | 说明 |
| --- | --- | --- | --- | --- |
| `Growth` | 是 | 是 | 是 | 成长 |
| `ModifyStats` | 是 | 是 | 是 | 加攻/加血、光环 |
| `GrantKeyword` | 是 | 是 | 是（新增） | 诺奖批次动态赋予护盾与潜行 |
| `CannotAttack` | 是 | 是 | 是（新增） | 诺奖批次使敌方随从本回合无法攻击 |
| `DealDamage` | 是 | 是 | 是（复用既有技能） | 直伤 |
| `Summon` | 是 | 是 | 是 | 召唤衍生物 |
| `Discover` / `Copy` | 是 | 否 | `Discover` 是 | 发现与复制 |
| `GainMinion` / `GrantMinion` / `RefreshShop` / `AddShopSlot` | 是 | 否 | 否 | 商店与手牌类 |
| `Decompose` | 否 | 是 | 否 | 战斗专属 |
| `Mimic` / `Devour` | 是 | 是 | 是（复用既有技能） | 已有玩法专属 |
| `Revive` | 否（返回空） | 是 | 是（仅战斗触发） | 复苏 |
| `Detonate` / `Convert` / `Recall` / `Salvage` | 否（返回空） | 是 | 否 | 招募不可达，本文回避 |
| `Latent` / `SevenVoyages` | 是 | 否 | 否 | 已有玩法专属 |
| `GainGold` | 是（新增） | 否 | 是（新增） | 金币 |
| `Ramp` | 是（新增） | 是（新增） | 是（新增） | 递增成长 |
| `Cleanse` | 是（新增） | 是（新增） | 是（新增） | 清除传染 |
| `Sacrifice` | 是（新增） | 否 | 是（新增） | 献祭友方 |
| `SwapStats` | 是（新增） | 是（新增） | 是（新增） | 攻击力与生命值互换 |
| `Silence` | 否（新增，仅战斗） | 是（新增） | 是（新增） | 本场战斗失去技能与关键词 |
| `Freeze` | 否（新增，仅战斗） | 是（新增） | 是（新增） | 本场战斗不能攻击 |

#### 4.3 本批卡牌的支持状态

| 状态 | 张数 | 构成 |
| --- | --- | --- |
| A. 复用现有技能或模板，仅补数据 | 77 | 随从 49（首批 9 + 扩展 40）、效果 25、英雄 3 |
| B. 新增技能模板或实例，引擎不改 | 45 | 随从 27（首批 5 + 扩展 22）、效果 13、英雄 5 |
| C. 需要新增参数、触发器或原语 | 30 | 随从 10（首批 6 + 扩展 4）、效果 20、英雄 0 |

逐卡支持状态：首批见 §7.1–§7.4 每卡的「支持状态」行；扩展批次见 §7.5–§7.8 的「支持」列；诺奖批次见 §7.9 的「支持」列。C 类分布：首批为海利恩、薛定谔的猫、费波那契、詹纳、李斯特、凯恩斯与 4 张效果牌；扩展为变色龙、北极燕鸥、竹节虫、算盘与 9 张效果牌；诺奖为基因编辑、转座子、细胞凋亡、免疫检查点、激光冷却、引力波天文学、中微子振荡 7 张效果牌。

#### 4.4 如果引擎仍不支持：三条路线

| 路线 | 做法 | 代价 | 适用 |
| --- | --- | --- | --- |
| A. 扩模板（推荐） | 新增参数、触发器、原语与技能模板，玩法与关键词不变 | 引擎与校验改动集中，可复用 | 本批全部机制 |
| B. 加关键词 | 把「短命」「免疫」等做成常驻关键词 | 需同步关键词判定表、时代解锁、卡面名词、折算与校验；关键词一旦登记就跨卡生效 | 只有当机制会成为流派身份时 |
| C. 加玩法/模式 | 新增第 14 个玩法，或做独立模式 | 玩法抽取、卡池分布、英雄候选、图鉴、协议、平衡全部改；成本最高 | 不建议在 v0.3 前做 |

本文全部走 A。需要表达「灵长类」「量子」时，用 `play:` 多归属、`taxon`、`subtype` 组合，不新开玩法。例如「量子」由 `subtype: quantum` 与概率机制表达；「灵长类」由 `taxon: primates / cercopithecidae` 与哺乳纲归属表达。

### 5. 新机制方案

#### 5.1 概率判定：`chance`

**来源**：薛定谔的猫（战斗开始 50% 加攻）、薛定谔气体模型（概率群体成长）。

**语义**：一次性触发结算时，按百分比决定该段效果是否执行；判定消耗一次种子随机流，结果写入事件日志，回放一致。

**写入位置**：不进入 `Params` 的单个变体，而是作为效果段的通用可选键，与 `when` 并列：

```text
OnCombatStart|ModifyStats|self|attack=3,duration=permanent,chance=50
OnCombatStart|Growth|friendly:all|times=1,chance=50
```

**模板参数**：`arg.chance: int 1..99`；文案占位符用新增修饰符 `{chance.percent}`，渲染为「50%」，金色不翻倍。

**引擎改动**：

1. `Effect` 增加 `chance: Option<u8>`，解析第 4 段时统一取出并校验 1–99；
2. 结算入口在解析目标之前判定一次；失败则不执行该段，但仍写入「未触发」事件；
3. 随机流复用 `Stream::Effects`，每次判定推进一次，保证与随机选目标同源可重放；
4. 金色只翻倍产出数值，不改变概率。

**校验**：`chance` 只允许挂在非光环触发上；`Aura` / `ConditionalAura` 直接报错。概率为 0、100 或非整数报错。

**平衡**：折算按期望值计算；50% × +3 攻击记 1.5 点，向上取 2 点。概率卡不叠加，同一技能内最多一个 `chance`。

**未决**：是否允许 `chance` 与 `times` 同时存在（本文只用「一次判定、整段执行」的口径，不做逐目标判定）。

#### 5.2 出售触发与金币：`OnSell` + `GainGold`

**来源**：小猴子海利恩（出售返还 2 金币）、凯恩斯（回合开始获得金币）、创造性破坏（献祭换金币）。

**语义**：

- `OnSell`：在 `SellMinion` 移除随从之前触发，事件载荷为被出售的随从实例；从棋盘与手牌出售都算；
- `GainGold`：给当前玩家加金币，只允许招募阶段的触发器使用；金币仍按「不跨回合累积」处理。

```text
OnSell|GainGold|self|amount=2
OnTurnStart|GainGold|self|amount=1
OnPlayEffect|GainGold|self|amount=3
```

**引擎改动**：

1. `Trigger` 增加 `OnSell`，在 `SellMinion` 的移除逻辑前派发该随从的技能；
2. `Op` 增加 `GainGold`，`Params::GainGold { amount }`，`amount` 1–3；
3. 状态增加 `GainGold` 每回合金币来源计数；`OnSell` 来源合计每回合最多 3，`GainGold` 合计每回合最多 5，超出部分不再加；七下西洋等既有金币来源不受此限；
4. 事件日志新增金币变动事件，客户端沿用金币显示。

**校验**：`OnSell` 只能出现在随从卡引用的技能上，不能出现在英雄卡；`OnSell` 技能只允许 `GainGold`，避免随从离场后仍结算目标类效果。`GainGold` 只允许 `OnSell`、`OnTurnStart`、`OnActivate`、`OnPlayEffect`。

**平衡**：出售基础返还 1（金色 3）；`amount=2` 的卡卖出等于原价，做成「免费租用」而非净赚。`OnSell` 来源每回合 3 金币上限，避免多张经济卡叠加成无限购买。凯恩斯每回合 1 金币，按 5 星 0/17 折算 2 点；若模拟过强，先改成 `OnFirstRefresh` 或提到 6 星。

**未决**：金色 `OnSell` 是否翻倍返还。本文按「不翻倍，只按基础返还 3」处理，避免金色经济卡失控。

#### 5.3 递增成长：`Ramp`

**来源**：费波那契（按回合数列成长）。

**语义**：按对局回合数计算成长次数，线性递增并设上限；不读时钟，不做指数曲线。

```text
OnTurnStart|Ramp|self|every=2,max=4
OnPlayEffect|Ramp|friendly:single|every=2,max=4
```

**计算**：`times = min(ceil(turn / every), max)`。回合 1 为 1 次，回合 3 为 2 次，回合 7 起封顶 4 次。

**引擎改动**：`Op` 增加 `Ramp`，`Params::Ramp { every, max }`；`every` 1–4，`max` 1–4；结算时取当前对局回合数，调用与 `Growth` 相同的成长路径。

**校验**：只允许 `OnTurnStart`、`OnCombatStart`、`OnPlayEffect`；`max` 超过 4 报错（对应[平衡测算](../balance-analysis.md) §7 的「单随从累计成长 ≤12 次」观察项）。

**平衡**：费波那契原文是指数数列，直接实现会失控；本文把它线性化为「每两回合 +1、封顶 4」，既保留「越拖越强」的身份，也不突破成长上限。折算暂按 4 点；若模拟显示超模，先降 `max` 到 3。

#### 5.4 清除传染：`Cleanse`

**来源**：爱德华·詹纳与疫苗体系、李斯特的消毒、免疫检查点。

**语义**：清除目标身上全部传染层；按 `duration` 决定是否同时短时免疫。

```text
OnPlay|Cleanse|friendly:single|duration=combat
OnPlay|Cleanse|enemy:all|duration=instant
OnPlayEffect|Cleanse|friendly:all|duration=combat
```

- `instant`：只清除当前层数；
- `combat`：清除层数，并在本场战斗内不再获得传染；
- `permanent`：清除层数，并永久免疫（本文不使用，留给后续）。

**引擎改动**：`Op` 增加 `Cleanse`，`Params::Cleanse { duration }`；在赋予传染的入口检查免疫标记；免疫标记随战斗结束清除（`permanent` 除外）。

**校验**：`duration` 必填且属于 `instant` / `combat` / `permanent`；选择器只允许友方或敌方，不允许 `both`（避免一张卡同时清两边）。

**平衡**：清除类按小效果折算 2 点；群体清除按 3 点。疫苗是 3 星 2 费效果牌，按 3 点上限校验。

**未决**：清除是否返还「已造成的每回合掉血」——不返还，只清层数与后续获取。

#### 5.5 献祭：`Sacrifice`

**来源**：创造性破坏、细胞凋亡。

**语义**：把一个友方随从移出战场，不经过伤害、不触发亡语、复生与死亡事件，然后由后续效果发金币。

```text
OnPlayEffect|Sacrifice|friendly:single
OnPlayEffect|GainGold|self|amount=3
```

**引擎改动**：`Op` 增加 `Sacrifice`，选择器固定 `friendly:single`；移除实例时跳过死亡队列与亡语，归还该实例实际扣池份数（与出售一致），但不返还基础 1 金币。

**校验**：只允许 `OnPlayEffect` 与 `OnActivate`；选择器固定 `friendly:single`，不能选 `self`；英雄主动技能若使用它，目标必须是棋盘上的友方随从。

**平衡**：献祭不触发亡语是刻意的，避免「分裂衍生物 → 献祭 → 再召唤 → 再献祭」的金币循环。若后续要做「献祭触发亡语」的流派，另开原语，不复用本项。

**未决**：献祭是否计入「友方随从死亡」类计数——本文定义为不计入。

#### 5.6 属性互换：`SwapStats`

**来源**：变色龙（体色与形态变化）、交换律（数学）、对称性（物理）、转座子。

**语义**：把一个随从当前的攻击力与生命值互换一次；交换的是结算时的当前值（含成长与光环），交换后光环继续叠加；生命值降到 0 以下不触发交换。

**引擎改动**：`Op` 增加 `SwapStats`，无参数；选择器 `friendly:single`；`OnPlay` / `OnPlayEffect` / `OnActivate` 只能作用于友方，`OnCombatStart` 可作用于 `enemy:single`。

**校验**：`SwapStats` 禁止与 `duration` 以外的参数同用；不得出现在光环触发上。

**平衡**：一次互换折算 2 点；对高血低攻卡是增益，对高攻低血卡是自伤，不额外补偿。对称性额外 +1/+1 折算 3 点。

**确定性**：无随机。

**未决**：互换后是否重算光环——本文按「交换当前值、光环继续叠加」处理。

#### 5.7 沉默：`Silence`

**来源**：沉默的螺旋（社会学）、大过滤器（天文）、基因编辑、RNA 干扰。

**语义**：目标随从在本场战斗内失去全部技能与关键词（含光环、亡语、复生、护盾、嘲讽），当前攻击力、生命值与已积累成长不变；战斗结束恢复。

**引擎改动**：`Op` 增加 `Silence`，`duration=combat` 固定；战斗快照增加 `silenced` 标记，结算入口跳过其技能派发与关键词判定。

**校验**：只允许 `OnCombatStart`；选择器为 `enemy:single` 或 `enemy:all`。招募阶段不可指定敌方，因此效果牌也不在打出时结算，而是延迟到战斗开始。

**平衡**：沉默 1 个折算 2 点，全体折算 4 点（对应 4 费 7 星）。

**确定性**：无随机。

**未决**：是否清除已获得的临时关键词——本文按清除处理。

#### 5.8 冻结：`Freeze`

**来源**：北极燕鸥（极地）、冰期（地质）、激光冷却（物理）、引力波（天文）。

**语义**：目标随从下一场战斗不能攻击，仍可被攻击；战斗结束解除，不叠加。

**引擎改动**：`Op` 增加 `Freeze`，状态标记 `frozen`；攻击阶段跳过带标记的随从。

**校验**：只允许 `OnCombatStart`；选择器 `enemy:leftmost` / `enemy:random` / `enemy:single` / `enemy:all`，其中 `enemy:all` 只允许 6 星 4 费以上效果牌。

**平衡**：冻结 1 个折算 2–3 点，按目标位置价值取 3 点；群体冻结按 6 点计，只放在最高费用档。

**确定性**：无随机；选随机目标时走种子流。

**未决**：冻结是否阻止反击——本文按「只跳过主动攻击」处理。

#### 5.9 购买触发：`OnBuy`

**来源**：算盘（计算工具的经济性）。

**语义**：玩家从商店买下一张卡后、进入手牌前，结算该卡自身的 `OnBuy` 技能；三连奖励与发现不触发。

**引擎改动**：`Trigger` 增加 `OnBuy`；在 `BuyMinion` 扣费成功后派发；只允许 `GainGold`、`ModifyStats`、`Growth` 且选择器为 `self` / `friendly:all`。

**平衡**：购买返 1 金币折算 2 点；金币仍受 §5.2 的每回合来源上限约束。

**确定性**：无随机。

**未决**：金色购买是否翻倍——本文按不翻倍处理。

#### 5.10 暂不实施

| 机制 | 来源 | 不做的原因 | 后续建议 |
| --- | --- | --- | --- |
| `Expire` 生命周期 | 科学怪物「只活一回合」 | 需要新的实例字段、清理时点与视图字段，首批与扩展批次没有卡依赖它 | 复用 `OnAllySummon` 做成 `OnAllySummon|Expire|target|duration=turn` 的非池卡专用模板，只用 `in_shop_pool: false` |
| `Steal` 策反 | 头足纲的拟态与寄生 | 改变归属会牵动牌池份数、三连与布阵校验，收益与风险都过大 | 先做「复制」类卡，不做真实归属转移 |
| `Transform` 形态转化 | 昆虫变态、蝌蚪变蛙 | 需要实例替换与牌池归还规则，与召唤链耦合 | 用「召唤更高星衍生物」近似 |

### 6. 新技能与模板

#### 6.1 新增技能实例

所有技能 id 为草案，落地前需在技能命名空间查重。`keyword` 只在技能使携带者自身登记关键词时填写；赋予他人的效果留空。

| 技能 id | 名称 | 模板调用 | 登记关键词 | 渲染文案 |
| --- | --- | --- | --- | --- |
| `aura-primates-plus-1-plus-1` | 灵长类观察 | `aura-all-taxon-both(taxon=primates,attack=1,health=1)` | — | 你的灵长目随从获得 +1/+1 |
| `battlecry-summon-2-termites` | 钓白蚁 | `on-play-battlecry-summon(token=termite-token,count=2)` | `battlecry` | 战吼：召唤两个 2/2 的白蚁 |
| `conditional-machine-self-attack-2` | 借力 | `conditional-playstyle-self-attack(playstyle=machine,attack=2,count=1)` | — | 若你控制至少 1 个机械随从，获得 +2 攻击力 |
| `deathrattle-summon-monkey` | 猴群 | `on-death-summon(token=monkey-token,count=1)` | `deathrattle` | 亡语：召唤一个 2/2 的猴子 |
| `ally-summon-self-attack-1` | 部队核心 | `ally-summon-self-attack(attack=1)` | — | 每当一个友方随从进入战场，获得 +1 攻击力 |
| `battlecry-friendly-primate-growth-twice` | 传承 | `on-play-battlecry-growth-single-taxon(taxon=primates,times=2)` | `battlecry` | 战吼：使一个友方灵长目随从获得两次成长 |
| `sell-refund-2` | 回售 | `on-sell-gain-gold(amount=2)` | — | 出售时：返还 2 金币 |
| `chance-cat-attack` | 叠加态 | `combatstart-chance-self-attack(chance=50,attack=3)` | — | 战斗开始时，有 50% 概率获得 +3 攻击力 |
| `fibonacci-ramp` | 数列成长 | `turn-start-ramp-growth(every=2,max=4)` | — | 回合开始时，获得等同于本局回合数除以 2 的成长（向上取整，至多 4 次） |
| `battlecry-enemy-all-minus-1-attack` | 量子化 | `on-play-battlecry-stats-enemy-all-attack(attack=-1)` | `battlecry` | 战吼：使敌方所有随从获得 -1 攻击力 |
| `ally-growth-self-both-1` | 感应 | `ally-growth-self-both(attack=1,health=1)` | — | 每当一个友方随从获得成长，获得 +1/+1 |
| `battlecry-cleanse-friendly-single-immunity` | 接种 | `on-play-battlecry-cleanse-single` | `battlecry` | 战吼：清除一个友方随从的全部传染；本场战斗内不再获得传染 |
| `battlecry-cleanse-all-enemy` | 消毒 | `on-play-battlecry-cleanse-all-enemy` | `battlecry` | 战吼：清除敌方所有随从的传染 |
| `battlecry-friendly-growth-thrice` | 向日葵 | `on-play-battlecry-growth-single(times=3)` | `battlecry` | 战吼：使一个友方随从获得三次成长 |
| `turn-start-gain-1-gold` | 有效需求 | `turn-start-gain-gold(amount=1)` | — | 回合开始时，获得 1 金币 |
| `activate-friendly-mech-plus-2-plus-2` | 无用发明 | `activate-stats-single-ps-both(playstyle=machine,attack=2,health=2)` | — | 使一个友方机械随从获得 +2/+2 |
| `behavioral-observation` | 观察 | `activate-discover-playstyle` | — | 发现一张本局启用的玩法随从 |
| `gas-model-chance-growth` | 气体模型 | `combatstart-chance-growth-all(chance=50,times=1)` | — | 战斗开始时，有 50% 概率使你的所有随从获得一次成长 |
| `vaccine-cleanse` | 疫苗 | `on-play-effect-cleanse-all-friendly` | — | 清除你的所有随从的传染；本场战斗内不再获得传染 |
| `sequence-ramp-growth-2-4` | 数列 | `on-play-effect-ramp-growth-single(every=2,max=4)` | — | 使一个友方随从获得等同于本局回合数除以 2 的成长（向上取整，至多 4 次） |
| `creative-destruction` | 创造性破坏 | `on-play-effect-sacrifice；on-play-effect-gain-gold(amount=3)` | — | 献祭一个友方随从，获得 3 金币 |

`activate-friendly-mech-plus-2-plus-2` 与 `behavioral-observation` 是主动技能，必须在实例上写 `cost` 与 `uses_per_turn`：前者 `cost: 1`、`uses_per_turn: 1`；后者 `cost: 2`、`uses_per_turn: 1`。其余为触发式技能。

`battlecry-summon-2-termites` 与 `deathrattle-summon-monkey` 使用 `text` 覆盖，原因是衍生物展示名已含数值（`白蚁（2/2）`、`猴子（2/2）`），模板会把数值重复渲染一次。

#### 6.2 新增模板

| 模板 id | 触发 | 原语 | 选择器 | 参数 | 文案 |
| --- | --- | --- | --- | --- | --- |
| `conditional-playstyle-self-attack` | `ConditionalAura` | `ModifyStats` | `self` | `attack`, `duration=permanent`, `condition=count:friendly:playstyle={playstyle}:>={count}` | 若你控制至少 {count} 个{playstyle}随从，获得 +{attack} 攻击力 |
| `ally-summon-self-attack` | `OnAllySummon` | `ModifyStats` | `self` | `attack`, `duration=permanent` | 每当一个友方随从进入战场，获得 +{attack} 攻击力 |
| `on-play-battlecry-growth-single-taxon` | `OnPlay` | `Growth` | `friendly:single:taxon={taxon}` | `times` | 战吼：使一个友方{taxon}随从获得{times.cn}次成长 |
| `on-sell-gain-gold` | `OnSell` | `GainGold` | `self` | `amount` | 出售时：返还 {amount} 金币 |
| `combatstart-chance-self-attack` | `OnCombatStart` | `ModifyStats` | `self` | `attack`, `duration=permanent`, `chance` | 战斗开始时，有 {chance.percent} 概率获得 +{attack} 攻击力 |
| `turn-start-ramp-growth` | `OnTurnStart` | `Ramp` | `self` | `every`, `max` | 回合开始时，获得等同于本局回合数除以 {every} 的成长（向上取整，至多 {max} 次） |
| `on-play-battlecry-stats-enemy-all-attack` | `OnPlay` | `ModifyStats` | `enemy:all` | `attack`, `duration=permanent` | 战吼：使敌方所有随从获得 {attack} 攻击力 |
| `ally-growth-self-both` | `OnAllyGrowth` | `ModifyStats` | `self` | `attack`, `health`, `duration=permanent` | 每当一个友方随从获得成长，获得 +{attack}/+{health} |
| `on-play-battlecry-cleanse-single` | `OnPlay` | `Cleanse` | `friendly:single` | `duration=combat`（固定） | 战吼：清除一个友方随从的全部传染；本场战斗内不再获得传染 |
| `on-play-battlecry-cleanse-all-enemy` | `OnPlay` | `Cleanse` | `enemy:all` | `duration=instant`（固定） | 战吼：清除敌方所有随从的传染 |
| `turn-start-gain-gold` | `OnTurnStart` | `GainGold` | `self` | `amount` | 回合开始时，获得 {amount} 金币 |
| `activate-stats-single-ps-both` | `OnActivate` | `ModifyStats` | `friendly:single:playstyle={playstyle}` | `playstyle`, `attack`, `health`, `duration=permanent` | 使一个友方{playstyle}随从获得 +{attack}/+{health} |
| `combatstart-chance-growth-all` | `OnCombatStart` | `Growth` | `friendly:all` | `times`, `chance` | 战斗开始时，有 {chance.percent} 概率使你的所有随从获得{times.cn}次成长 |
| `on-play-effect-cleanse-all-friendly` | `OnPlayEffect` | `Cleanse` | `friendly:all` | `duration=combat`（固定） | 清除你的所有随从的传染；本场战斗内不再获得传染 |
| `on-play-effect-ramp-growth-single` | `OnPlayEffect` | `Ramp` | `friendly:single` | `every`, `max` | 使一个友方随从获得等同于本局回合数除以 {every} 的成长（向上取整，至多 {max} 次） |
| `on-play-effect-sacrifice` | `OnPlayEffect` | `Sacrifice` | `friendly:single` | — | 献祭一个友方随从 |
| `on-play-effect-gain-gold` | `OnPlayEffect` | `GainGold` | `self` | `amount` | 获得 {amount} 金币 |

参数域：

- `chance`：`int 1..99`，文案用 `{chance.percent}`；
- `every`：`int 1..4`；`max`：`int 1..4`；
- `amount`：`int 1..3`（金币）；
- `attack` / `health`：`int`，允许负值；
- `times`：`int 1..3`；
- `count`：`int 1..4`。

`GainGold` 相关的三个模板不登记关键词；`Cleanse` 相关模板也不登记。

#### 6.3 复用清单

| 复用对象 | 用在哪张卡 | 说明 |
| --- | --- | --- |
| `aura-all-taxon-both` | 珍·古德（随从） | 传 `taxon=primates`；需要名词表新增「灵长目」 |
| `on-play-battlecry-summon` | 白胡子大卫 | 传 `termite-token`、`count=2`；文案覆盖 |
| `battlecry-friendly-mammal-growth-once` | 因达 | 现有技能直接引用 |
| `on-death-summon` | 泰特拉一世 | 传 `monkey-token`、`count=1`；文案覆盖 |
| `battlecry-friendly-protozoa-growth-once` | 列文虎克 | 现有技能直接引用 |
| `on-play-battlecry-growth-single` | 梵高 | 传 `times=3` |
| `battlecry-all-mech-growth-once` | 贝尔 | 现有技能直接引用 |
| `aura-mech-plus-2-plus-2` | 诺贝尔 | 现有技能直接引用 |
| `keyword-assemble-1` | 手工耿（随从）、布里切特三世 | 现有技能直接引用 |
| `keyword-taunt` | 阿兹、因达、奥利佛、手工耿（随从） | 现有技能直接引用 |
| `activate-discover-playstyle` | 珍·古德（英雄） | 新技能实例 `behavioral-observation` 复用模板 |

#### 6.4 数据与文本补充

| 位置 | 变更 | 原因 |
| --- | --- | --- |
| [分类树与玩法](../taxonomy.md) §1 | 新增 `cercopithecidae \| 猴科 \| primates \| 科 \| 猴类`，以及扩展批次的 45 个科/目级叶子（清单见 §9.2） | 衍生物、泰特拉一世与扩展物种的 `taxon` 需要叶子 |
| [卡面名词](../card-nouns.md) §1 | 新增 `primates \| 灵长目`；如模板文案使用猴科，再新增 `cercopithecidae \| 猴科` | 模板只从名词表取卡面写法，不回退分类树 |
| 文本表 | `gen_text.py` 重新生成卡名、英雄介绍、技能文案与金色文案 | 新卡与新技能的展示文本 |
| 校验脚本 | 英雄总数 32 → 40；新增触发/原语/参数/前缀校验 | 见 §9 |
| [数值与经济](../../systems/economy-and-numbers.md) §1 | 补充 `GainGold` 的每回合来源上限（`OnSell` ≤3、`GainGold` 合计 ≤5），并声明七下西洋等既有来源不受限 | 新金币原语需要与既有「不设上限」口径区分 |
| 索引 | [卡牌索引](../cards/README.md)、[英雄卡索引](../heroes/README.md)、[技能索引](../skills/README.md) 补行 | 文档契约 |

`subtype` 只取文本表已有的通用值：首批用 `primate`、`quantum`、`robot`、`medicine`、`microbiology`、`physics`、`chemistry`、`economics`、`mathematics`、`art`、`invention`、`communication`；扩展批次按玩法取 `bird`、`insect`、`fish`、`mollusk`、`ocean`、`fungus`、`bacteria`、`protozoa`、`virus`、`reptile`、`moss`、`fern`、`flower`、`tree`、`crop`、`aquatic-plant`、`robot`、`invention`、`navigation`、`computer`、`spaceflight`、`animal`，均已在文本表中存在。

#### 6.5 扩展技能实例（33）

| 技能 | 模板调用 | 关键词 | 文案 |
| --- | --- | --- | --- |
| `aura-other-cnidarian-plus-0-plus-1` | `aura-all-ps-other-health(playstyle=cnidarian,health=1)` | 无 | 你的其他水螅纲随从获得 +0/+1 |
| `aura-other-cnidarian-plus-0-plus-2` | `aura-all-ps-other-health(playstyle=cnidarian,health=2)` | 无 | 你的其他水螅纲随从获得 +0/+2 |
| `aura-other-bacteria-plus-0-plus-1` | `aura-all-ps-other-health(playstyle=bacteria,health=1)` | 无 | 你的其他细菌域随从获得 +0/+1 |
| `battlecry-friendly-bird-growth-twice` | `on-play-battlecry-growth-single-ps(playstyle=bird,times=2)` | battlecry | 战吼：使一个友方鸟纲随从获得两次成长 |
| `battlecry-friendly-insect-growth-once` | `on-play-battlecry-growth-single-ps(playstyle=insect,times=1)` | battlecry | 战吼：使一个友方昆虫纲随从获得一次成长 |
| `battlecry-friendly-insect-growth-twice` | `on-play-battlecry-growth-single-ps(playstyle=insect,times=2)` | battlecry | 战吼：使一个友方昆虫纲随从获得两次成长 |
| `battlecry-friendly-fish-growth-twice` | `on-play-battlecry-growth-single-ps(playstyle=fish,times=2)` | battlecry | 战吼：使一个友方鱼纲随从获得两次成长 |
| `battlecry-friendly-fungus-growth-twice` | `on-play-battlecry-growth-single-ps(playstyle=fungus,times=2)` | battlecry | 战吼：使一个友方真菌随从获得两次成长 |
| `battlecry-friendly-ceph-growth-twice` | `on-play-battlecry-growth-single-ps(playstyle=cephalopod,times=2)` | battlecry | 战吼：使一个友方头足纲随从获得两次成长 |
| `battlecry-friendly-ceph-growth-thrice` | `on-play-battlecry-growth-single-ps(playstyle=cephalopod,times=3)` | battlecry | 战吼：使一个友方头足纲随从获得三次成长 |
| `battlecry-friendly-bacteria-growth-once` | `on-play-battlecry-growth-single-ps(playstyle=bacteria,times=1)` | battlecry | 战吼：使一个友方细菌域随从获得一次成长 |
| `battlecry-friendly-bacteria-growth-twice` | `on-play-battlecry-growth-single-ps(playstyle=bacteria,times=2)` | battlecry | 战吼：使一个友方细菌域随从获得两次成长 |
| `battlecry-friendly-mech-growth-twice` | `on-play-battlecry-growth-single-ps(playstyle=machine,times=2)` | battlecry | 战吼：使一个友方机械随从获得两次成长 |
| `deathrattle-friendly-growth-once` | `on-death-growth-single(times=1)` | deathrattle | 亡语：使一个友方随从获得一次成长 |
| `deathrattle-friendly-fungus-growth-twice` | `on-death-growth-single-ps(playstyle=fungus,times=2)` | deathrattle | 亡语：使一个友方真菌随从获得两次成长 |
| `deathrattle-friendly-fungus-growth-once` | `on-death-growth-single-ps(playstyle=fungus,times=1)` | deathrattle | 亡语：使一个友方真菌随从获得一次成长 |
| `deathrattle-summon-1-moth` | `on-death-summon(token=moth-token,count=1)` | deathrattle | 亡语：召唤一个 1/1 的蚕蛾 |
| `swap-stats-friendly` | `on-play-swap-stats-single` | 无 | 战吼：使一个友方随从的攻击力与生命值互换 |
| `swap-stats-friendly-plus-1` | `on-play-effect-swap-stats-plus(attack=1,health=1)` | 无 | 使一个友方随从的攻击力与生命值互换，然后获得 +1/+1 |
| `freeze-enemy-leftmost` | `combatstart-freeze-single` | 无 | 战斗开始时，冻结敌方最左侧随从 |
| `chance-self-attack-2` | `combatstart-chance-self-attack(chance=50,attack=2)` | 无 | 战斗开始时，有 50% 概率获得 +2 攻击力 |
| `chance-growth-all-50` | `combatstart-chance-growth-all(chance=50,times=1)` | 无 | 战斗开始时，有 50% 概率使你的所有随从获得一次成长 |
| `chance-enemy-single-minus-3-75` | `combatstart-chance-stats-enemy-single(chance=75,attack=-3)` | 无 | 战斗开始时，有 75% 概率使一个敌方随从获得 -3 攻击力 |
| `silence-enemy-single` | `combatstart-silence-single` | 无 | 战斗开始时，使一个敌方随从失去全部技能与关键词，直到本场战斗结束 |
| `silence-all-enemy` | `combatstart-silence-all-enemies` | 无 | 使敌方所有随从失去全部技能与关键词，直到本场战斗结束 |
| `enemy-all-minus-2-attack` | `combatstart-stats-enemy-all-attack(attack=-2)` | 无 | 战斗开始时，使敌方所有随从获得 -2 攻击力 |
| `gain-gold-2` | `on-play-effect-gain-gold(amount=2)` | 无 | 获得 2 金币 |
| `summon-two-trilobites` | `on-play-summon(token=trilobite-token,count=2)` | 无 | 召唤两个 3/3 的三叶虫 |
| `buy-gain-1-gold` | `on-buy-gain-gold(amount=1)` | 无 | 购买时：获得 1 金币 |
| `uncertainty` | `activate-growth-random(times=2)` | 无 | 使一个随机友方随从获得两次成长；费用 2，每回合 1 次 |
| `quantum-observation` | `activate-discover-playstyle` | 无 | 发现一张本局启用的玩法随从；费用 2，每回合 1 次 |
| `antibiosis` | `activate-cleanse-single` | 无 | 清除一个友方随从的全部传染；本场战斗内不再获得传染；费用 1，每回合 1 次 |
| `printing-press` | `activate-copy-hand-minion` | 无 | 复制你手牌中的一张随从牌；费用 2，每回合 1 次 |

#### 6.6 扩展模板（10）

| 模板 id | 触发 | 原语 | 选择器 | 参数 | 文案 |
| --- | --- | --- | --- | --- | --- |
| `on-death-growth-single` | `OnDeath` | `Growth` | `friendly:single` | `times` | 亡语：使一个友方随从获得{times.cn}次成长 |
| `on-play-swap-stats-single` | `OnPlay` | `SwapStats` | `friendly:single` | `—` | 战吼：使一个友方随从的攻击力与生命值互换 |
| `on-play-effect-swap-stats-plus` | `OnPlayEffect` | `SwapStats` | `friendly:single` | `attack,health` | 使一个友方随从的攻击力与生命值互换，然后获得 +{attack}/+{health} |
| `combatstart-freeze-single` | `OnCombatStart` | `Freeze` | `enemy:leftmost` | `—` | 战斗开始时，冻结敌方最左侧随从 |
| `combatstart-chance-stats-enemy-single` | `OnCombatStart` | `ModifyStats` | `enemy:single` | `attack,duration=combat,chance` | 战斗开始时，有 {chance.percent} 概率使一个敌方随从获得 {attack} 攻击力 |
| `combatstart-silence-single` | `OnCombatStart` | `Silence` | `enemy:single` | `—` | 战斗开始时，使一个敌方随从失去全部技能与关键词，直到本场战斗结束 |
| `combatstart-silence-all-enemies` | `OnCombatStart` | `Silence` | `enemy:all` | `—` | 使敌方所有随从失去全部技能与关键词，直到本场战斗结束 |
| `activate-growth-random` | `OnActivate` | `Growth` | `friendly:random` | `times` | 使一个随机友方随从获得{times.cn}次成长 |
| `activate-cleanse-single` | `OnActivate` | `Cleanse` | `friendly:single` | `duration=combat` | 清除一个友方随从的全部传染；本场战斗内不再获得传染 |
| `on-buy-gain-gold` | `OnBuy` | `GainGold` | `self` | `amount` | 购买时：获得 {amount} 金币 |

#### 6.7 诺奖效果牌技能实例（15）

| 技能 | 模板调用 | 关键词 | 文案 |
| --- | --- | --- | --- |
| `cannot-attack-enemy-single` | `combatstart-cannot-attack-single` | 无 | 战斗开始时，使一个敌方随从本回合无法攻击 |
| `stats-leftmost-plus-2-plus-2` | `on-play-effect-stats-leftmost(attack=2,health=2)` | 无 | 使最左侧的友方随从获得 +2/+2 |
| `cleanse-friendly-single-stealth` | `on-play-effect-cleanse-stealth` | 无 | 清除一个友方随从的全部传染，并使其获得潜行直到回合结束 |
| `sacrifice-all-friendly-plus-1-plus-1` | `on-play-effect-sacrifice-then-all-stats(attack=1,health=1)` | 无 | 献祭一个友方随从，使你的所有随从获得 +1/+1 |
| `copy-friendly-minion-plus-1-plus-1` | `on-play-effect-copy-plus(attack=1,health=1)` | 无 | 复制一个友方随从，然后使一个友方随从获得 +1/+1 |
| `ramp-all-friendly-growth` | `on-play-effect-ramp-all(every=2,max=2)` | 无 | 使你的所有随从获得等同于本局回合数除以 2 的成长（向上取整，至多 2 次） |
| `stats-enemy-single-minus-3` | `combatstart-stats-enemy-single(attack=-3)` | 无 | 战斗开始时，使一个敌方随从获得 -3 攻击力 |
| `stats-friendly-single-plus-3-attack` | `on-play-effect-stats-single(attack=3)` | 无 | 使一个友方随从获得 +3 攻击力 |
| `stats-friendly-single-plus-3-plus-3` | `on-play-effect-stats-single(attack=3,health=3)` | 无 | 使一个友方随从获得 +3/+3 |
| `stats-all-friendly-plus-2-plus-2` | `on-play-effect-stats-all-friendly(attack=2,health=2)` | 无 | 使你的所有随从获得 +2/+2 |
| `stats-all-friendly-plus-0-plus-3` | `on-play-effect-stats-all-friendly(attack=0,health=3)` | 无 | 使你的所有随从获得 +0/+3 |
| `shield-all-friendly` | `on-play-effect-shield-all` | 无 | 使你的所有随从在本场战斗内获得护盾 |
| `stats-enemy-all-minus-3-attack` | `combatstart-stats-enemy-all-attack(attack=-3)` | 无 | 战斗开始时，使敌方所有随从获得 -3 攻击力 |
| `freeze-enemy-single` | `combatstart-freeze-enemy-single` | 无 | 战斗开始时，冻结一个敌方随从 |
| `freeze-enemy-all` | `combatstart-freeze-enemy-all` | 无 | 战斗开始时，冻结敌方所有随从 |

以上技能均不登记关键词：护盾、潜行是赋予他人，不是携带者自身登记。`shield-all-friendly` 走既有 `GrantKeyword`，`freeze-enemy-single` 与 `freeze-enemy-all` 走 §5.8 的 `Freeze`，`cleanse-friendly-single-stealth` 先 `Cleanse` 再 `GrantKeyword`。`ramp-all-friendly-growth` 的上限固定为 2，低于 §5.3 单体 `Ramp` 的 4，避免群体递增在长局里失控。

#### 6.8 诺奖效果牌模板（12）

| 模板 id | 触发 | 原语 | 选择器 | 参数 | 文案 |
| --- | --- | --- | --- | --- | --- |
| `on-play-effect-stats-single` | `OnPlayEffect` | `ModifyStats` | `friendly:single` | `attack`, `health`, `duration=permanent` | 使一个友方随从获得 +{attack}/+{health} |
| `on-play-effect-stats-all-friendly` | `OnPlayEffect` | `ModifyStats` | `friendly:all` | `attack`, `health`, `duration=permanent` | 使你的所有随从获得 +{attack}/+{health} |
| `on-play-effect-stats-leftmost` | `OnPlayEffect` | `ModifyStats` | `friendly:leftmost` | `attack`, `health`, `duration=permanent` | 使最左侧的友方随从获得 +{attack}/+{health} |
| `combatstart-stats-enemy-single` | `OnCombatStart` | `ModifyStats` | `enemy:single` | `attack`, `duration=combat` | 战斗开始时，使一个敌方随从获得 {attack} 攻击力 |
| `combatstart-cannot-attack-single` | `OnCombatStart` | `CannotAttack` | `enemy:single` | `duration=combat` | 战斗开始时，使一个敌方随从本回合无法攻击 |
| `combatstart-freeze-enemy-single` | `OnCombatStart` | `Freeze` | `enemy:single` | — | 战斗开始时，冻结一个敌方随从 |
| `combatstart-freeze-enemy-all` | `OnCombatStart` | `Freeze` | `enemy:all` | — | 战斗开始时，冻结敌方所有随从 |
| `on-play-effect-cleanse-stealth` | `OnPlayEffect` | `Cleanse`；`GrantKeyword` | `friendly:single` | `keyword=stealth`, `duration=combat` | 清除一个友方随从的全部传染，并使其获得潜行直到回合结束 |
| `on-play-effect-sacrifice-then-all-stats` | `OnPlayEffect` | `Sacrifice`；`ModifyStats` | `friendly:single`；`friendly:all` | `attack`, `health` | 献祭一个友方随从，使你的所有随从获得 +{attack}/+{health} |
| `on-play-effect-copy-plus` | `OnPlayEffect` | `Copy`；`ModifyStats` | `friendly:single`；`friendly:single` | `attack`, `health`, `to=hand` | 复制一个友方随从，然后使一个友方随从获得 +{attack}/+{health} |
| `on-play-effect-ramp-all` | `OnPlayEffect` | `Ramp` | `friendly:all` | `every`, `max` | 使你的所有随从获得等同于本局回合数除以 {every} 的成长（向上取整，至多 {max} 次） |
| `on-play-effect-shield-all` | `OnPlayEffect` | `GrantKeyword` | `friendly:all` | `keyword=divine_shield`, `duration=combat` | 使你的所有随从在本场战斗内获得护盾 |

参数域沿用 §6.2 与 §6.6：`attack` / `health` 为整数，允许 0 与负值；`every` 1..4；`max` 1..4；`duration` 只取 `permanent` / `combat`（引擎当前只接受这两个值，「本回合」与「直到回合结束」都落到 `combat`）。多原语模板的结算顺序固定为表中原语顺序：`sacrifice-then-all-stats` 先献祭再群体加身材，献祭失败（没有可选友方随从）时群体加身材仍结算；`cleanse-stealth` 先清除传染再给潜行；`copy-plus` 先复制到手牌，再给一个友方随从加身材（两次选择可以指向不同目标）。

### 7. 卡牌设计

以下字段块可直接转成实体文档；`## 属性` 表、`## 效果`、`## 设计说明`、`## 出处` 在落地时按[卡牌规范](../card-system.md) §1 的正文约定补齐。

#### 7.1 随从牌（20）

##### 1. 珍·古德（jane-goodall）

```yaml
id: jane-goodall
name: 珍·古德
card_type: minion
categories: [play:mammal]
taxon: hominidae
subtype: primate
level: 5
era: modern
attack: 0
health: 17
skills: [aura-primates-plus-1-plus-1]
```

**效果**：你的灵长目随从获得 +1/+1

**设计说明**：灵长类子流派的锚点。用 `taxon=primates` 同时覆盖人科与猴科，把「黑猩猩体系」从单卡联动升级为一个可辨认的光环。0 攻击 17 生命符合 5 星 0 攻的取值。

**支持状态**：A，复用 `aura-all-taxon-both`，只需补名词表。

**出处**：珍·古德自 1960 年起在坦桑尼亚贡贝溪国家公园研究野生黑猩猩。

##### 2. 黑猩猩「白胡子」大卫（david）

```yaml
id: david
name: 黑猩猩「白胡子」大卫
card_type: minion
categories: [play:mammal, play:insect]
taxon: hominidae
subtype: primate
level: 3
era: modern
attack: 4
health: 5
skills: [battlecry-summon-2-termites]
```

**效果**：战吼：召唤两个 2/2 的白蚁

**设计说明**：把素材库的「钓白蚁」从不可结算的动作改成召唤。跨哺乳纲与昆虫纲归属，既补昆虫纲的 3 星，也让灵长类卡能进入昆虫卡池。4/5 加 4 点召唤折算正好 13，卡在 3 星上限。

**支持状态**：A，复用 `on-play-battlecry-summon`；新技能实例 + 文案覆盖。

**出处**：「白胡子」大卫是贡贝黑猩猩群中第一只接近古德的成年雄性，以用草茎钓白蚁闻名。

##### 3. 猩猩阿兹（az）

```yaml
id: az
name: 猩猩阿兹
card_type: minion
categories: [play:mammal, play:machine]
taxon: hominidae
subtype: primate
level: 4
era: modern
attack: 5
health: 6
skills: [conditional-machine-self-attack-2, keyword-taunt]
```

**效果**：若你控制至少 1 个机械随从，获得 +2 攻击力；嘲讽

**设计说明**：素材库的「跨流派桥梁」。条件光环复用 `ConditionalAura`，把机械条件换成自身攻击力；嘲讽让它站在前排，符合「阿兹是群体中的守卫者」的叙事。5/6 + 2 攻 + 0.5 嘲讽 = 13.5，低于 4 星上限 17。

**支持状态**：B，新增 `conditional-playstyle-self-attack`。

**出处**：阿兹是语言实验中的黑猩猩个体；素材库条目未给出可查证来源，出处待补。

##### 4. 猩猩因达（inda）

```yaml
id: inda
name: 猩猩因达
card_type: minion
categories: [play:mammal, play:machine]
taxon: hominidae
subtype: primate
level: 3
era: modern
attack: 3
health: 5
skills: [battlecry-friendly-mammal-growth-once, keyword-taunt]
```

**效果**：战吼：使一个友方哺乳纲随从获得一次成长；嘲讽

**设计说明**：与阿兹组成「机械—哺乳」双向桥：阿兹吃机械在场，因达把成长送回哺乳纲。3/5 + 2 成长 + 0.5 嘲讽 = 10.5，低于 3 星上限 13。

**支持状态**：A，现有技能直接引用。

**出处**：因达是语言实验中的黑猩猩个体；素材库条目未给出可查证来源，出处待补。

##### 5. 猴子泰特拉一世（tetra-i）

```yaml
id: tetra-i
name: 猴子泰特拉一世
card_type: minion
categories: [play:mammal]
taxon: cercopithecidae
subtype: primate
level: 3
era: modern
attack: 3
health: 6
skills: [deathrattle-summon-monkey]
```

**效果**：亡语：召唤一个 2/2 的猴子

**设计说明**：把「召唤链过长」压成单次召唤。3/6 + 4 点召唤 = 13，正好 3 星上限。需要新增猴科节点与 `monkey-token`。

**支持状态**：A，复用 `on-death-summon`；新增衍生物与分类节点。

**出处**：素材库条目未给出可查证来源，出处待补。

##### 6. 布里切特三世（brichett-iii）

```yaml
id: brichett-iii
name: 布里切特三世
card_type: minion
categories: [play:machine]
taxon: automation
subtype: robot
level: 4
era: future
attack: 5
health: 7
skills: [ally-summon-self-attack-1, keyword-assemble-1]
```

**效果**：装配 1：进入战场时，使你的其他机械随从各获得 1 次成长；每当一个友方随从进入战场，获得 +1 攻击力

**设计说明**：素材库的「猴子部队体系核心」。入场的随从越多，它越能滚攻击力；装配负责把铺场转成机械成长。5/7 + 2 点事件效果 + 2 点装配 = 16，低于 4 星上限 17。

**支持状态**：B，新增 `ally-summon-self-attack`。

**出处**：素材库条目为「猴子驾驶机械」的设定，未给出可查证来源，出处待补。

##### 7. 黑猩猩奥利佛（oliver）

```yaml
id: oliver
name: 黑猩猩奥利佛
card_type: minion
categories: [play:mammal]
taxon: hominidae
subtype: primate
level: 3
era: modern
attack: 4
health: 4
skills: [battlecry-friendly-primate-growth-twice, keyword-taunt]
```

**效果**：战吼：使一个友方灵长目随从获得两次成长；嘲讽

**设计说明**：素材库的「调酒增益」改成成长。嘲讽提供站位价值，成长把资源集中到单个灵长类核心。4/4 + 2 点成长 + 0.5 嘲讽 = 10.5，低于 3 星上限 13。

**支持状态**：B，新增 `on-play-battlecry-growth-single-taxon`。

**出处**：奥利佛是一只因外貌与行为拟人而受到关注的黑猩猩。

##### 8. 小猴子海利恩（helion）

```yaml
id: helion
name: 小猴子海利恩
card_type: minion
categories: [play:mammal]
taxon: hominidae
subtype: primate
level: 2
era: modern
attack: 4
health: 3
skills: [sell-refund-2]
```

**效果**：出售时：返还 2 金币

**设计说明**：素材库的「回手效果」与战棋骨架冲突，改为出售返还。2★ 4/3 加 2 点经济效果 = 9，低于 2 星上限 10。买 3 卖 3，是「免费租用」而不是净赚；每回合金币上限防止多张叠加。

**支持状态**：C，新增 `OnSell` 与 `GainGold`。

**出处**：素材库条目未给出可查证来源，出处待补。

##### 9. 薛定谔的猫（schrodinger-cat）

```yaml
id: schrodinger-cat
name: 薛定谔的猫
card_type: minion
categories: [play:mammal]
taxon: felidae
subtype: quantum
level: 3
era: modern
attack: 4
health: 6
skills: [chance-cat-attack]
```

**效果**：战斗开始时，有 50% 概率获得 +3 攻击力

**设计说明**：把素材库的「随机攻击力」改成结算时的一次概率判定，随机走种子流。4/6 加期望 1.5 攻（按 2 点折算）= 12，低于 3 星上限 13。猫科让科学分类多一个「科」，但不新增玩法。

**支持状态**：C，新增 `chance` 参数与 `.percent` 修饰符。

**出处**：薛定谔 1935 年提出「猫」思想实验，用来说明量子叠加与观测。

##### 10. 费波那契（fibonacci）

```yaml
id: fibonacci
name: 费波那契
card_type: minion
categories: [play:mammal]
taxon: hominidae
subtype: mathematics
level: 6
era: feudal
attack: 0
health: 21
skills: [fibonacci-ramp]
```

**效果**：回合开始时，获得等同于本局回合数除以 2 的成长（向上取整，至多 4 次）

**设计说明**：素材库指出斐波那契成长曲线难平衡，本文把指数曲线线性化为「每两回合 +1、封顶 4」。6 星 0/21 按 0 攻规则取值；折算 4 点后 25，低于 6 星上限 26。拖到后期的收益高，但不会突破单随从成长上限。

**支持状态**：C，新增 `Ramp` 原语。

**出处**：斐波那契在 1202 年《计算之书》中提出以 1、1 开始的数列。

##### 11. 普朗克（planck）

```yaml
id: planck
name: 普朗克
card_type: minion
categories: [play:mammal]
taxon: hominidae
subtype: quantum
level: 5
era: modern
attack: 0
health: 17
skills: [battlecry-enemy-all-minus-1-attack]
```

**效果**：战吼：使敌方所有随从获得 -1 攻击力

**设计说明**：量子子流派的「削攻」端：不直接伤害，而是压低敌方攻击力，配合概率与成长拖长战斗。战吼中的敌方效果按核心规则延迟到战斗开始时结算。

**支持状态**：B，新增 `on-play-battlecry-stats-enemy-all-attack`。

**出处**：普朗克 1900 年提出能量量子化假设。

##### 12. 麦克斯韦（maxwell）

```yaml
id: maxwell
name: 麦克斯韦
card_type: minion
categories: [play:machine]
taxon: hominidae
subtype: physics
level: 5
era: industrial
attack: 6
health: 11
skills: [ally-growth-self-both-1]
```

**效果**：每当一个友方随从获得成长，获得 +1/+1

**设计说明**：把「成长」当作电磁感应：队伍每次成长都给它一次自身增益，天然与装配、繁衍、灵长类成长联动。6/11 + 2 点事件效果 = 19，低于 5 星上限 22。

**支持状态**：B，新增 `ally-growth-self-both`。

**出处**：麦克斯韦 1865 年发表电磁场方程组。

##### 13. 爱德华·詹纳（jenner）

```yaml
id: jenner
name: 爱德华·詹纳
card_type: minion
categories: [play:bacteria]
taxon: hominidae
subtype: medicine
level: 4
era: industrial
attack: 0
health: 13
skills: [battlecry-cleanse-friendly-single-immunity]
```

**效果**：战吼：清除一个友方随从的全部传染；本场战斗内不再获得传染

**设计说明**：疫苗体系第一张。它不消灭细菌，而是让己方关键随从不再被传染叠层，克制病毒/细菌的爆发。4 星 0/13 按 0 攻规则取值。

**支持状态**：C，新增 `Cleanse` 原语。

**出处**：詹纳 1796 年用牛痘接种预防天花。

##### 14. 约瑟夫·李斯特（lister）

```yaml
id: lister
name: 约瑟夫·李斯特
card_type: minion
categories: [play:bacteria]
taxon: hominidae
subtype: medicine
level: 4
era: industrial
attack: 4
health: 9
skills: [battlecry-cleanse-all-enemy]
```

**效果**：战吼：清除敌方所有随从的传染

**设计说明**：消毒是「清场」而不是「免疫」：它把敌方已经叠好的传染拆掉，但本场仍可能再被叠。与詹纳同为 4 星细菌域，但技能集不同（单体免疫 vs 群体清除），不触发「同玩法同星级效果技能集最多 1 张」的冲突。

**支持状态**：C，新增 `Cleanse`。

**出处**：李斯特 1867 年以石炭酸消毒改进外科手术。

##### 15. 安东尼·范·列文虎克（leeuwenhoek）

```yaml
id: leeuwenhoek
name: 安东尼·范·列文虎克
card_type: minion
categories: [play:protozoa]
taxon: hominidae
subtype: microbiology
level: 3
era: feudal
attack: 3
health: 6
skills: [battlecry-friendly-protozoa-growth-once]
```

**效果**：战吼：使一个友方原生生物界随从获得一次成长

**设计说明**：原生生物界的低费补强。素材库的「机制为空」在这里落成最简单的成长，不新增子系统。3/6 + 2 点成长 = 11，低于 3 星上限 13。

**支持状态**：A，现有技能直接引用。

**出处**：列文虎克在 1670 年代用自制显微镜首次观察到微生物。

##### 16. 梵高（vangogh）

```yaml
id: vangogh
name: 梵高
card_type: minion
categories: [play:mammal]
taxon: hominidae
subtype: art
level: 5
era: industrial
attack: 0
health: 17
skills: [battlecry-friendly-growth-thrice]
```

**效果**：战吼：使一个友方随从获得三次成长

**设计说明**：艺术体系的第一张锚点。素材库的「机制为空」用高次数单体成长填充；0 攻让位给友方核心，符合「艺术家不亲自上阵」的叙事。

**支持状态**：A，复用 `on-play-battlecry-growth-single`。

**出处**：梵高在 1888—1889 年创作《向日葵》系列。

##### 17. 凯恩斯（keynes）

```yaml
id: keynes
name: 凯恩斯
card_type: minion
categories: [play:mammal]
taxon: hominidae
subtype: economics
level: 5
era: modern
attack: 0
health: 17
skills: [turn-start-gain-1-gold]
```

**效果**：回合开始时，获得 1 金币

**设计说明**：经济流派的最小可行单卡。素材库说经济效果容易过强，本文先只做「每回合 1 金币」并设全局上限；如果模拟过强，先改触发时机或提星，不直接加数值。

**支持状态**：C，新增 `GainGold`。

**出处**：凯恩斯 1936 年出版《就业、利息和货币通论》。

##### 18. 贝尔（bell）

```yaml
id: bell
name: 贝尔
card_type: minion
categories: [play:machine]
taxon: hominidae
subtype: communication
level: 6
era: industrial
attack: 9
health: 13
skills: [battlecry-all-mech-growth-once]
```

**效果**：战吼：使你的所有机械随从获得一次成长

**设计说明**：素材库建议「莱特兄弟/贝尔/诺贝尔合并为机械光环」。贝尔做群体成长，负责把机械铺场转成身材；9/13 + 4 点召唤级折算 = 26，正好 6 星上限。

**支持状态**：A，现有技能直接引用。

**出处**：贝尔 1876 年获得电话专利。

##### 19. 诺贝尔（nobel）

```yaml
id: nobel
name: 诺贝尔
card_type: minion
categories: [play:machine]
taxon: hominidae
subtype: chemistry
level: 7
era: industrial
attack: 11
health: 17
skills: [aura-mech-plus-2-plus-2]
```

**效果**：你的机械随从获得 +2/+2

**设计说明**：机械体系终局光环。7 星 11/17 加 4 点光环 = 32，低于 7 星上限 33。与贝尔分工：贝尔是一次性群体成长，诺贝尔是持续光环。

**支持状态**：A，现有技能直接引用。

**出处**：诺贝尔 1867 年发明达纳炸药，并以遗嘱设立诺贝尔奖。

##### 20. 手工耿（shougeng）

```yaml
id: shougeng
name: 手工耿
card_type: minion
categories: [play:machine]
taxon: hominidae
subtype: invention
level: 4
era: modern
attack: 6
health: 7
skills: [keyword-assemble-1, keyword-taunt]
```

**效果**：装配 1：进入战场时，使你的其他机械随从各获得 1 次成长；嘲讽

**设计说明**：同名英雄卡的随从版本。素材库说「召唤物已有同名卡，作为彩蛋英雄卡」，本文让英雄与随从同 id 同名、技能各自维护：随从是装配前排，英雄是主动加身材。6/7 + 2 点装配 + 0.5 嘲讽 = 15.5，低于 4 星上限 17。

**支持状态**：A，现有技能直接引用。

**出处**：手工耿以「无用发明」系列视频为人所知。

#### 7.2 英雄卡（2）

英雄卡不带 `card_type`、星级、攻击、生命、时代、科学分类与费用；`introduction` 40–80 字，正文 `## 简介` 与之一致。

##### 21. 珍·古德（jane-goodall，英雄）

```yaml
id: jane-goodall
name: 珍·古德
categories: [play:mammal]
introduction: 英国灵长类学家，自 1960 年起在坦桑尼亚贡贝研究野生黑猩猩，记录其工具使用与社会行为，改变了人类对灵长类行为的认识。
skills: [behavioral-observation]
```

**效果**：发现一张本局启用的玩法随从

**设计说明**：主动技能 2 费、每回合 1 次，与达尔文的「进化」同模板但独立命名与计次。英雄身份给「观察」，同名随从给灵长类光环，两条线分别服务招募稳定性与阵容方向。

**支持状态**：A，复用 `activate-discover-playstyle`。

##### 22. 手工耿（shougeng，英雄）

```yaml
id: shougeng
name: 手工耿
categories: [play:machine]
introduction: 中国创作者，以把废旧材料做成「无用发明」的作品闻名；他的机械装置夸张而精巧，常被视为民间发明文化的代表。
skills: [activate-friendly-mech-plus-2-plus-2]
```

**效果**：使一个友方机械随从获得 +2/+2

**设计说明**：主动技能 1 费、每回合 1 次，给机械体系一个可主动投入的单体成长位；与爱迪生的群体成长形成「单点强化 vs 群体铺开」的差异。

**支持状态**：B，新增 `activate-stats-single-ps-both`。

#### 7.3 效果牌（4）

##### 23. 薛定谔气体模型（schrodinger-gas）

```yaml
id: schrodinger-gas
name: 薛定谔气体模型
card_type: effect
categories: [play:mammal, play:machine]
taxon: physics
level: 2
era: modern
cost: 1
skills: [gas-model-chance-growth]
```

**效果**：战斗开始时，有 50% 概率使你的所有随从获得一次成长

**设计说明**：素材库的「机制为空，作为量子流派种子卡」。它用同一套概率参数做成群体成长，给量子子流派一个低费入口；期望收益约等于 2 次成长，符合 1 费效果牌上限。

**支持状态**：C，新增 `chance` 参数。

**出处**：素材库条目未给出可查证来源，出处待补。

##### 24. 疫苗（yimiao）

```yaml
id: yimiao
name: 疫苗
card_type: effect
categories: [play:bacteria, play:virus]
taxon: medicine
level: 3
era: industrial
cost: 2
skills: [vaccine-cleanse]
```

**效果**：清除你的所有随从的传染；本场战斗内不再获得传染

**设计说明**：疫苗体系的群体版本。它不处理身材，只处理「传染」这一条负面资源，因此是针对性科技牌，而不是通用增益。3 星 2 费按 3 点小效果上限校验。

**支持状态**：C，新增 `Cleanse`。

**出处**：疫苗源于詹纳 1796 年的牛痘接种实践。

##### 25. 斐波那契数列（feibonaqi-shulie）

```yaml
id: feibonaqi-shulie
name: 斐波那契数列
card_type: effect
categories: [play:mammal, play:machine]
taxon: mathematics
level: 4
era: feudal
cost: 2
skills: [sequence-ramp-growth-2-4]
```

**效果**：使一个友方随从获得等同于本局回合数除以 2 的成长（向上取整，至多 4 次）

**设计说明**：把费波那契的「递增」做成一张可购买的效果牌，让没有抽到费波那契的玩家也能体验曲线成长。4 星 2 费，折算暂按 3 点，接近 2 费上限；若模拟过强，先降 `max`。

**支持状态**：C，新增 `Ramp`。

**出处**：斐波那契数列见 1202 年《计算之书》。

##### 26. 创造性破坏（chuangzaoxing-pohuai）

```yaml
id: chuangzaoxing-pohuai
name: 创造性破坏
card_type: effect
categories: [play:mammal, play:machine]
taxon: economics
level: 5
era: modern
cost: 2
skills: [creative-destruction]
```

**效果**：献祭一个友方随从，获得 3 金币

**设计说明**：经济流派的第二张卡，也是「牺牲换资源」的原型。献祭不触发亡语、复生与死亡事件，避免「召唤—献祭—再召唤」的金币循环；金币仍受每回合上限约束。5 星 2 费，强度不在身材而在节奏转换。

**支持状态**：C，新增 `Sacrifice` 与 `GainGold`。

**出处**：熊彼特在 1942 年《资本主义、社会主义与民主》中提出「创造性破坏」。

#### 7.4 衍生物（1）

##### 27. 猴子（monkey-token）

```yaml
id: monkey-token
name: 猴子（2/2）
card_type: minion
in_shop_pool: false
categories: [play:mammal]
taxon: cercopithecidae
subtype: primate
level: 1
era: modern
attack: 2
health: 2
```

**效果**：无

**设计说明**：泰特拉一世的亡语衍生物，不进商店、不扣牌池。命名带数值，避免模板重复渲染。

**支持状态**：A，新增非池卡与猴科节点。

**出处**：素材库条目未给出可查证来源，出处待补。

#### 7.5 扩展批次：随从牌（66）

各列口径同 §7.1；`分类` 为 `taxon · subtype`，`折算` 为纸面估算，`支持` 见 §4.3。

| id | 名称 | 玩法 | 分类 | 星级 | 时代 | 攻/血 | 技能 | 效果 | 折算 | 支持 | 出处 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `darwin-finch` | 达尔文雀 | 鸟纲 | fringillidae · bird | 1 | 现代 | 1/2 | `aura-other-biology-plus-1-attack` | 你的其他动物随从获得 +1 攻击力 | 2 | A | 达尔文 1835 年在加拉帕戈斯群岛采集的地雀，后来成为自然选择的经典证据。 |
| `firefly` | 萤火虫 | 昆虫纲 | lampyridae · insect | 1 | 原始 | 2/3 | `battlecry-friendly-growth-once` | 战吼：使一个友方随从获得一次成长 | 2 | A | 萤火虫科昆虫以腹部的发光器发出冷光，用于求偶与警戒。 |
| `goldfish` | 金鱼 | 鱼纲 | cyprinidae · fish | 1 | 封建 | 2/4 | `trigger-all-friendly-plus-0-plus-1` | 战吼：使你的所有随从获得 +0/+1 | 2 | A | 金鱼由宋代开始从鲫鱼中人工选育，是最早的观赏鱼之一。 |
| `firefly-squid` | 火焰乌贼 | 头足纲 | sepiidae · mollusk | 1 | 现代 | 2/2 | `trigger-friendly-plus-1-plus-1` | 战吼：使一个友方随从获得 +1/+1 | 2 | A | 火焰乌贼生活在澳大利亚近海，受惊时体表闪烁红黑相间的警戒色。 |
| `sand-anemone` | 沙海葵 | 水螅纲 | actiniidae · ocean | 1 | 史前 | 1/3 | `aura-other-cnidarian-plus-0-plus-1` | 你的其他水螅纲随从获得 +0/+1 | 2 | B | 海葵纲动物以基盘附着在沙质海底，触手带刺细胞。 |
| `rhizopus` | 根霉 | 真菌界 | mucoraceae · fungus | 1 | 史前 | 2/2 | `battlecry-friendly-growth-once` | 战吼：使一个友方随从获得一次成长 | 2 | A | 根霉属真菌的菌丝在淀粉质基质上迅速蔓延，常用于制曲。 |
| `bifidobacterium` | 双歧杆菌 | 细菌域 | bifidobacteriaceae · bacteria | 1 | 原始 | 1/3 | `trigger-friendly-single-plus-0-plus-2` | 战吼：使一个友方随从获得 +0/+2 | 2 | A | 双歧杆菌是人和动物肠道中的常见益生菌，1899 年由法国巴斯德研究所分离。 |
| `compass` | 指南针 | 机械 | automation · navigation | 1 | 古典 | 1/3 | `mechanism-art` | 战吼：使一个友方机械随从获得一次成长 | 2 | A | 中国战国时期已有司南，宋代用于航海指向。 |
| `dodo` | 渡渡鸟 | 鸟纲 | raphidae · bird | 2 | 现代 | 3/4 | `deathrattle-friendly-growth-once` | 亡语：使一个友方随从获得一次成长 | 2 | B | 渡渡鸟是毛里求斯特有的大型鸽类，17 世纪末因人类捕猎与外来物种灭绝。 |
| `hummingbird` | 蜂鸟 | 鸟纲 | trochilidae · bird | 2 | 现代 | 4/3 | `trigger-bird-single-windfury-this-turn` | 战吼：使一个友方鸟纲随从获得连击直到回合结束 | 3 | A | 蜂鸟每秒可振翅数十次，是唯一能悬停与倒飞的鸟类。 |
| `dung-beetle` | 蜣螂 | 昆虫纲 | scarabaeidae · insect | 2 | 史前 | 2/5 | `deathrattle-all-friendly-plus-0-plus-1` | 亡语：使你的所有随从获得 +0/+1 | 2 | A | 蜣螂把动物粪便滚成球运回巢中，古埃及人以圣甲虫象征太阳运行。 |
| `glass-octopus` | 玻璃章鱼 | 头足纲 | octopodidae · mollusk | 2 | 现代 | 2/4 | `trigger-friendly-single-plus-0-plus-2` | 战吼：使一个友方随从获得 +0/+2 | 2 | A | 玻璃章鱼除消化道外全身透明，生活在深海暮光带。 |
| `edible-jellyfish` | 海蜇 | 水螅纲 | rhizostomatidae · ocean | 2 | 现代 | 2/5 | `deathrattle-friendly-cnidarian-growth-once` | 亡语：使一个友方水螅纲随从获得一次成长 | 2 | A | 海蜇是根口水母目的食用水母，中国沿海自明代已有捕捞加工记录。 |
| `wood-ear` | 木耳 | 真菌界 | auriculariaceae · fungus | 2 | 原始 | 3/4 | `deathrattle-all-friendly-plus-0-plus-1` | 亡语：使你的所有随从获得 +0/+1 | 2 | A | 木耳生长在阔叶树腐木上，是中国传统的食用与药用真菌。 |
| `bacillus-subtilis` | 枯草芽孢杆菌 | 细菌域 | bacillaceae · bacteria | 2 | 原始 | 3/3 | `battlecry-friendly-growth-once` | 战吼：使一个友方随从获得一次成长 | 2 | A | 枯草芽孢杆菌能形成耐热芽孢，是微生物学研究的模式菌之一。 |
| `measles-virus` | 麻疹病毒 | 病毒 | paramyxoviridae · virus | 2 | 原始 | 3/3 | `combatstart-random-enemy-contagion-1` | 战斗开始时，使一个随机敌方随从获得 1 层传染 | 2 | A | 麻疹病毒通过飞沫传播，1954 年由恩德斯与皮布尔斯分离培养。 |
| `heliozoan` | 太阳虫 | 原生生物界 | actinophryidae · protozoa | 2 | 史前 | 2/4 | `aura-other-protozoa-plus-1-attack` | 你的其他原生生物界随从获得 +1 攻击力 | 2 | B | 太阳虫的轴足向四周放射，形似太阳，用于捕食与浮游。 |
| `abacus` | 算盘 | 机械 | automation · computer | 2 | 古典 | 2/4 | `buy-gain-1-gold` | 购买时：获得 1 金币 | 2 | C | 算盘以珠记数，是中国古代广泛使用的计算工具。 |
| `lyrebird` | 琴鸟 | 鸟纲 | menuridae · bird | 3 | 现代 | 4/5 | `battlecry-friendly-bird-growth-twice` | 战吼：使一个友方鸟纲随从获得两次成长 | 3 | B | 琴鸟的尾羽展开如竖琴，能模仿数十种鸟叫与机械声。 |
| `cicada` | 蝉 | 昆虫纲 | cicadidae · insect | 3 | 古典 | 3/6 | `battlecry-friendly-insect-growth-once`；`keyword-breed-1` | 战吼：使一个友方昆虫纲随从获得一次成长；繁衍 1 | 4 | B | 蝉的若虫在地下生活多年，成虫羽化后鸣叫求偶。 |
| `silkworm` | 蚕 | 昆虫纲 | bombycidae · insect | 3 | 古典 | 3/6 | `deathrattle-summon-1-moth` | 亡语：召唤一个 1/1 的蚕蛾 | 4 | B | 家蚕由野桑蚕驯化而来，中国在公元前数千年已掌握缫丝。 |
| `electric-eel` | 电鳗 | 鱼纲 | gymnotidae · fish | 3 | 现代 | 4/4 | `combatstart-random-enemy-damage-2`；`keyword-windfury` | 战斗开始时，随机对一个敌方随从造成 2 点伤害；连击 | 5 | A | 电鳗能产生数百伏的电击，用于捕食与防御；它其实是刀鱼近亲。 |
| `flounder` | 比目鱼 | 鱼纲 | pleuronectidae · fish | 3 | 古典 | 3/6 | `battlecry-friendly-fish-growth-twice`；`keyword-taunt` | 战吼：使一个友方鱼纲随从获得一次成长；嘲讽 | 2.5 | B | 比目鱼幼体两眼分列两侧，变态后一只眼移到同一侧并侧卧海底。 |
| `seahorse` | 海马 | 鱼纲 | syngnathidae · fish | 3 | 古典 | 3/6 | `battlecry-friendly-fish-growth-once`；`keyword-breed-1` | 战吼：使一个友方鱼纲随从获得一次成长；繁衍 1 | 4 | B | 海马由雄性育儿袋孵化幼体，是唯一雄性怀孕的鱼类。 |
| `cross-octopus` | 十字蛸 | 头足纲 | octopodidae · mollusk | 3 | 现代 | 3/5 | `combatstart-random-enemy-minus-2-attack`；`keyword-stealth` | 战斗开始时，使一个随机敌方随从获得 -2 攻击力；潜行 | 5 | A | 十字蛸生活在地中海深水，腕间有十字形膜。 |
| `sea-fan` | 海扇 | 水螅纲 | gorgoniidae · ocean | 3 | 史前 | 3/6 | `aura-other-cnidarian-plus-0-plus-1`；`keyword-armor-1` | 你的其他水螅纲随从获得 +0/+1；减伤 1 | 4 | B | 海扇是柳珊瑚目的一类，群体呈扇形网状，常见于热带礁区。 |
| `ganoderma` | 灵芝 | 真菌界 | ganodermataceae · fungus | 3 | 古典 | 4/5 | `battlecry-friendly-fungus-growth-twice`；`keyword-taunt` | 战吼：使一个友方真菌随从获得两次成长；嘲讽 | 3.5 | B | 灵芝是多孔菌科真菌，在中国传统医药中被称为「仙草」。 |
| `cyanobacteria` | 蓝细菌 | 细菌域 | cyanobacteriaceae · bacteria | 3 | 史前 | 3/6 | `aura-other-bacteria-plus-0-plus-1`；`keyword-armor-1` | 你的其他细菌域随从获得 +0/+1；减伤 1 | 4 | B | 蓝细菌通过光合作用释放氧气，约 24 亿年前改变了地球大气。 |
| `noctiluca` | 夜光虫 | 原生生物界 | noctilucaceae · protozoa | 3 | 现代 | 3/5 | `combatstart-random-enemy-contagion-1`；`keyword-breed-1` | 战斗开始时，使一个随机敌方随从获得 1 层传染；繁衍 1 | 4 | A | 夜光虫受扰动时发出蓝光，形成「蓝眼泪」海面荧光。 |
| `tmv` | 烟草花叶病毒 | 病毒 | virgaviridae · virus | 3 | 现代 | 4/4 | `combatstart-random-enemy-minus-2-attack`；`keyword-divine-shield` | 战斗开始时，使一个随机敌方随从获得 -2 攻击力；护盾 | 2 | A | 烟草花叶病毒 1892 年由伊万诺夫斯基发现，是首个被发现的病毒。 |
| `chameleon` | 变色龙 | 爬行纲 | chamaeleonidae · reptile | 3 | 古典 | 4/5 | `swap-stats-friendly`；`keyword-armor-1` | 战吼：使一个友方随从的攻击力与生命值互换；减伤 1 | 4 | C | 变色龙以皮肤色素细胞改变体色，并用独立转动的双眼观察四周。 |
| `microscope` | 显微镜 | 机械 | automation · invention | 3 | 工业 | 3/6 | `trigger-discover-tribe-minion` | 战吼：发现一张本局玩法的随从 | 4 | A | 列文虎克在 17 世纪用自制单式显微镜首次观察到微生物。 |
| `water-lily` | 睡莲 | 植物界 | nymphaeaceae · aquatic-plant | 3 | 古典 | 3/6 | `battlecry-friendly-plant-growth-once`；`keyword-sow` | 战吼：使一个友方植物随从获得一次成长；播种 | 4 | A | 睡莲的叶片浮于水面，花朵随光照开合，广泛分布于静水环境。 |
| `arctic-tern` | 北极燕鸥 | 鸟纲 | sternidae · bird | 4 | 现代 | 6/7 | `freeze-enemy-leftmost`；`keyword-taunt` | 战斗开始时，冻结一个敌方随从；嘲讽 | 3.5 | C | 北极燕鸥每年在南北极之间往返迁徙，是已知迁徙距离最长的动物。 |
| `stick-insect` | 竹节虫 | 昆虫纲 | phasmatidae · insect | 4 | 史前 | 5/7 | `chance-self-attack-2`；`keyword-breed-1` | 战斗开始时，有 50% 概率获得 +2 攻击力；繁衍 1 | 4 | C | 竹节虫以体形拟态树枝，静止时极难被发现。 |
| `archerfish` | 射水鱼 | 鱼纲 | toxotidae · fish | 4 | 现代 | 5/7 | `combat-random-enemy-damage-3`；`keyword-ignore-taunt` | 战斗开始时，随机对一个敌方随从造成 3 点伤害；无视嘲讽 | 5 | A | 射水鱼从口中射出水柱击落水面上的昆虫。 |
| `sea-pen` | 海笔 | 水螅纲 | pennatulidae · ocean | 4 | 史前 | 4/8 | `aura-other-cnidarian-plus-0-plus-2`；`keyword-armor-1` | 你的其他水螅纲随从获得 +0/+2；减伤 1 | 5 | B | 海笔是八放珊瑚的一类，群体形如羽毛笔，插入沙质海底。 |
| `mucor` | 毛霉 | 真菌界 | mucoraceae · fungus | 4 | 原始 | 5/7 | `deathrattle-friendly-fungus-growth-once`；`keyword-breed-1` | 亡语：使一个友方真菌随从获得一次成长；繁衍 1 | 4 | B | 毛霉属真菌的孢子囊柄直立，常用于豆制品发酵。 |
| `azotobacter` | 固氮菌 | 细菌域 | pseudomonadaceae · bacteria | 4 | 现代 | 5/7 | `battlecry-friendly-bacteria-growth-once`；`keyword-armor-1` | 战吼：使一个友方细菌域随从获得一次成长；减伤 1 | 4 | B | 固氮菌把大气中的氮转化为氨，是土壤氮循环的重要环节。 |
| `h-pylori` | 幽门螺杆菌 | 细菌域 | helicobacteraceae · bacteria | 4 | 现代 | 4/8 | `combatstart-random-enemy-contagion-2`；`keyword-armor-1` | 战斗开始时，使一个随机敌方随从获得 2 层传染；减伤 1 | 5 | A | 幽门螺杆菌能在胃酸中存活，1982 年由马歇尔与沃伦分离。 |
| `difflugia` | 砂壳虫 | 原生生物界 | difflugiidae · protozoa | 4 | 史前 | 4/8 | `aura-other-protozoa-plus-1-attack`；`keyword-breed-1` | 你的其他原生生物界随从获得 +1 攻击力；繁衍 1 | 4 | B | 砂壳虫用砂粒黏合外壳，常见于淡水沉积物中。 |
| `herpes-virus` | 疱疹病毒 | 病毒 | herpesviridae · virus | 4 | 史前 | 5/7 | `deathrattle-all-enemies-minus-1-health`；`keyword-breed-1` | 亡语：使敌方所有随从获得 -1 生命值；繁衍 1 | 2 | A | 疱疹病毒可长期潜伏在宿主神经节内，应激时复发。 |
| `marine-iguana` | 海鬣蜥 | 爬行纲 | iguanidae · reptile | 4 | 现代 | 5/8 | `trigger-friendly-taunt-until-turn-end`；`keyword-armor-1` | 战吼：使一个友方随从获得嘲讽直到回合结束；减伤 1 | 2.5 | A | 海鬣蜥是唯一适应海洋生活的蜥蜴，在加拉帕戈斯群岛潜水啃食海藻。 |
| `venus-flytrap` | 捕蝇草 | 植物界 | droseraceae · flower | 4 | 现代 | 5/7 | `devour-2`；`keyword-taunt` | 吞噬：战斗开始时，消灭一个攻击力不高于 2 的敌方随从，在本场战斗内永久获得其攻击力与生命值；嘲讽 | 4.5 | A | 捕蝇草的叶片特化为捕虫夹，触碰感觉毛两次即闭合。 |
| `telescope` | 望远镜 | 机械 | automation · invention | 4 | 工业 | 4/9 | `trigger-discover-tribe-minion` | 战吼：发现一张本局玩法的随从 | 4 | A | 伽利略 1609 年自制望远镜观测天体，发现木星卫星与月球环形山。 |
| `horned-lizard` | 角蜥 | 爬行纲 | phrynosomatidae · reptile | 4 | 现代 | 5/7 | `trigger-friendly-single-plus-0-plus-2`；`keyword-armor-1` | 战吼：使一个友方随从获得 +0/+2；减伤 1 | 4 | A | 角蜥以头部棘刺与体色拟态沙地，受威胁时从眼角喷血。 |
| `dandelion` | 蒲公英 | 植物界 | asteraceae · flower | 4 | 现代 | 4/8 | `battlecry-friendly-plant-growth-once`；`keyword-sow` | 战吼：使一个友方植物随从获得一次成长；播种 | 4 | A | 蒲公英的冠毛果实借风传播，种子可飞行数百米。 |
| `peregrine-falcon` | 游隼 | 鸟纲 | falconidae · bird | 5 | 现代 | 8/9 | `trigger-bird-single-windfury-this-turn`；`keyword-ignore-taunt` | 战吼：使一个友方鸟纲随从获得连击直到回合结束；无视嘲讽 | 5 | A | 游隼俯冲时速可超过 300 公里，是速度最快的动物。 |
| `parasitoid-wasp` | 寄生蜂 | 昆虫纲 | ichneumonidae · insect | 5 | 原始 | 7/10 | `battlecry-friendly-insect-growth-twice`；`keyword-breed-1` | 战吼：使一个友方昆虫纲随从获得两次成长；繁衍 1 | 5 | B | 寄生蜂把卵产在宿主昆虫体内，幼虫孵化后取食宿主；达尔文曾为此困扰。 |
| `sailfish` | 旗鱼 | 鱼纲 | istiophoridae · fish | 5 | 现代 | 8/8 | `combat-optimal-attack`；`keyword-windfury` | 战吼：使一个友方随从获得无视嘲讽直到回合结束；连击 | 6 | A | 旗鱼背鳍展开如帆，是游速最快的鱼类之一。 |
| `lungfish` | 肺鱼 | 鱼纲 | protopteridae · fish | 5 | 工业 | 6/12 | `battlecry-friendly-fish-growth-twice`；`keyword-taunt` | 战吼：使一个友方鱼纲随从获得两次成长；嘲讽 | 3.5 | B | 肺鱼除鳃外还有肺，旱季可钻入泥中休眠数月。 |
| `trichoderma` | 木霉 | 真菌界 | hypocreaceae · fungus | 5 | 现代 | 7/11 | `deathrattle-all-friendly-plus-0-plus-1`；`keyword-contagion` | 亡语：使你的所有随从获得 +0/+1；传染 | 4 | A | 木霉属真菌生长迅速，被广泛用于生物防治与纤维素酶生产。 |
| `actinomyces` | 放线菌 | 细菌域 | actinomycetaceae · bacteria | 5 | 工业 | 7/10 | `battlecry-friendly-bacteria-growth-twice`；`keyword-armor-1` | 战吼：使一个友方细菌域随从获得两次成长；减伤 1 | 5 | B | 放线菌以分枝菌丝生长，是链霉素等抗生素的主要来源。 |
| `entamoeba` | 内阿米巴 | 原生生物界 | entamoebidae · protozoa | 5 | 原始 | 7/10 | `combatstart-all-enemies-contagion-1`；`keyword-breed-1` | 战斗开始时，使敌方所有随从获得 1 层传染；繁衍 1 | 5 | A | 内阿米巴属的溶组织内阿米巴可引发阿米巴痢疾。 |
| `hbv` | 乙型肝炎病毒 | 病毒 | hepadnaviridae · virus | 5 | 现代 | 6/10 | `combatstart-all-enemies-contagion-2`；`keyword-breed-1` | 战斗开始时，使敌方所有随从获得 2 层传染；繁衍 1 | 6 | A | 乙型肝炎病毒 1965 年由布隆伯格发现，其表面抗原成为乙肝疫苗基础。 |
| `cuvier` | 居维叶 | 头足纲 | octopodidae · mollusk | 6 | 工业 | 9/10 | `battlecry-friendly-ceph-growth-thrice`；`keyword-stealth` | 战吼：使一个友方头足纲随从获得三次成长；潜行 | 7 | B | 居维叶 1795 年起在巴黎自然史博物馆比较解剖软体动物，建立古生物学。 |
| `spirula` | 旋壳鹦鹉螺 | 头足纲 | spirulidae · mollusk | 6 | 史前 | 9/12 | `battlecry-friendly-ceph-growth-twice`；`keyword-breed-1` | 战吼：使一个友方头足纲随从获得两次成长；繁衍 1 | 5 | B | 旋壳鹦鹉螺的内壳卷成平面螺旋，是少数带内壳的头足类。 |
| `lion-mane-jellyfish` | 霞水母 | 水螅纲 | cyaneidae · ocean | 6 | 现代 | 8/12 | `revive-2`；`keyword-armor-1` | 复苏：以 +2/+2 复活一次；减伤 1 | 6 | A | 霞水母属的狮鬃水母触手可长达 30 米以上，是已知最长的动物之一。 |
| `hericium` | 猴头菇 | 真菌界 | hericiaceae · fungus | 6 | 古典 | 8/13 | `deathrattle-friendly-fungus-growth-twice`；`keyword-contagion` | 亡语：使一个友方真菌随从获得两次成长；传染 | 5 | B | 猴头菇因子实体形似猴头而得名，是中国传统食用菌。 |
| `leishmania` | 利什曼原虫 | 原生生物界 | trypanosomatidae · protozoa | 6 | 现代 | 8/13 | `combatstart-all-enemies-contagion-1`；`keyword-breed-1` | 战斗开始时，使敌方所有随从获得 1 层传染；繁衍 1 | 5 | A | 利什曼原虫由白蛉传播，引起利什曼病；1903 年由利什曼与多诺万分别发现。 |
| `polio-virus` | 脊髓灰质炎病毒 | 病毒 | picornaviridae · virus | 6 | 现代 | 8/12 | `combatstart-all-enemies-contagion-2`；`keyword-breed-1` | 战斗开始时，使敌方所有随从获得 2 层传染；繁衍 1 | 6 | A | 脊髓灰质炎病毒 1908 年由兰德施泰纳与波普尔分离，索尔克与萨宾先后研制疫苗。 |
| `tuatara` | 楔齿蜥 | 爬行纲 | sphenodontidae · reptile | 6 | 史前 | 9/13 | `revive-1`；`keyword-breed-1` | 复苏：以 +1/+1 复活一次；繁衍 1 | 4 | A | 楔齿蜥是喙头目唯一现存成员，仅分布于新西兰离岛。 |
| `prion` | 朊病毒 | 病毒 | prion-incertae-sedis · virus | 7 | 现代 | 12/15 | `combatstart-all-enemies-contagion-2`；`keyword-divine-shield` | 战斗开始时，使敌方所有随从获得 2 层传染；护盾 | 6 | A | 朊病毒是错误折叠的蛋白质，能诱导正常蛋白同样折叠，引起克雅氏病与疯牛病。 |
| `redwood` | 红杉 | 植物界 | cupressaceae · tree | 7 | 现代 | 11/16 | `aura-plant-plus-2-plus-2`；`keyword-sow` | 你的植物随从获得 +2/+2；播种 | 6 | A | 加州红杉可高逾百米、寿命逾千年，是现存最高的植物。 |
| `rocket` | 火箭 | 机械 | automation · spaceflight | 7 | 现代 | 14/14 | `battlecry-friendly-mech-growth-twice`；`keyword-assemble-1` | 战吼：使一个友方机械随从获得两次成长；装配 1 | 5 | B | 齐奥尔科夫斯基 1903 年提出火箭方程，为现代航天奠定理论基础。 |
| `giant-water-lily` | 王莲 | 植物界 | nymphaeaceae · aquatic-plant | 7 | 现代 | 12/16 | `aura-plant-plus-1-plus-1`；`keyword-sow` | 你的植物随从获得 +1/+1；播种 | 5 | A | 王莲的叶片直径可超过 3 米，能承载数十公斤重量。 |

#### 7.6 扩展批次：效果牌（22）

| id | 名称 | 玩法 | 分类 | 星级 | 时代 | 费用 | 技能 | 效果 | 折算 | 支持 | 出处 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `courtship-display` | 求偶展示 | 鸟纲 | biology | 1 | 现代 | 0 | `trigger-friendly-taunt-until-turn-end` | 使一个友方随从获得嘲讽直到回合结束 | 1 | A | 极乐鸟与园丁鸟以复杂的求偶场与舞蹈吸引配偶，达尔文以此讨论性选择。 |
| `phototaxis` | 趋光性 | 昆虫纲 | biology | 1 | 现代 | 0 | `trigger-friendly-plus-2-attack` | 使一个友方随从获得 +2 攻击力 | 2 | A | 许多夜行昆虫依靠月光与星光定向，人造光源会扰乱其导航。 |
| `germination` | 萌芽 | 植物界 | biology | 1 | 史前 | 0 | `trigger-friendly-single-growth-once` | 使一个友方随从获得一次成长 | 2 | A | 种子吸水后胚根突破种皮，是植物生命周期的起点。 |
| `spore-rain` | 孢子雨 | 真菌界 | biology | 1 | 史前 | 1 | `trigger-all-friendly-plus-0-plus-1` | 使你的所有随从获得 +0/+1 | 2 | A | 真菌通过弹射孢子繁殖，一次释放可达数十亿枚。 |
| `migration-instinct` | 迁徙本能 | 鸟纲 | biology | 1 | 现代 | 1 | `trigger-friendly-plus-2-attack` | 使一个友方随从获得 +2 攻击力 | 2 | A | 候鸟依靠地磁、太阳与星象导航，完成数千公里的季节性迁徙。 |
| `commutativity` | 交换律 | 机械 | mathematics | 1 | 古典 | 1 | `swap-stats-friendly` | 使一个友方随从的攻击力与生命值互换 | 2 | C | 加法与乘法的交换律是算术的基本性质，欧几里得《几何原本》已使用。 |
| `capillary-action` | 毛细现象 | 植物界 | physics | 1 | 工业 | 1 | `trigger-friendly-single-plus-0-plus-2` | 使一个友方随从获得 +0/+2 | 2 | A | 水在细管中因表面张力上升，是植物根系吸水与纸张吸墨的原理。 |
| `humus` | 腐殖质 | 真菌界 | earth-science | 1 | 原始 | 1 | `trigger-friendly-single-plus-0-plus-2` | 使一个友方随从获得 +0/+2 | 2 | A | 腐殖质是动植物残体经微生物分解形成的稳定有机质，决定土壤肥力。 |
| `convergent-evolution` | 趋同演化 | 鱼纲 | biology | 2 | 现代 | 1 | `trigger-friendly-single-plus-0-plus-2` | 使一个友方随从获得 +0/+2 | 2 | A | 鲨鱼与海豚分属软骨鱼与哺乳类，却演化出相似的流线体型。 |
| `symbiosis` | 共生 | 真菌界 | biology | 2 | 现代 | 1 | `trigger-friendly-growth-twice` | 使一个友方随从获得两次成长 | 3 | A | 地衣是真菌与藻类或蓝细菌的共生体；菌根让真菌与植物交换水分与养分。 |
| `cold-resistance` | 耐寒 | 植物界 | biology | 2 | 工业 | 1 | `trigger-friendly-single-armor-2-this-turn` | 使一个友方随从获得减伤 2 直到回合结束 | 2 | A | 植物通过降低细胞冰点与积累抗冻蛋白抵御低温。 |
| `mimicry-learning` | 拟态学习 | 头足纲 | biology | 3 | 现代 | 2 | `mimic-1` | 战吼：复制一个友方随从的当前攻击力与生命值直到本回合结束 | 4 | A | 章鱼能迅速改变体色与形态模仿环境，甚至模仿其他物种。 |
| `symmetry` | 对称性 | 爬行纲 | physics | 4 | 现代 | 2 | `swap-stats-friendly-plus-1` | 使一个友方随从的攻击力与生命值互换，然后获得 +1/+1 | 3 | C | 诺特定理把物理定律的对称性与守恒量联系起来。 |
| `spiral-of-silence` | 沉默的螺旋 | 哺乳纲 | psychology | 4 | 现代 | 2 | `silence-enemy-single` | 使一个敌方随从失去全部技能与关键词，直到本场战斗结束 | 2 | C | 诺依曼 1974 年提出「沉默的螺旋」：少数意见因担心孤立而沉默，反而强化主流意见。 |
| `ice-age` | 冰期 | 爬行纲 | earth-science | 4 | 史前 | 2 | `freeze-enemy-leftmost` | 战斗开始时，冻结敌方最左侧随从 | 2 | C | 第四纪冰期反复出现，冰盖推进改变了物种分布与人类迁徙路线。 |
| `probability-cloud` | 概率云 | 机械 | physics | 5 | 现代 | 3 | `chance-growth-all-50` | 战斗开始时，有 50% 概率使你的所有随从获得一次成长 | 3 | C | 量子力学以波函数描述电子出现概率，形成「概率云」图像。 |
| `relative-deprivation` | 相对剥夺 | 哺乳纲 | economics | 5 | 现代 | 3 | `enemy-all-minus-2-attack` | 战斗开始时，使敌方所有随从获得 -2 攻击力 | 3 | B | 相对剥夺感指个体与他人比较后感到自身处境不利，是社会学与经济学概念。 |
| `immune-memory` | 免疫记忆 | 细菌域 | medicine | 5 | 现代 | 3 | `vaccine-cleanse` | 清除你的所有随从的传染；本场战斗内不再获得传染 | 3 | C | 获得性免疫通过记忆细胞在再次感染时快速应答，是疫苗的理论基础。 |
| `industrial-upgrade` | 产业升级 | 机械 | economics | 5 | 工业 | 2 | `gain-gold-2` | 获得 2 金币 | 2 | C | 产业升级指生产要素从低附加值环节转向高附加值环节，是发展经济学的核心议题。 |
| `observer-effect` | 观测者效应 | 机械 | physics | 6 | 现代 | 4 | `chance-enemy-single-minus-3-75` | 战斗开始时，有 75% 概率使一个敌方随从获得 -3 攻击力 | 3 | C | 观测者效应指测量行为影响被测量系统，在量子力学与心理学中含义不同。 |
| `cambrian-explosion` | 寒武纪大爆发 | 水螅纲 | biology | 6 | 史前 | 4 | `summon-two-trilobites` | 召唤两个 3/3 的三叶虫 | 6 | A | 约 5.4 亿年前的寒武纪，多门类动物化石在短时间内大量出现。 |
| `great-filter` | 大过滤器 | 机械 | astronomy | 7 | 未来 | 4 | `silence-all-enemy` | 使敌方所有随从失去全部技能与关键词，直到本场战斗结束 | 4 | C | 「大过滤器」是解释费米悖论的假说：智慧文明演化途中存在极难跨越的障碍。 |

#### 7.7 扩展批次：英雄卡（6）

英雄卡与同 id 随从卡分属两池；下表只列英雄侧技能，随从侧见 §7.5 与现有卡池。

| id | 名称 | 玩法 | 主动技能 | 被动技能 | 简介 |
| --- | --- | --- | --- | --- | --- | --- |
| `heisenberg` | 海森堡 | 哺乳纲 | `uncertainty` | — | 德国物理学家，1927 年提出不确定性原理，指出位置与动量不能同时被精确测定，1932 年获诺贝尔物理学奖。 |
| `schrodinger` | 薛定谔 | 爬行纲 | `quantum-observation` | — | 奥地利物理学家，1926 年提出波动方程，1935 年以「猫」的思想实验讨论量子叠加与测量问题。 |
| `wright-brothers` | 莱特兄弟 | 机械 | — | `aura-mech-optimal-attack` | 美国航空先驱，1903 年在基蒂霍克完成首次有动力、可操控的持续飞行，开启现代航空。 |
| `fleming` | 弗莱明 | 病毒 | `antibiosis` | — | 苏格兰细菌学家，1928 年发现青霉素，1945 年与弗洛里、钱恩共同获得诺贝尔生理学或医学奖。 |
| `lavoisier` | 拉瓦锡 | 爬行纲 | — | `aura-all-plus-0-plus-1` | 法国化学家，1777 年提出燃烧的氧化学说，建立质量守恒定律，被誉为现代化学之父。 |
| `gutenberg` | 古腾堡 | 机械 | `printing-press` | — | 德意志发明家，15 世纪中叶发明金属活字印刷术，使书籍大规模复制成为可能。 |

#### 7.8 扩展批次：衍生物（6）

| id | 名称 | 玩法 | 分类 | 星级 | 攻/血 | 出处 |
| --- | --- | --- | --- | --- | --- | --- |
| `firefly-token` | 萤火 | 昆虫纲 | lampyridae · insect | 1 | 1/1 | 萤火虫的成虫形态，用于召唤衍生物。 |
| `moth-token` | 蚕蛾 | 昆虫纲 | bombycidae · insect | 1 | 1/1 | 家蚕的成虫形态，翅退化、不再取食。 |
| `hypha-token` | 菌丝 | 真菌界 | mucoraceae · fungus | 1 | 1/1 | 真菌的营养体，由分枝菌丝构成。 |
| `larva-token` | 幼虫 | 昆虫纲 | cicadidae · insect | 1 | 1/1 | 蝉的若虫阶段，在土中吸食植物根部汁液。 |
| `hydra-token` | 水螅幼体 | 水螅纲 | hydridae · ocean | 1 | 1/1 | 水螅的出芽生殖个体。 |
| `trilobite-token` | 三叶虫 | 昆虫纲 | trilobitidae · animal | 2 | 3/3 | 古生代海洋节肢动物，寒武纪大爆发的代表化石。 |

#### 7.9 诺奖效果牌（32）

本批只做效果牌，不改随从池，也不新增第 10 项机制。选题来自本库存「诺奖设计角度」部分的机制原型与可结算概念，学科集中在化学（12）、生理学或医学（10）、物理学（10）；星级按 2★ 4、3★ 8、4★ 7、5★ 6、6★ 4、7★ 3 配平，效果牌占同星级进池卡的比例全部低于 30%（见 §8.2）。`分类` 列写学科根节点，`支持` 见 §4.3，A 类 13 张、B 类 12 张、C 类 7 张。

本节是精选机制设计；「每位得主至少一张卡」的全量覆盖见本库存「诺奖全量名册」部分（1028 位得主/机构，一人一卡）。精选版与名册版不互相替代：前者给专属机制，后者保证覆盖。

本批有两个刻意的取舍：其一，31 张为现代、1 张为工业（哈伯-博施法），因为诺奖对应的是 20 世纪以来的科学建制，早期时代留给「前诺奖科学概念」批次；其二，机械玩法占 15 张，用来把物理与化学概念落到已有机械体系里，其余按医学落到哺乳纲、微生物落到细菌域/病毒、水螅纲落到绿色荧光蛋白。若后续要把诺奖概念补到早期时代，应另开批次，不从本批改时代。

本库存「诺奖设计角度」部分 §十一 的首批建议里，换位、合并、储存、技能编辑、升星替换与共享池都需要新子系统；本批不实现这些子系统，改用已有原语做近似：转座子用属性互换，点击化学用复制加身材，MOF 用单体 +3/+3，CRISPR 用沉默，mRNA 用群体护盾。这样 32 张都能挂在 §5 的 9 项机制上，代价是概念只取「可结算的那一面」。落地时若决定做原机制，应另开模板批次，不在本批改卡。

| id | 名称 | 玩法 | 分类 | 星级 | 时代 | 费用 | 技能 | 效果 | 折算 | 支持 | 出处 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `pavlov-reflex` | 条件反射 | 哺乳纲 | medicine | 2 | 现代 | 1 | `trigger-friendly-plus-2-attack` | 使一个友方随从获得 +2 攻击力 | 2 | A | 巴甫洛夫以狗的唾液分泌实验建立经典条件反射，1904 年获诺贝尔生理学或医学奖。 |
| `isotope-tracer` | 同位素示踪 | 细菌域 | chemistry | 2 | 现代 | 1 | `trigger-friendly-single-plus-0-plus-2` | 使一个友方随从获得 +0/+2 | 2 | A | 赫维西 1923 年首次用放射性铅同位素追踪植物对铅的吸收，1943 年获诺贝尔化学奖。 |
| `blood-typing` | 血型 | 哺乳纲 | medicine | 2 | 现代 | 1 | `trigger-friendly-plus-1-plus-1` | 使一个友方随从获得 +1/+1 | 2 | A | 兰德施泰纳 1901 年发现 ABO 血型系统，1930 年获诺贝尔生理学或医学奖。 |
| `chemiosmosis` | 化学渗透 | 细菌域 | chemistry | 2 | 现代 | 1 | `trigger-all-friendly-plus-0-plus-1` | 使你的所有随从获得 +0/+1 | 2 | A | 米切尔 1961 年提出化学渗透假说，认为跨膜质子梯度驱动 ATP 合成，1978 年获诺贝尔化学奖。 |
| `pcr-amplification` | 聚合酶链式反应 | 细菌域、病毒 | chemistry | 3 | 现代 | 2 | `trigger-copy-friendly-minion` | 复制一个友方随从 | 4 | A | 穆利斯 1983 年发明 PCR，使 DNA 片段可在体外指数扩增，1993 年获诺贝尔化学奖。 |
| `crispr-silence` | 基因编辑 | 细菌域 | chemistry | 3 | 现代 | 2 | `silence-enemy-single` | 战斗开始时，使一个敌方随从失去全部技能与关键词，直到本场战斗结束 | 2 | C | 杜德纳与夏彭蒂耶 2012 年揭示 CRISPR-Cas9 的可编程切割机制，2020 年获诺贝尔化学奖。 |
| `rna-interference` | RNA 干扰 | 病毒 | medicine | 3 | 现代 | 1 | `cannot-attack-enemy-single` | 战斗开始时，使一个敌方随从本回合无法攻击 | 2 | B | 法尔与梅洛 1998 年发现双链 RNA 可特异性沉默基因表达，2006 年获诺贝尔生理学或医学奖。 |
| `transposon-swap` | 转座子 | 植物界 | medicine | 3 | 现代 | 2 | `swap-stats-friendly-plus-1` | 使一个友方随从的攻击力与生命值互换，然后获得 +1/+1 | 3 | C | 麦克林托克在玉米中发现的转座子能改变基因组位置，1983 年获诺贝尔生理学或医学奖。 |
| `apoptosis-sacrifice` | 细胞凋亡 | 哺乳纲 | medicine | 3 | 现代 | 2 | `sacrifice-all-friendly-plus-1-plus-1` | 献祭一个友方随从，使你的所有随从获得 +1/+1 | 4 | C | 布伦纳、霍尔维茨与萨尔斯顿以线虫揭示程序性细胞死亡的基因调控，2002 年获诺贝尔生理学或医学奖。 |
| `telomerase` | 端粒酶 | 哺乳纲 | medicine | 3 | 现代 | 2 | `trigger-friendly-single-reborn` | 使一个友方随从获得复生 | 3 | A | 布莱克本、格雷德与绍斯塔克发现端粒与端粒酶维持染色体末端，2009 年获诺贝尔生理学或医学奖。 |
| `gfp-marker` | 绿色荧光蛋白 | 水螅纲 | chemistry | 3 | 现代 | 2 | `trigger-friendly-single-stealth-this-turn` | 使一个友方随从获得潜行直到回合结束 | 3 | A | 下村脩从水母中分离绿色荧光蛋白，查尔菲与钱永健将其发展为活体标记，2008 年获诺贝尔化学奖。 |
| `asymmetric-catalysis` | 不对称催化 | 细菌域 | chemistry | 3 | 现代 | 2 | `stats-leftmost-plus-2-plus-2` | 使最左侧的友方随从获得 +2/+2 | 4 | B | 诺尔斯、野依良治与夏普莱斯开发手性催化剂，2001 年获诺贝尔化学奖。 |
| `alpha-fold` | AlphaFold | 机械 | chemistry | 4 | 现代 | 3 | `trigger-discover-tribe-minion` | 发现一张本局玩法的随从 | 4 | A | 哈萨比斯与江珀以 AlphaFold 预测蛋白质三维结构，2024 年获诺贝尔化学奖。 |
| `click-chemistry` | 点击化学 | 细菌域 | chemistry | 4 | 现代 | 3 | `copy-friendly-minion-plus-1-plus-1` | 复制一个友方随从，然后使一个友方随从获得 +1/+1 | 5 | B | 贝尔托齐、梅达尔与夏普莱斯发展模块化、高选择性的点击化学，2022 年获诺贝尔化学奖。 |
| `mri-imaging` | 磁共振成像 | 机械 | medicine | 4 | 现代 | 2 | `trigger-discover-tribe-minion` | 发现一张本局玩法的随从 | 4 | A | 劳特布尔与曼斯菲尔德把核磁共振发展为医学成像，2003 年获诺贝尔生理学或医学奖。 |
| `immune-checkpoint` | 免疫检查点 | 哺乳纲 | medicine | 4 | 现代 | 3 | `cleanse-friendly-single-stealth` | 清除一个友方随从的全部传染，并使其获得潜行直到回合结束 | 5 | C | 艾利森与本庶佑发现抑制免疫检查点可激活 T 细胞抗肿瘤，2018 年获诺贝尔生理学或医学奖。 |
| `ozone-hole` | 臭氧层空洞 | 机械 | chemistry | 4 | 现代 | 2 | `stats-enemy-single-minus-3` | 战斗开始时，使一个敌方随从获得 -3 攻击力 | 3 | B | 莫利纳与罗兰揭示氯氟烃破坏平流层臭氧，克鲁岑阐明其大气化学，1995 年获诺贝尔化学奖。 |
| `super-resolution` | 超分辨显微 | 机械 | chemistry | 4 | 现代 | 2 | `stats-friendly-single-plus-3-attack` | 使一个友方随从获得 +3 攻击力 | 3 | B | 黑尔、贝齐格与莫纳发展超分辨荧光显微技术，突破光学衍射极限，2014 年获诺贝尔化学奖。 |
| `haber-bosch` | 哈伯-博施法 | 细菌域 | chemistry | 4 | 工业 | 3 | `trigger-all-friendly-plus-1-plus-1` | 使你的所有随从获得 +1/+1 | 4 | A | 哈伯 1909 年实现合成氨，博施将其工业化，二人分获 1918 与 1931 年诺贝尔化学奖。 |
| `stem-cell` | 诱导多能干细胞 | 哺乳纲 | medicine | 5 | 现代 | 2 | `trigger-friendly-single-reborn` | 使一个友方随从获得复生 | 3 | A | 山中伸弥 2006 年把成体细胞重编程为诱导多能干细胞，2012 年获诺贝尔生理学或医学奖。 |
| `holography` | 全息摄影 | 机械 | physics | 5 | 现代 | 3 | `copy-friendly-minion-plus-1-plus-1` | 复制一个友方随从，然后使一个友方随从获得 +1/+1 | 5 | B | 加博 1947 年提出全息照相原理，1971 年获诺贝尔物理学奖。 |
| `superconductivity` | 超导 | 机械 | physics | 5 | 现代 | 3 | `enemy-all-minus-2-attack` | 战斗开始时，使敌方所有随从获得 -2 攻击力 | 3 | B | 巴丁、库珀与施里弗提出 BCS 超导理论，1972 年获诺贝尔物理学奖。 |
| `laser-cooling` | 激光冷却 | 机械 | physics | 5 | 现代 | 2 | `freeze-enemy-single` | 战斗开始时，冻结一个敌方随从 | 3 | C | 朱棣文、科昂-塔诺季与菲利普斯用激光把原子冷却到微开尔文量级，1997 年获诺贝尔物理学奖。 |
| `atomic-clock` | 原子钟 | 机械 | physics | 5 | 现代 | 3 | `trigger-machine-all-plus-1-plus-1` | 使你的机械随从获得 +1/+1 | 4 | A | 拉姆齐发展分离振荡场方法，为铯原子钟奠定基础，1989 年获诺贝尔物理学奖。 |
| `electron-microscope` | 电子显微镜 | 机械 | physics | 5 | 现代 | 2 | `trigger-discover-tribe-minion` | 发现一张本局玩法的随从 | 4 | A | 鲁斯卡 1931 年建成第一台电子显微镜，1986 年获诺贝尔物理学奖。 |
| `mrna-vaccine` | mRNA 疫苗 | 哺乳纲 | medicine | 6 | 现代 | 4 | `shield-all-friendly` | 使你的所有随从在本场战斗内获得护盾 | 6 | B | 卡里科与魏斯曼发现核苷修饰可降低 mRNA 免疫原性并提高蛋白表达，2023 年获诺贝尔生理学或医学奖。 |
| `dark-matter` | 暗物质 | 机械 | physics | 6 | 现代 | 3 | `stats-enemy-all-minus-3-attack` | 战斗开始时，使敌方所有随从获得 -3 攻击力 | 4 | B | 皮布尔斯建立宇宙微波背景与结构形成的理论框架，2019 年获诺贝尔物理学奖。 |
| `gravitational-waves` | 引力波天文学 | 机械 | physics | 6 | 现代 | 4 | `freeze-enemy-all` | 战斗开始时，冻结敌方所有随从 | 6 | C | 韦斯、巴里什与索恩推动 LIGO 建成并首次直接探测引力波，2017 年获诺贝尔物理学奖。 |
| `mof` | 金属有机框架 | 机械 | chemistry | 6 | 现代 | 4 | `stats-friendly-single-plus-3-plus-3` | 使一个友方随从获得 +3/+3 | 6 | B | 北川进、罗布森与亚吉开创金属有机框架，2025 年获诺贝尔化学奖。 |
| `higgs-field` | 希格斯场 | 机械 | physics | 7 | 现代 | 4 | `stats-all-friendly-plus-2-plus-2` | 使你的所有随从获得 +2/+2 | 6 | B | 希格斯与恩格勒 1964 年提出赋予基本粒子质量的机制，2013 年获诺贝尔物理学奖。 |
| `neutrino-oscillation` | 中微子振荡 | 机械 | physics | 7 | 现代 | 4 | `ramp-all-friendly-growth` | 使你的所有随从获得等同于本局回合数除以 2 的成长（向上取整，至多 2 次） | 6 | C | 梶田隆章与麦克唐纳发现中微子振荡，证明中微子有质量，2015 年获诺贝尔物理学奖。 |
| `cosmic-microwave` | 宇宙微波背景 | 机械 | physics | 7 | 现代 | 4 | `stats-all-friendly-plus-0-plus-3` | 使你的所有随从获得 +0/+3 | 6 | B | 彭齐亚斯与威尔逊 1964 年探测到宇宙微波背景辐射，1978 年获诺贝尔物理学奖。 |

### 8. 数值与分布自检

#### 8.1 单卡强度

折算口径：战吼/亡语小效果 2 点，召唤/复制类 4 点，嘲讽 0.5 点，装配 1 为 2 点，概率按期望值向上取整；补关键词时按关键词折算（嘲讽 0.5；繁衍 1/传染/无视嘲讽/护盾/减伤 1 为 2；连击/潜行 3）等量扣减身材，逐卡见 §8.3。

| 卡 | 星级 | 身材 | 折算 | 合计 | 上限 | 结论 |
| --- | --- | --- | --- | --- | --- | --- |
| 珍·古德 | 5 | 0/17 | 4（光环 +1/+1） | 21 | 22 | 通过 |
| 白胡子大卫 | 3 | 4/5 | 4（召唤两个 2/2） | 13 | 13 | 通过 |
| 猩猩阿兹 | 4 | 5/6 | 2 + 0.5 | 13.5 | 17 | 通过 |
| 猩猩因达 | 3 | 3/5 | 2 + 0.5 | 10.5 | 13 | 通过 |
| 泰特拉一世 | 3 | 3/6 | 4（召唤一个 2/2） | 13 | 13 | 通过 |
| 布里切特三世 | 4 | 5/7 | 2 + 2 | 16 | 17 | 通过 |
| 奥利佛 | 3 | 4/4 | 2 + 0.5 | 10.5 | 13 | 通过 |
| 海利恩 | 2 | 4/3 | 2（经济） | 9 | 10 | 通过 |
| 薛定谔的猫 | 3 | 4/6 | 2（期望 1.5） | 12 | 13 | 通过 |
| 费波那契 | 6 | 0/21 | 4（递增成长） | 25 | 26 | 通过 |
| 普朗克 | 5 | 0/17 | 2 | 19 | 22 | 通过 |
| 麦克斯韦 | 5 | 6/11 | 2 | 19 | 22 | 通过 |
| 詹纳 | 4 | 0/13 | 2 | 15 | 17 | 通过 |
| 李斯特 | 4 | 4/9 | 2 | 15 | 17 | 通过 |
| 列文虎克 | 3 | 3/6 | 2 | 11 | 13 | 通过 |
| 梵高 | 5 | 0/17 | 3 | 20 | 22 | 通过 |
| 凯恩斯 | 5 | 0/17 | 2（经济） | 19 | 22 | 通过 |
| 贝尔 | 6 | 9/13 | 4 | 26 | 26 | 通过 |
| 诺贝尔 | 7 | 11/17 | 4（光环 +2/+2） | 32 | 33 | 通过 |
| 手工耿（随从） | 4 | 6/7 | 2 + 0.5 | 15.5 | 17 | 通过 |

#### 8.2 卡池分布

| 检查项 | 结果 |
| --- | --- |
| 同「玩法 + 星级 + 攻击力 + 生命值」> 2 | 未发现。合并后恰好 2 张的组合：3 星哺乳纲 3/6 为猛虎 + 泰特拉；3 星哺乳纲 3/5 为火枪手 + 因达；3 星哺乳纲 4/5 为电饭煲 + 大卫；3 星哺乳纲 4/4 为侠客 + 奥利佛；3 星哺乳纲 4/6 为黑猩猩坎兹 + 猫；4 星机械 5/7 为自动驾驶汽车 + 布里切特；其余组合均不超过 2 张。阿兹改为 4 星 5/6 后，哺乳纲与机械各自只有它 1 张 |
| 同「玩法 + 星级 + 效果技能集」> 1 | 未发现：同身材组的卡技能集均不同；詹纳与李斯特的 `Cleanse` 模板不同（单体免疫 vs 群体清除）；疱疹病毒、烟草花叶病毒改用病毒 3–4 星未被占用的效果技能 |
| 只带关键词卡按「玩法 + 星级 + 身材 + 完整技能列表」> 1 | 未发现：手工耿（随从）4 星机械 6/7，与现有 `self-driving-car` 5/7、`telephone` 7/6 不同身材 |
| 攻 0 卡按「玩法 + 星级 + 完整技能列表」> 1 | 未发现：新增 6 张 0 攻卡的技能列表互不相同，且与现有 0 攻卡不重复 |
| 技能数量 | 3–4 星 1–2 个；5–7 星 1–2 个（扩展批次统一 2 个）；均未超限 |
| 关键词登记 | 新增双技能卡各登记 1 个关键词；每张卡登记关键词不超过 2 个 |
| 效果牌星级占比（累计，进池口径） | 1★ 17/60（28.3%）、2★ 26/89（29.2%）、3★ 32/112（28.6%）、4★ 31/110（28.2%）、5★ 27/95（28.4%）、6★ 15/52（28.8%）、7★ 9/34（26.5%）；均低于 30%，2★ 最接近上限 |
| 诺奖批次与随从分布 | 32 张全部为效果牌，不进入随从身材、效果技能集与技能密度统计，只参与效果牌占比与费用校验 |
| 4 费效果牌 | 新增 9 张，均为 6–7 星：观测者效应（6★）、寒武纪大爆发（6★）、大过滤器（7★）、mRNA 疫苗（6★）、引力波天文学（6★）、金属有机框架（6★）、希格斯场（7★）、中微子振荡（7★）、宇宙微波背景（7★）；连同现有 6 张，4 费牌共 15 张，全部落在 6–7 星，符合「4 费仅限 6–7 星」 |

#### 8.3 技能密度

| 指标 | 现状 | 加入三批后 | 要求 | 结论 |
| --- | --- | --- | --- | --- |
| 3–4 星至少 2 个技能 | 50 / 100（50.0%） | 80 / 139（57.6%） | ≥50% | 通过（首批 3–4 星 10 张中 5 张双技能；扩展 3–4 星 29 张中 25 张补第二技能；诺奖批次不含随从，不影响本行） |
| 5–7 星至少 2 个技能 | 71 / 79（89.9%） | 90 / 101（89.1%） | ≥80% | 通过（扩展批次 19 张 5–7 星随从统一补第二技能；诺奖批次不含随从，不影响本行） |

口径同 `scripts/balance_report.py`：只统计进池、攻击力不为 0 的随从，按「技能列表长度 ≥ 2」计数；`make balance-check` 当前输出 3–4 星 50%、5–7 星 90%，与上表「现状」一致。现状恰好卡在 3–4 星的 50% 下限，任何新增单技能 3–4 星卡都会立刻跌破门槛，因此本批 3–4 星必须优先补第二技能；扩展批次的 29 张 3–4 星里有 4 张（琴鸟、蚕、显微镜、望远镜）保持单技能，其余 25 张补关键词。5–7 星为留出余量，19 张全部补关键词。补关键词时按关键词折算扣减 1–3 点身材，逐张复核仍在上限内。

#### 8.4 三批汇总

| 项 | 数值 |
| --- | --- |
| 随从星级分布（现 332 → 418） | 1★ 58（+8）、2★ 69（+11）、3★ 82（+21）、4★ 79（+19）、5★ 68（+13）、6★ 37（+9）、7★ 25（+5） |
| 效果牌星级分布（现 100 → 158） | 1★ 18（+8）、2★ 26（+8）、3★ 32（+10）、4★ 31（+11）、5★ 27（+11）、6★ 15（+6）、7★ 9（+4） |
| 诺奖 32 张效果牌按玩法 | 机械 15、哺乳纲 7、细菌域 7、病毒 2、植物界 1、水螅纲 1（PCR 双归属多计 1 条，共 33 条归属） |
| 诺奖 32 张效果牌按时代 | 现代 31、工业 1 |
| 诺奖 32 张效果牌按学科 | 化学 12、生理学或医学 10、物理学 10 |
| 诺奖 32 张效果牌按支持状态 | A 13、B 12、C 7 |
| 效果牌占比 | 加入诺奖后各星级效果牌占同星级进池卡 26.5%–29.2%，2★ 29.2% 最接近上限 |
| 扩展 66 张随从按玩法 | 鱼纲 7、鸟纲 6、昆虫纲 6、真菌界 6、细菌域 6、病毒 6、头足纲 5、水螅纲 5、机械 5、原生生物界 5、植物界 5、爬行纲 4、哺乳纲 0 |
| 扩展 66 张随从按时代 | 现代 30、史前 12、古典 10、原始 8、工业 5、封建 1 |
| 单卡强度 | 86 张随从全部通过 `攻 + 血 + 折算 ≤ 白板 + 4`；其中 6 张 0 攻卡（首批）按 1.3 倍生命规则取值 |
| 身材组 | 与现有 332 张随从合并后，无「玩法 + 星级 + 攻 + 血」超过 2 张；首批与现有重号的 4 张已改身材：阿兹 5/6、因达 3/5、奥利佛 4/4、猫 4/6 |
| 效果技能集 | 扩展批次内无重复；与现有同「玩法 + 星级」的效果技能集不重复 |
| 技能密度 | 3–4 星 57.6%、5–7 星 89.1%，见 §8.3 |

#### 8.5 自检的性质

以上是纸面推算，不是校验脚本结果。落地时必须用 `python3 scripts/validate_docs.py` 与 `make balance-check` 复核，任何一格与文档不符，以校验输出为准并回改本文。

### 9. 实施拆分与验收

#### 9.1 阶段

| 阶段 | 内容 | 退出条件 |
| --- | --- | --- |
| 0. 规则与词表 | 分类树新增猴科与 45 个扩展叶子；名词表新增灵长目；关键词文档补「出售时」「献祭」「概率」「购买时」「冻结」「沉默」的文案约定 | `make data-check` 通过，文档无断链 |
| 1. 引擎 | `chance`、`OnSell`、`OnBuy`、`GainGold`、`Ramp`、`Cleanse`、`Sacrifice`、`SwapStats`、`Silence`、`Freeze`；金币每回合上限；免疫与沉默、冻结标记 | `cargo test --workspace` 与新增单元测试通过 |
| 2. 模板 | 39 个新模板；模板渲染与校验 | `render_docs.py --check` 无差异 |
| 3. 技能 | 69 个新技能实例；复用清单核对 | 技能命名空间无重名 |
| 4. 卡牌 | 86 随从 + 58 效果 + 8 英雄 + 7 衍生物 | `validate_docs.py` 退出码 0 |
| 5. 数据与模拟 | 重新生成 `server/data/v0.2/`；跑平衡与批量模拟 | `make balance-check`、`make sim-batch && make sim-report` 有结果 |

#### 9.2 落地时必须改的计数

| 项 | 现状 | 加入三批后 |
| --- | --- | --- |
| 卡牌文档 | 432 | 583（+144 进池 +7 非池） |
| 进池卡 | 408 | 552 |
| 非池卡 | 24 | 31 |
| 随从 | 332 | 418 |
| 效果牌 | 100 | 158 |
| 技能 | 262 | 331 |
| 模板 | 145 | 184 |
| 英雄卡 | 32 | 40 |
| 分类树节点 | 338 | 383 |

扩展批次新增的 45 个分类叶子：`actinomycetaceae`、`actinophryidae`、`auriculariaceae`、`bifidobacteriaceae`、`bombycidae`、`chamaeleonidae`、`cicadidae`、`cupressaceae`、`cyaneidae`、`cyanobacteriaceae`、`difflugiidae`、`droseraceae`、`entamoebidae`、`falconidae`、`ganodermataceae`、`gorgoniidae`、`gymnotidae`、`helicobacteraceae`、`hepadnaviridae`、`hericiaceae`、`herpesviridae`、`hypocreaceae`、`ichneumonidae`、`iguanidae`、`istiophoridae`、`lampyridae`、`menuridae`、`noctilucaceae`、`nymphaeaceae`、`pennatulidae`、`phasmatidae`、`phrynosomatidae`、`picornaviridae`、`pleuronectidae`、`prion-incertae-sedis`、`protopteridae`、`pseudomonadaceae`、`raphidae`、`sphenodontidae`、`spirulidae`、`sternidae`、`toxotidae`、`trilobitidae`、`trochilidae`、`virgaviridae`。

英雄总数 32 是 `scripts/validate_docs.py` 的硬断言，加入三批后改为 40；`card-system.md` 与 `skill-system.md` 的期望输出同步更新。

#### 9.3 校验命令

实现后按顺序执行，本文未执行任何一条：

```bash
make data-check
python3 -m unittest discover -s tests -v
make test
make lint
make balance-check
make sim-batch && make sim-report
make check
```

#### 9.4 验收标准

1. 全部新卡可被引擎加载，不存在未实现的原语或字段；
2. 同一 seed 重放，概率与随机目标结果一致；
3. 金币上限、献祭与出售的结算在端到端用例中覆盖；
4. 13 个玩法夺冠率仍在既有门禁内；若新增卡把某玩法推过门禁，先调本批卡，不动全局伤害公式；
5. 3–4 星与 5–7 星技能密度不低于 §8.3 的门槛；
6. `SwapStats`、`Silence`、`Freeze`、`OnBuy` 各有一条端到端用例，覆盖招募与战斗两个阶段；
7. 扩展批次的 66 张随从与 22 张效果牌、以及 §7.9 的 32 张诺奖效果牌全部出现在卡牌索引中，且 `taxon` 均为分类树叶子。

### 10. 风险与未决

| 风险/未决 | 影响 | 处理 |
| --- | --- | --- |
| 经济上限是否足够 | 多张金币卡叠加可能破坏购买节奏 | 先按每回合 3/5 上限实现，模拟后再调；不加「无限金币」类效果 |
| `Ramp` 的上限 | 拖到 20 回合时成长可能超模 | 单体先按 `max=4`；诺奖批次的群体 `Ramp` 固定 `max=2`；若仍超模先降上限，再降 `every` |
| `Cleanse` 与传染的交互 | 清除后立即再被叠层，体感可能像「没生效」 | 免疫只对 `duration=combat` 生效；文案写清「本场战斗内」 |
| `Sacrifice` 的语义 | 玩家可能预期触发亡语 | 文案用「献祭」而非「消灭」；规则文档写明不触发亡语、复生与死亡事件 |
| 概率卡的金色表现 | 金色是否改变概率 | 不改变概率，只翻倍产出；若体验不佳再评估 |
| 出处待考证的素材 | 卡牌正文要求可查证的「## 出处」 | 阿兹、因达、泰特拉一世、布里切特三世、海利恩、薛定谔气体模型、猴子衍生物的出处需在落地前补证；补不到时改用可查证的文化概念表述，不杜撰 |
| 英雄总数断言 | 校验脚本写死 32 | 改为 40，并同步文档期望输出 |
| 属性互换的结算口径 | 光环下交换当前值可能与玩家直觉不符 | 文案写「当前攻击力与生命值互换」，规则文档固定「交换后光环继续叠加」 |
| 沉默的强度 | 沉默全体可能直接废掉敌方体系 | 群体沉默只允许 4 费 7 星；单体沉默可下探到 3–4 星（扩展「沉默的螺旋」4★、诺奖「基因编辑」3★）；模拟后若单体过强先提费 |
| 冻结的强度 | 冻结全体一轮可能直接改变战斗走向 | 群体冻结只允许 6★ 4 费以上（诺奖「引力波天文学」）；单体冻结按 2–3 费；模拟后若过强先减目标数 |
| 购买触发的金币 | 与既有金币来源叠加 | 与 `OnSell`、`GainGold` 共用一个每回合上限 |
| 45 个新分类叶子 | 分类树与名词表一次性扩张较多 | 可按批次切片；不影响卡牌结算，只影响文档与筛选 |
| 3–4 星密度卡在门槛 | 现状恰好 50%，任何新增单技能 3–4 星卡都会立即跌破 | 本批 3–4 星按双技能设计，抬到 57.6%；后续补卡沿用同一约束 |
| 新增偏中高星 | 扩展批次 1–2 星只新增 18 张（1★ 8、2★ 10），首批没有 1–2 星 | 这是「高血人物补强」的刻意取舍；下一批补 1–2 星与植物/病毒低星 |
| 效果牌占比接近 30% | 加入诺奖后 2★ 29.2% 最接近上限，再补 2★ 效果牌或削减 2★ 随从都会越线 | 下一批 2★ 优先补随从；新效果牌按 §8.2 逐星复核 |
| 诺奖批次时代与玩法集中 | 31/32 现代、15/32 机械，可能放大机械与现代的卡池厚度 | 按学科分批上线；先落地医学/化学中低星，物理高星随模拟结果取用 |
| 12 个诺奖模板一次引入 | 模板面扩大，渲染与校验成本上升 | 先做单原语模板（stats/cannot-attack/freeze），再做 sacrifice/copy/cleanse 组合模板 |
| 诺奖卡面的事实核对 | 32 张都引用真实人物与奖项，写错年份或理由会破坏可信度 | 落地时逐卡核对奖项年份与官方理由；卡面不使用得主肖像或奖项商标 |

## 第二部分 · 诺奖全量名册（1028 位）

### 1. 结论

1. **1028 位得主 / 机构，一人一卡**：原始名单 1036 条得主记录，合并同一人多次获奖与同一机构多次获奖后为 1028 个实体；每个实体分配且只分配一张卡，不出现「有奖无卡」。
2. **664 张随从 + 364 张效果牌**：物理、化学、生理学或医学三奖得主做成学者随从；文学、和平、经济学三奖得主做成效果牌。和平奖中的机构同样按效果牌处理。
3. **9 张复用已有内容、1019 张新增**：8 张直接复用现有卡（爱因斯坦、居里、卢瑟福、海森堡、薛定谔、费米、弗莱明、摩尔根），1 张（普朗克）复用第一部分设计，其余全部为新增卡；每张新增卡都有自己的技能 id `<卡牌 id>-skill`，效果取自该奖的效果池。
4. **技能用 36 个共享模板分配**：每个奖项配 6 个技能，按年份顺序轮转分配；名册解决「有没有」，不解决「专属机制」。精选的 32 张诺奖效果牌已经给出专属机制，两者不冲突。
5. **本批不做平衡与分布校验**：按设计阶段约定，星级、费用、技能撞车、卡池占比都留给实施阶段裁剪；名册是素材库存，不是上线清单。

### 2. 分配规则

| 规则 | 口径 |
| --- | --- |
| 一人一卡 | 同一人多次获奖合并为一张卡；卡面年份取首次获奖，备注列列出全部获奖记录 |
| 类型 | 物理 / 化学 / 生理学或医学 → 随从；文学 / 和平 / 经济学 → 效果牌；机构按所属奖项处理 |
| 星级与时代 | 1901–1939 为 4★ 工业；1940–1945 为 5★ 工业；1946–1969 为 5★ 现代；1970–1999 为 6★ 现代；2000 年以后为 7★ 未来 |
| 身材 | 学者随从攻击力 0，生命值取同星级上限 ×1.3：4★ 0/13、5★ 0/17、6★ 0/21、7★ 0/26 |
| 效果牌费用 | 4★ 2 费、5★ 3 费、6★ 3 费、7★ 4 费；4 费只出现在 6–7★ |
| 玩法归属 | 全部为 `play:mammal`（人科随从与人类制度、思想）；机构卡同样以「人类组织」归入哺乳纲 |
| 分类 | 物理 `physics`、化学 `chemistry`、医学 `medicine`、文学 `literature`、和平 `politics`、经济 `economics`；随从统一 `hominidae` |
| id | `laureate-<英文姓名 slug>`；已存在同名卡时复用现有 id，不新建 |
| 技能 | 每奖 6 个共享技能轮转，详 §3；名册阶段不追求专属技能 |

### 3. 奖项效果池（36）

六个效果覆盖该奖项最常见的机制面；同奖项内按年份顺序轮转。每位新增得主的技能 id 为 `<卡牌 id>-skill`，其效果取表内对应条目；共享的只是效果原型，不是技能身份。

#### 3.1 物理学奖（6）

| 技能 id | 名称 | 文案 | 模板 |
| --- | --- | --- | --- |
| `laureate-precision` | 精密测量 | 战吼：发现一张随从 | `on-play-battlecry-discover` |
| `laureate-radiation` | 辐射 | 战斗开始时，使敌方所有随从获得 -1 攻击力 | `combatstart-stats-enemy-all-attack` |
| `laureate-quantum` | 量子跃迁 | 战斗开始时，有 50% 概率使你的所有随从获得 +1/+1 | `combatstart-chance-stats-all-friendly` |
| `laureate-relativity` | 相对性 | 战吼：使一个友方随从的攻击力与生命值互换 | `on-play-battlecry-swap-stats` |
| `laureate-condensed` | 凝聚态 | 战斗开始时，使你的所有随从获得 +0/+2 | `combatstart-stats-all-friendly` |
| `laureate-cosmos` | 天体运行 | 战斗开始时，冻结一个敌方随从 | `combatstart-freeze-enemy-single` |

#### 3.2 化学奖（6）

| 技能 id | 名称 | 文案 | 模板 |
| --- | --- | --- | --- |
| `laureate-synthesis` | 合成 | 战吼：使一个友方随从获得 +2/+2 | `on-play-battlecry-stats-single` |
| `laureate-catalysis` | 催化 | 战吼：使一个友方随从获得两次成长 | `on-play-battlecry-growth-single` |
| `laureate-structure` | 结构 | 战吼：使一个友方随从的攻击力与生命值互换 | `on-play-battlecry-swap-stats` |
| `laureate-analysis` | 分析 | 战吼：发现一张效果牌 | `on-play-battlecry-discover` |
| `laureate-materials` | 材料 | 战吼：使你的所有随从获得 +0/+2 | `on-play-battlecry-stats-all-friendly` |
| `laureate-purification` | 提纯 | 战吼：清除你的所有随从的传染 | `on-play-battlecry-cleanse-all-friendly` |

#### 3.3 生理学或医学奖（6）

| 技能 id | 名称 | 文案 | 模板 |
| --- | --- | --- | --- |
| `laureate-immunity` | 免疫 | 战斗开始时，使你的所有随从获得护盾 | `combatstart-grant-keyword-all` |
| `laureate-genetics` | 遗传 | 战吼：复制一个友方随从到手牌 | `on-play-battlecry-copy-to-hand` |
| `laureate-neurology` | 神经 | 战斗开始时，使一个敌方随从本回合无法攻击 | `combatstart-cannot-attack-single` |
| `laureate-microbiology` | 微生物 | 战斗开始时，使一个敌方随从失去全部技能与关键词 | `combatstart-silence-single` |
| `laureate-physiology` | 生理 | 战吼：使一个友方随从获得 +0/+3 | `on-play-battlecry-stats-single` |
| `laureate-pharmacology` | 药理 | 战吼：清除一个友方随从的传染，并使其获得潜行 | `on-play-battlecry-cleanse-stealth` |

#### 3.4 文学奖（6）

| 技能 id | 名称 | 文案 | 模板 |
| --- | --- | --- | --- |
| `narrative-epic` | 史诗 | 使你的所有随从获得 +1 攻击力 | `on-play-effect-stats-all-friendly` |
| `narrative-lyric` | 抒情 | 使一个友方随从获得 +2/+2 | `on-play-effect-stats-single` |
| `narrative-satire` | 讽刺 | 使一个敌方随从失去全部技能与关键词 | `combatstart-silence-single` |
| `narrative-realism` | 写实 | 发现一张随从 | `on-play-effect-discover` |
| `narrative-modernism` | 现代主义 | 使一个友方随从的攻击力与生命值互换 | `on-play-effect-swap-stats` |
| `narrative-memory` | 记忆 | 复制一个友方随从到手牌 | `on-play-effect-copy` |

#### 3.5 和平奖（6）

| 技能 id | 名称 | 文案 | 模板 |
| --- | --- | --- | --- |
| `peace-mediation` | 调停 | 战斗开始时，使一个敌方随从本回合无法攻击 | `combatstart-cannot-attack-single` |
| `peace-relief` | 救济 | 使你的所有随从获得 +0/+2 | `on-play-effect-stats-all-friendly` |
| `peace-disarmament` | 裁军 | 战斗开始时，使敌方所有随从获得 -1 攻击力 | `combatstart-stats-enemy-all-attack` |
| `peace-rights` | 权利 | 清除你的所有随从的传染 | `on-play-effect-cleanse-all-friendly` |
| `peace-institution` | 制度 | 获得 2 金币 | `on-play-effect-gain-gold` |
| `peace-solidarity` | 团结 | 使你的所有随从获得 +1/+1 | `on-play-effect-stats-all-friendly` |

#### 3.6 经济学奖（6）

| 技能 id | 名称 | 文案 | 模板 |
| --- | --- | --- | --- |
| `econ-incentive` | 激励 | 获得 2 金币 | `on-play-effect-gain-gold` |
| `econ-equilibrium` | 均衡 | 使一个友方随从获得 +2/+2 | `on-play-effect-stats-single` |
| `econ-information` | 信息 | 发现一张随从 | `on-play-effect-discover` |
| `econ-institution` | 制度 | 获得 2 金币，然后使你的所有随从获得 +0/+1 | `on-play-effect-gain-gold-then-all-stats` |
| `econ-growth` | 增长 | 使一个友方随从获得两次成长 | `on-play-effect-growth-single` |
| `econ-behavior` | 行为 | 使一个友方随从的攻击力与生命值互换 | `on-play-effect-swap-stats` |

### 4. 名册专用模板（15）

以下模板是本名册新用到的；其余模板（`combatstart-stats-enemy-all-attack`、`combatstart-freeze-enemy-single`、`combatstart-cannot-attack-single`、`combatstart-silence-single`、`on-play-effect-stats-single`、`on-play-effect-stats-all-friendly`、`on-play-effect-gain-gold`、`on-play-battlecry-growth-single`、`on-play-effect-cleanse-stealth`）已在第一部分 §6.2、§6.6–§6.8 定义。

| 模板 id | 触发 | 原语 | 选择器 | 说明 |
| --- | --- | --- | --- | --- |
| `on-play-battlecry-discover` | `OnPlay` | `Discover` | `self` | 战吼：发现一张随从或效果牌，`pool` 参数区分 |
| `on-play-battlecry-stats-single` | `OnPlay` | `ModifyStats` | `friendly:single` | 战吼：使一个友方随从获得 +X/+Y |
| `on-play-battlecry-stats-all-friendly` | `OnPlay` | `ModifyStats` | `friendly:all` | 战吼：使你的所有随从获得 +X/+Y |
| `on-play-battlecry-swap-stats` | `OnPlay` | `SwapStats` | `friendly:single` | 战吼：使一个友方随从的攻击力与生命值互换 |
| `on-play-battlecry-copy-to-hand` | `OnPlay` | `Copy` | `friendly:single` | 战吼：复制一个友方随从到手牌 |
| `on-play-battlecry-cleanse-all-friendly` | `OnPlay` | `Cleanse` | `friendly:all` | 战吼：清除你的所有随从的传染 |
| `on-play-battlecry-cleanse-stealth` | `OnPlay` | `Cleanse；GrantKeyword` | `friendly:single` | 战吼：清除传染并获得潜行，`duration=combat` |
| `combatstart-stats-all-friendly` | `OnCombatStart` | `ModifyStats` | `friendly:all` | 战斗开始时，使你的所有随从获得 +X/+Y |
| `combatstart-chance-stats-all-friendly` | `OnCombatStart` | `ModifyStats` | `friendly:all` | 战斗开始时，按概率使你的所有随从获得 +X/+Y |
| `combatstart-grant-keyword-all` | `OnCombatStart` | `GrantKeyword` | `friendly:all` | 战斗开始时，使你的所有随从获得关键词 |
| `on-play-effect-discover` | `OnPlayEffect` | `Discover` | `self` | 发现一张随从或效果牌 |
| `on-play-effect-swap-stats` | `OnPlayEffect` | `SwapStats` | `friendly:single` | 使一个友方随从的攻击力与生命值互换 |
| `on-play-effect-copy` | `OnPlayEffect` | `Copy` | `friendly:single` | 复制一个友方随从到手牌 |
| `on-play-effect-cleanse-all-friendly` | `OnPlayEffect` | `Cleanse` | `friendly:all` | 清除你的所有随从的传染 |
| `on-play-effect-gain-gold-then-all-stats` | `OnPlayEffect` | `GainGold；ModifyStats` | `self；friendly:all` | 获得金币，然后使你的所有随从获得 +X/+Y |

### 5. 全量名册

列口径：`类型` 为随从或效果；`星级/费用` 随从写星级、效果写费用；`技能` 为个人技能 id，效果原型见 §3；`备注` 对新增卡写官方英文原名与效果来源，对复用卡写复用来源。

#### 5.1 物理学奖（230）

| 年份 | 得主 | 类型 | 卡牌 id | 星级/费用 | 时代 | 技能 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1901 | 威廉·康拉德·伦琴 | 随从 | `laureate-wilhelm-conrad-rontgen` | 4★ | 工业 | `laureate-wilhelm-conrad-rontgen-skill` | Wilhelm Conrad Röntgen；效果取自「精密测量」（§3） |
| 1902 | 亨德里克·洛伦兹 | 随从 | `laureate-hendrik-a-lorentz` | 4★ | 工业 | `laureate-hendrik-a-lorentz-skill` | Hendrik A. Lorentz；效果取自「辐射」（§3） |
| 1902 | 彼得·塞曼 | 随从 | `laureate-pieter-zeeman` | 4★ | 工业 | `laureate-pieter-zeeman-skill` | Pieter Zeeman；效果取自「量子跃迁」（§3） |
| 1903 | 亨利·贝克勒尔 | 随从 | `laureate-henri-becquerel` | 4★ | 工业 | `laureate-henri-becquerel-skill` | Henri Becquerel；效果取自「相对性」（§3） |
| 1903 | 玛丽·居里 | 现有随从 | `marie-curie` | 5★ | 现代 | `[battlecry-friendly-growth-twice, radiation]` | 复用现有卡，不新增 |
| 1903 | 皮埃尔·居里 | 随从 | `laureate-pierre-curie` | 4★ | 工业 | `laureate-pierre-curie-skill` | Pierre Curie；效果取自「凝聚态」（§3） |
| 1904 | 瑞利勋爵 | 随从 | `laureate-lord-rayleigh` | 4★ | 工业 | `laureate-lord-rayleigh-skill` | Lord Rayleigh；效果取自「天体运行」（§3） |
| 1905 | 菲利普·莱纳德 | 随从 | `laureate-philipp-lenard` | 4★ | 工业 | `laureate-philipp-lenard-skill` | Philipp Lenard；效果取自「精密测量」（§3） |
| 1906 | 约瑟夫·约翰·汤姆孙 | 随从 | `laureate-j-j-thomson` | 4★ | 工业 | `laureate-j-j-thomson-skill` | J.J. Thomson；效果取自「辐射」（§3） |
| 1907 | 阿尔伯特·迈克尔逊 | 随从 | `laureate-albert-a-michelson` | 4★ | 工业 | `laureate-albert-a-michelson-skill` | Albert A. Michelson；效果取自「量子跃迁」（§3） |
| 1908 | 加布里埃尔·李普曼 | 随从 | `laureate-gabriel-lippmann` | 4★ | 工业 | `laureate-gabriel-lippmann-skill` | Gabriel Lippmann；效果取自「相对性」（§3） |
| 1909 | 费迪南德·布劳恩 | 随从 | `laureate-ferdinand-braun` | 4★ | 工业 | `laureate-ferdinand-braun-skill` | Ferdinand Braun；效果取自「凝聚态」（§3） |
| 1909 | 古列尔莫·马可尼 | 随从 | `laureate-guglielmo-marconi` | 4★ | 工业 | `laureate-guglielmo-marconi-skill` | Guglielmo Marconi；效果取自「天体运行」（§3） |
| 1910 | 约翰内斯·迪德里克·范德华 | 随从 | `laureate-johannes-diderik-van-der-waals` | 4★ | 工业 | `laureate-johannes-diderik-van-der-waals-skill` | Johannes Diderik van der Waals；效果取自「精密测量」（§3） |
| 1911 | 威廉·维恩 | 随从 | `laureate-wilhelm-wien` | 4★ | 工业 | `laureate-wilhelm-wien-skill` | Wilhelm Wien；效果取自「辐射」（§3） |
| 1912 | 古斯塔夫·达伦 | 随从 | `laureate-gustaf-dalen` | 4★ | 工业 | `laureate-gustaf-dalen-skill` | Gustaf Dalén；效果取自「量子跃迁」（§3） |
| 1913 | 海克·卡末林·昂内斯 | 随从 | `laureate-heike-kamerlingh-onnes` | 4★ | 工业 | `laureate-heike-kamerlingh-onnes-skill` | Heike Kamerlingh Onnes；效果取自「相对性」（§3） |
| 1914 | 马克斯·冯·劳厄 | 随从 | `laureate-max-von-laue` | 4★ | 工业 | `laureate-max-von-laue-skill` | Max von Laue；效果取自「凝聚态」（§3） |
| 1915 | 劳伦斯·布拉格 | 随从 | `laureate-lawrence-bragg` | 4★ | 工业 | `laureate-lawrence-bragg-skill` | Lawrence Bragg；效果取自「天体运行」（§3） |
| 1915 | 威廉·布拉格 | 随从 | `laureate-william-bragg` | 4★ | 工业 | `laureate-william-bragg-skill` | William Bragg；效果取自「精密测量」（§3） |
| 1917 | 查尔斯·格洛弗·巴克拉 | 随从 | `laureate-charles-glover-barkla` | 4★ | 工业 | `laureate-charles-glover-barkla-skill` | Charles Glover Barkla；效果取自「辐射」（§3） |
| 1918 | 马克斯·普朗克 | 设计随从 | `planck` | 5★ | 工业 | `battlecry-enemy-all-minus-1-attack` | 复用第一部分设计，不重复建卡 |
| 1919 | 约翰内斯·斯塔克 | 随从 | `laureate-johannes-stark` | 4★ | 工业 | `laureate-johannes-stark-skill` | Johannes Stark；效果取自「量子跃迁」（§3） |
| 1920 | 夏尔·爱德华·纪尧姆 | 随从 | `laureate-charles-edouard-guillaume` | 4★ | 工业 | `laureate-charles-edouard-guillaume-skill` | Charles Edouard Guillaume；效果取自「相对性」（§3） |
| 1921 | 阿尔伯特·爱因斯坦 | 现有随从 | `einstein` | 5★ | 现代 | `[battlecry-all-friendly-growth-once, light-speed-strike]` | 复用现有卡，不新增 |
| 1922 | 尼尔斯·玻尔 | 随从 | `laureate-niels-bohr` | 4★ | 工业 | `laureate-niels-bohr-skill` | Niels Bohr；效果取自「凝聚态」（§3） |
| 1923 | 罗伯特·密立根 | 随从 | `laureate-robert-a-millikan` | 4★ | 工业 | `laureate-robert-a-millikan-skill` | Robert A. Millikan；效果取自「天体运行」（§3） |
| 1924 | 曼内·西格巴恩 | 随从 | `laureate-manne-siegbahn` | 4★ | 工业 | `laureate-manne-siegbahn-skill` | Manne Siegbahn；效果取自「精密测量」（§3） |
| 1925 | 古斯塔夫·赫兹 | 随从 | `laureate-gustav-hertz` | 4★ | 工业 | `laureate-gustav-hertz-skill` | Gustav Hertz；效果取自「辐射」（§3） |
| 1925 | 詹姆斯·弗兰克 | 随从 | `laureate-james-franck` | 4★ | 工业 | `laureate-james-franck-skill` | James Franck；效果取自「量子跃迁」（§3） |
| 1926 | 让·巴蒂斯特·佩兰 | 随从 | `laureate-jean-baptiste-perrin` | 4★ | 工业 | `laureate-jean-baptiste-perrin-skill` | Jean Baptiste Perrin；效果取自「相对性」（§3） |
| 1927 | 阿瑟·康普顿 | 随从 | `laureate-arthur-h-compton` | 4★ | 工业 | `laureate-arthur-h-compton-skill` | Arthur H. Compton；效果取自「凝聚态」（§3） |
| 1927 | 查尔斯·汤姆逊·里斯·威尔逊 | 随从 | `laureate-c-t-r-wilson` | 4★ | 工业 | `laureate-c-t-r-wilson-skill` | C.T.R. Wilson；效果取自「天体运行」（§3） |
| 1928 | 欧文·威兰斯·理查森 | 随从 | `laureate-owen-willans-richardson` | 4★ | 工业 | `laureate-owen-willans-richardson-skill` | Owen Willans Richardson；效果取自「精密测量」（§3） |
| 1929 | 路易·德布罗意 | 随从 | `laureate-louis-de-broglie` | 4★ | 工业 | `laureate-louis-de-broglie-skill` | Louis de Broglie；效果取自「辐射」（§3） |
| 1930 | 钱德拉塞卡拉·文卡塔·拉曼 | 随从 | `laureate-sir-chandrasekhara-venkata-raman` | 4★ | 工业 | `laureate-sir-chandrasekhara-venkata-raman-skill` | Sir Chandrasekhara Venkata Raman；效果取自「量子跃迁」（§3） |
| 1932 | 维尔纳·海森堡 | 现有随从 | `heisenberg` | 7★ | 未来 | `[keyword-reborn, deathrattle-all-enemies-minus-1-health]` | 复用现有卡，不新增 |
| 1933 | 埃尔温·薛定谔 | 现有随从 | `schrodinger` | 7★ | 未来 | `[keyword-divine-shield, combatstart-random-enemy-contagion-2]` | 复用现有卡，不新增 |
| 1933 | 保罗·狄拉克 | 随从 | `laureate-paul-a-m-dirac` | 4★ | 工业 | `laureate-paul-a-m-dirac-skill` | Paul A.M. Dirac；效果取自「相对性」（§3） |
| 1935 | 詹姆斯·查德威克 | 随从 | `laureate-james-chadwick` | 4★ | 工业 | `laureate-james-chadwick-skill` | James Chadwick；效果取自「凝聚态」（§3） |
| 1936 | 卡尔·安德森 | 随从 | `laureate-carl-d-anderson` | 4★ | 工业 | `laureate-carl-d-anderson-skill` | Carl D. Anderson；效果取自「天体运行」（§3） |
| 1936 | 维克托·赫斯 | 随从 | `laureate-victor-f-hess` | 4★ | 工业 | `laureate-victor-f-hess-skill` | Victor F. Hess；效果取自「精密测量」（§3） |
| 1937 | 克林顿·戴维森 | 随从 | `laureate-clinton-davisson` | 4★ | 工业 | `laureate-clinton-davisson-skill` | Clinton Davisson；效果取自「辐射」（§3） |
| 1937 | 乔治·佩吉特·汤姆孙 | 随从 | `laureate-george-paget-thomson` | 4★ | 工业 | `laureate-george-paget-thomson-skill` | George Paget Thomson；效果取自「量子跃迁」（§3） |
| 1938 | 恩里科·费米 | 现有随从 | `fermi` | 7★ | 未来 | `[battlecry-all-friendly-growth-once]` | 复用现有卡，不新增 |
| 1939 | 欧内斯特·劳伦斯 | 随从 | `laureate-ernest-lawrence` | 4★ | 工业 | `laureate-ernest-lawrence-skill` | Ernest Lawrence；效果取自「相对性」（§3） |
| 1943 | 奥托·斯特恩 | 随从 | `laureate-otto-stern` | 5★ | 工业 | `laureate-otto-stern-skill` | Otto Stern；效果取自「凝聚态」（§3） |
| 1944 | 伊西多·艾萨克·拉比 | 随从 | `laureate-isidor-isaac-rabi` | 5★ | 工业 | `laureate-isidor-isaac-rabi-skill` | Isidor Isaac Rabi；效果取自「天体运行」（§3） |
| 1945 | 沃尔夫冈·泡利 | 随从 | `laureate-wolfgang-pauli` | 5★ | 工业 | `laureate-wolfgang-pauli-skill` | Wolfgang Pauli；效果取自「精密测量」（§3） |
| 1946 | 珀西·布里奇曼 | 随从 | `laureate-percy-w-bridgman` | 5★ | 现代 | `laureate-percy-w-bridgman-skill` | Percy W. Bridgman；效果取自「辐射」（§3） |
| 1947 | 爱德华·阿普尔顿 | 随从 | `laureate-edward-v-appleton` | 5★ | 现代 | `laureate-edward-v-appleton-skill` | Edward V. Appleton；效果取自「量子跃迁」（§3） |
| 1948 | 帕特里克·布莱克特 | 随从 | `laureate-patrick-m-s-blackett` | 5★ | 现代 | `laureate-patrick-m-s-blackett-skill` | Patrick M.S. Blackett；效果取自「相对性」（§3） |
| 1949 | 汤川秀树 | 随从 | `laureate-hideki-yukawa` | 5★ | 现代 | `laureate-hideki-yukawa-skill` | Hideki Yukawa；效果取自「凝聚态」（§3） |
| 1950 | 塞西尔·鲍威尔 | 随从 | `laureate-cecil-powell` | 5★ | 现代 | `laureate-cecil-powell-skill` | Cecil Powell；效果取自「天体运行」（§3） |
| 1951 | 欧内斯特·沃尔顿 | 随从 | `laureate-ernest-t-s-walton` | 5★ | 现代 | `laureate-ernest-t-s-walton-skill` | Ernest T.S. Walton；效果取自「精密测量」（§3） |
| 1951 | 约翰·考克饶夫 | 随从 | `laureate-john-cockcroft` | 5★ | 现代 | `laureate-john-cockcroft-skill` | John Cockcroft；效果取自「辐射」（§3） |
| 1952 | 爱德华·珀塞尔 | 随从 | `laureate-e-m-purcell` | 5★ | 现代 | `laureate-e-m-purcell-skill` | E. M. Purcell；效果取自「量子跃迁」（§3） |
| 1952 | 费利克斯·布洛赫 | 随从 | `laureate-felix-bloch` | 5★ | 现代 | `laureate-felix-bloch-skill` | Felix Bloch；效果取自「相对性」（§3） |
| 1953 | 弗里茨·泽尼克 | 随从 | `laureate-frits-zernike` | 5★ | 现代 | `laureate-frits-zernike-skill` | Frits Zernike；效果取自「凝聚态」（§3） |
| 1954 | 马克斯·玻恩 | 随从 | `laureate-max-born` | 5★ | 现代 | `laureate-max-born-skill` | Max Born；效果取自「天体运行」（§3） |
| 1954 | 瓦尔特·博特 | 随从 | `laureate-walther-bothe` | 5★ | 现代 | `laureate-walther-bothe-skill` | Walther Bothe；效果取自「精密测量」（§3） |
| 1955 | 波利卡普·库施 | 随从 | `laureate-polykarp-kusch` | 5★ | 现代 | `laureate-polykarp-kusch-skill` | Polykarp Kusch；效果取自「辐射」（§3） |
| 1955 | 威利斯·兰姆 | 随从 | `laureate-willis-e-lamb` | 5★ | 现代 | `laureate-willis-e-lamb-skill` | Willis E. Lamb；效果取自「量子跃迁」（§3） |
| 1956 | 约翰·巴丁 | 随从 | `laureate-john-bardeen` | 5★ | 现代 | `laureate-john-bardeen-skill` | John Bardeen；效果取自「相对性」（§3）；1956 物理学奖、1972 物理学奖 |
| 1956 | 沃尔特·布拉顿 | 随从 | `laureate-walter-h-brattain` | 5★ | 现代 | `laureate-walter-h-brattain-skill` | Walter H. Brattain；效果取自「凝聚态」（§3） |
| 1956 | 威廉·肖克利 | 随从 | `laureate-william-b-shockley` | 5★ | 现代 | `laureate-william-b-shockley-skill` | William B. Shockley；效果取自「天体运行」（§3） |
| 1957 | 杨振宁 | 随从 | `laureate-chen-ning-yang` | 5★ | 现代 | `laureate-chen-ning-yang-skill` | Chen Ning Yang；效果取自「精密测量」（§3） |
| 1957 | 李政道 | 随从 | `laureate-tsung-dao-lee` | 5★ | 现代 | `laureate-tsung-dao-lee-skill` | Tsung-Dao Lee；效果取自「辐射」（§3） |
| 1958 | 伊戈尔·塔姆 | 随从 | `laureate-igor-y-tamm` | 5★ | 现代 | `laureate-igor-y-tamm-skill` | Igor Y. Tamm；效果取自「量子跃迁」（§3） |
| 1958 | 伊利亚·弗兰克 | 随从 | `laureate-il-ja-m-frank` | 5★ | 现代 | `laureate-il-ja-m-frank-skill` | Il´ja M. Frank；效果取自「相对性」（§3） |
| 1958 | 帕维尔·切伦科夫 | 随从 | `laureate-pavel-a-cherenkov` | 5★ | 现代 | `laureate-pavel-a-cherenkov-skill` | Pavel A. Cherenkov；效果取自「凝聚态」（§3） |
| 1959 | 埃米利奥·塞格雷 | 随从 | `laureate-emilio-segre` | 5★ | 现代 | `laureate-emilio-segre-skill` | Emilio Segrè；效果取自「天体运行」（§3） |
| 1959 | 欧文·张伯伦 | 随从 | `laureate-owen-chamberlain` | 5★ | 现代 | `laureate-owen-chamberlain-skill` | Owen Chamberlain；效果取自「精密测量」（§3） |
| 1960 | 唐纳德·格拉泽 | 随从 | `laureate-donald-a-glaser` | 5★ | 现代 | `laureate-donald-a-glaser-skill` | Donald A. Glaser；效果取自「辐射」（§3） |
| 1961 | 罗伯特·霍夫施塔特 | 随从 | `laureate-robert-hofstadter` | 5★ | 现代 | `laureate-robert-hofstadter-skill` | Robert Hofstadter；效果取自「量子跃迁」（§3） |
| 1961 | 鲁道夫·穆斯堡尔 | 随从 | `laureate-rudolf-mossbauer` | 5★ | 现代 | `laureate-rudolf-mossbauer-skill` | Rudolf Mössbauer；效果取自「相对性」（§3） |
| 1962 | 列夫·朗道 | 随从 | `laureate-lev-landau` | 5★ | 现代 | `laureate-lev-landau-skill` | Lev Landau；效果取自「凝聚态」（§3） |
| 1963 | 尤金·维格纳 | 随从 | `laureate-eugene-wigner` | 5★ | 现代 | `laureate-eugene-wigner-skill` | Eugene Wigner；效果取自「天体运行」（§3） |
| 1963 | 约翰内斯·汉斯·丹尼尔·延森 | 随从 | `laureate-j-hans-d-jensen` | 5★ | 现代 | `laureate-j-hans-d-jensen-skill` | J. Hans D. Jensen；效果取自「精密测量」（§3） |
| 1963 | 玛丽亚·格佩特-梅耶 | 随从 | `laureate-maria-goeppert-mayer` | 5★ | 现代 | `laureate-maria-goeppert-mayer-skill` | Maria Goeppert Mayer；效果取自「辐射」（§3） |
| 1964 | 亚历山大·普罗霍罗夫 | 随从 | `laureate-aleksandr-m-prokhorov` | 5★ | 现代 | `laureate-aleksandr-m-prokhorov-skill` | Aleksandr M. Prokhorov；效果取自「量子跃迁」（§3） |
| 1964 | 查尔斯·汤斯 | 随从 | `laureate-charles-h-townes` | 5★ | 现代 | `laureate-charles-h-townes-skill` | Charles H. Townes；效果取自「相对性」（§3） |
| 1964 | 尼古拉·巴索夫 | 随从 | `laureate-nicolay-g-basov` | 5★ | 现代 | `laureate-nicolay-g-basov-skill` | Nicolay G. Basov；效果取自「凝聚态」（§3） |
| 1965 | 朱利安·施温格 | 随从 | `laureate-julian-schwinger` | 5★ | 现代 | `laureate-julian-schwinger-skill` | Julian Schwinger；效果取自「天体运行」（§3） |
| 1965 | 理查德·费曼 | 随从 | `laureate-richard-p-feynman` | 5★ | 现代 | `laureate-richard-p-feynman-skill` | Richard P. Feynman；效果取自「精密测量」（§3） |
| 1965 | 朝永振一郎 | 随从 | `laureate-sin-itiro-tomonaga` | 5★ | 现代 | `laureate-sin-itiro-tomonaga-skill` | Sin-Itiro Tomonaga；效果取自「辐射」（§3） |
| 1966 | 阿尔弗雷德·卡斯特勒 | 随从 | `laureate-alfred-kastler` | 5★ | 现代 | `laureate-alfred-kastler-skill` | Alfred Kastler；效果取自「量子跃迁」（§3） |
| 1967 | 汉斯·贝特 | 随从 | `laureate-hans-bethe` | 5★ | 现代 | `laureate-hans-bethe-skill` | Hans Bethe；效果取自「相对性」（§3） |
| 1968 | 路易斯·阿尔瓦雷斯 | 随从 | `laureate-luis-alvarez` | 5★ | 现代 | `laureate-luis-alvarez-skill` | Luis Alvarez；效果取自「凝聚态」（§3） |
| 1969 | 默里·盖尔曼 | 随从 | `laureate-murray-gell-mann` | 5★ | 现代 | `laureate-murray-gell-mann-skill` | Murray Gell-Mann；效果取自「天体运行」（§3） |
| 1970 | 汉内斯·阿尔文 | 随从 | `laureate-hannes-alfven` | 6★ | 现代 | `laureate-hannes-alfven-skill` | Hannes Alfvén；效果取自「精密测量」（§3） |
| 1970 | 路易·奈尔 | 随从 | `laureate-louis-neel` | 6★ | 现代 | `laureate-louis-neel-skill` | Louis Néel；效果取自「辐射」（§3） |
| 1971 | 丹尼斯·加博尔 | 随从 | `laureate-dennis-gabor` | 6★ | 现代 | `laureate-dennis-gabor-skill` | Dennis Gabor；效果取自「量子跃迁」（§3） |
| 1972 | 莱昂·库珀 | 随从 | `laureate-leon-n-cooper` | 6★ | 现代 | `laureate-leon-n-cooper-skill` | Leon N. Cooper；效果取自「相对性」（§3） |
| 1972 | 罗伯特·施里弗 | 随从 | `laureate-robert-schrieffer` | 6★ | 现代 | `laureate-robert-schrieffer-skill` | Robert Schrieffer；效果取自「凝聚态」（§3） |
| 1973 | 布赖恩·约瑟夫森 | 随从 | `laureate-brian-d-josephson` | 6★ | 现代 | `laureate-brian-d-josephson-skill` | Brian D. Josephson；效果取自「天体运行」（§3） |
| 1973 | 伊瓦尔·贾埃弗 | 随从 | `laureate-ivar-giaever` | 6★ | 现代 | `laureate-ivar-giaever-skill` | Ivar Giaever；效果取自「精密测量」（§3） |
| 1973 | 江崎玲於奈 | 随从 | `laureate-leo-esaki` | 6★ | 现代 | `laureate-leo-esaki-skill` | Leo Esaki；效果取自「辐射」（§3） |
| 1974 | 安东尼·休伊什 | 随从 | `laureate-antony-hewish` | 6★ | 现代 | `laureate-antony-hewish-skill` | Antony Hewish；效果取自「量子跃迁」（§3） |
| 1974 | 马丁·赖尔 | 随从 | `laureate-martin-ryle` | 6★ | 现代 | `laureate-martin-ryle-skill` | Martin Ryle；效果取自「相对性」（§3） |
| 1975 | 奥格·玻尔 | 随从 | `laureate-aage-n-bohr` | 6★ | 现代 | `laureate-aage-n-bohr-skill` | Aage N. Bohr；效果取自「凝聚态」（§3） |
| 1975 | 本·莫特尔森 | 随从 | `laureate-ben-r-mottelson` | 6★ | 现代 | `laureate-ben-r-mottelson-skill` | Ben R. Mottelson；效果取自「天体运行」（§3） |
| 1975 | 詹姆斯·雷恩沃特 | 随从 | `laureate-james-rainwater` | 6★ | 现代 | `laureate-james-rainwater-skill` | James Rainwater；效果取自「精密测量」（§3） |
| 1976 | 伯顿·里克特 | 随从 | `laureate-burton-richter` | 6★ | 现代 | `laureate-burton-richter-skill` | Burton Richter；效果取自「辐射」（§3） |
| 1976 | 丁肇中 | 随从 | `laureate-samuel-c-c-ting` | 6★ | 现代 | `laureate-samuel-c-c-ting-skill` | Samuel C.C. Ting；效果取自「量子跃迁」（§3） |
| 1977 | 约翰·范弗莱克 | 随从 | `laureate-john-h-van-vleck` | 6★ | 现代 | `laureate-john-h-van-vleck-skill` | John H. Van Vleck；效果取自「相对性」（§3） |
| 1977 | 菲利普·安德森 | 随从 | `laureate-philip-w-anderson` | 6★ | 现代 | `laureate-philip-w-anderson-skill` | Philip W. Anderson；效果取自「凝聚态」（§3） |
| 1977 | 内维尔·莫特 | 随从 | `laureate-sir-nevill-f-mott` | 6★ | 现代 | `laureate-sir-nevill-f-mott-skill` | Sir Nevill F. Mott；效果取自「天体运行」（§3） |
| 1978 | 阿诺·彭齐亚斯 | 随从 | `laureate-arno-penzias` | 6★ | 现代 | `laureate-arno-penzias-skill` | Arno Penzias；效果取自「精密测量」（§3） |
| 1978 | 彼得·卡皮察 | 随从 | `laureate-pyotr-kapitsa` | 6★ | 现代 | `laureate-pyotr-kapitsa-skill` | Pyotr Kapitsa；效果取自「辐射」（§3） |
| 1978 | 罗伯特·伍德罗·威尔逊 | 随从 | `laureate-robert-woodrow-wilson` | 6★ | 现代 | `laureate-robert-woodrow-wilson-skill` | Robert Woodrow Wilson；效果取自「量子跃迁」（§3） |
| 1979 | 阿卜杜勒·萨拉姆 | 随从 | `laureate-abdus-salam` | 6★ | 现代 | `laureate-abdus-salam-skill` | Abdus Salam；效果取自「相对性」（§3） |
| 1979 | 谢尔登·格拉肖 | 随从 | `laureate-sheldon-glashow` | 6★ | 现代 | `laureate-sheldon-glashow-skill` | Sheldon Glashow；效果取自「凝聚态」（§3） |
| 1979 | 史蒂文·温伯格 | 随从 | `laureate-steven-weinberg` | 6★ | 现代 | `laureate-steven-weinberg-skill` | Steven Weinberg；效果取自「天体运行」（§3） |
| 1980 | 詹姆斯·克罗宁 | 随从 | `laureate-james-cronin` | 6★ | 现代 | `laureate-james-cronin-skill` | James Cronin；效果取自「精密测量」（§3） |
| 1980 | 瓦尔·菲奇 | 随从 | `laureate-val-fitch` | 6★ | 现代 | `laureate-val-fitch-skill` | Val Fitch；效果取自「辐射」（§3） |
| 1981 | 阿瑟·肖洛 | 随从 | `laureate-arthur-l-schawlow` | 6★ | 现代 | `laureate-arthur-l-schawlow-skill` | Arthur L. Schawlow；效果取自「量子跃迁」（§3） |
| 1981 | 凯·西格巴恩 | 随从 | `laureate-kai-m-siegbahn` | 6★ | 现代 | `laureate-kai-m-siegbahn-skill` | Kai M. Siegbahn；效果取自「相对性」（§3） |
| 1981 | 尼古拉斯·布洛姆伯根 | 随从 | `laureate-nicolaas-bloembergen` | 6★ | 现代 | `laureate-nicolaas-bloembergen-skill` | Nicolaas Bloembergen；效果取自「凝聚态」（§3） |
| 1982 | 肯尼斯·威尔逊 | 随从 | `laureate-kenneth-g-wilson` | 6★ | 现代 | `laureate-kenneth-g-wilson-skill` | Kenneth G. Wilson；效果取自「天体运行」（§3） |
| 1983 | 苏布拉马尼扬·钱德拉塞卡 | 随从 | `laureate-subrahmanyan-chandrasekhar` | 6★ | 现代 | `laureate-subrahmanyan-chandrasekhar-skill` | Subrahmanyan Chandrasekhar；效果取自「精密测量」（§3） |
| 1983 | 威廉·福勒 | 随从 | `laureate-william-a-fowler` | 6★ | 现代 | `laureate-william-a-fowler-skill` | William A. Fowler；效果取自「辐射」（§3） |
| 1984 | 卡洛·鲁比亚 | 随从 | `laureate-carlo-rubbia` | 6★ | 现代 | `laureate-carlo-rubbia-skill` | Carlo Rubbia；效果取自「量子跃迁」（§3） |
| 1984 | 西蒙·范德梅尔 | 随从 | `laureate-simon-van-der-meer` | 6★ | 现代 | `laureate-simon-van-der-meer-skill` | Simon van der Meer；效果取自「相对性」（§3） |
| 1985 | 克劳斯·冯·克利青 | 随从 | `laureate-klaus-von-klitzing` | 6★ | 现代 | `laureate-klaus-von-klitzing-skill` | Klaus von Klitzing；效果取自「凝聚态」（§3） |
| 1986 | 恩斯特·鲁斯卡 | 随从 | `laureate-ernst-ruska` | 6★ | 现代 | `laureate-ernst-ruska-skill` | Ernst Ruska；效果取自「天体运行」（§3） |
| 1986 | 格尔德·宾宁 | 随从 | `laureate-gerd-binnig` | 6★ | 现代 | `laureate-gerd-binnig-skill` | Gerd Binnig；效果取自「精密测量」（§3） |
| 1986 | 海因里希·罗雷尔 | 随从 | `laureate-heinrich-rohrer` | 6★ | 现代 | `laureate-heinrich-rohrer-skill` | Heinrich Rohrer；效果取自「辐射」（§3） |
| 1987 | 格奥尔格·贝德诺尔茨 | 随从 | `laureate-j-georg-bednorz` | 6★ | 现代 | `laureate-j-georg-bednorz-skill` | J. Georg Bednorz；效果取自「量子跃迁」（§3） |
| 1987 | 卡尔·亚历山大·米勒 | 随从 | `laureate-k-alex-muller` | 6★ | 现代 | `laureate-k-alex-muller-skill` | K. Alex Müller；效果取自「相对性」（§3） |
| 1988 | 杰克·施泰因贝格尔 | 随从 | `laureate-jack-steinberger` | 6★ | 现代 | `laureate-jack-steinberger-skill` | Jack Steinberger；效果取自「凝聚态」（§3） |
| 1988 | 利昂·莱德曼 | 随从 | `laureate-leon-m-lederman` | 6★ | 现代 | `laureate-leon-m-lederman-skill` | Leon M. Lederman；效果取自「天体运行」（§3） |
| 1988 | 梅尔文·施瓦茨 | 随从 | `laureate-melvin-schwartz` | 6★ | 现代 | `laureate-melvin-schwartz-skill` | Melvin Schwartz；效果取自「精密测量」（§3） |
| 1989 | 汉斯·德默尔特 | 随从 | `laureate-hans-g-dehmelt` | 6★ | 现代 | `laureate-hans-g-dehmelt-skill` | Hans G. Dehmelt；效果取自「辐射」（§3） |
| 1989 | 诺曼·拉姆齐 | 随从 | `laureate-norman-f-ramsey` | 6★ | 现代 | `laureate-norman-f-ramsey-skill` | Norman F. Ramsey；效果取自「量子跃迁」（§3） |
| 1989 | 沃尔夫冈·保罗 | 随从 | `laureate-wolfgang-paul` | 6★ | 现代 | `laureate-wolfgang-paul-skill` | Wolfgang Paul；效果取自「相对性」（§3） |
| 1990 | 亨利·肯德尔 | 随从 | `laureate-henry-w-kendall` | 6★ | 现代 | `laureate-henry-w-kendall-skill` | Henry W. Kendall；效果取自「凝聚态」（§3） |
| 1990 | 杰罗姆·弗里德曼 | 随从 | `laureate-jerome-i-friedman` | 6★ | 现代 | `laureate-jerome-i-friedman-skill` | Jerome I. Friedman；效果取自「天体运行」（§3） |
| 1990 | 理查德·泰勒 | 随从 | `laureate-richard-e-taylor` | 6★ | 现代 | `laureate-richard-e-taylor-skill` | Richard E. Taylor；效果取自「精密测量」（§3） |
| 1991 | 皮埃尔-吉勒·德热纳 | 随从 | `laureate-pierre-gilles-de-gennes` | 6★ | 现代 | `laureate-pierre-gilles-de-gennes-skill` | Pierre-Gilles de Gennes；效果取自「辐射」（§3） |
| 1992 | 乔治·夏帕克 | 随从 | `laureate-georges-charpak` | 6★ | 现代 | `laureate-georges-charpak-skill` | Georges Charpak；效果取自「量子跃迁」（§3） |
| 1993 | 小约瑟夫·泰勒 | 随从 | `laureate-joseph-h-taylor-jr` | 6★ | 现代 | `laureate-joseph-h-taylor-jr-skill` | Joseph H. Taylor Jr.；效果取自「相对性」（§3） |
| 1993 | 拉塞尔·赫尔斯 | 随从 | `laureate-russell-a-hulse` | 6★ | 现代 | `laureate-russell-a-hulse-skill` | Russell A. Hulse；效果取自「凝聚态」（§3） |
| 1994 | 伯特伦·布罗克豪斯 | 随从 | `laureate-bertram-n-brockhouse` | 6★ | 现代 | `laureate-bertram-n-brockhouse-skill` | Bertram N. Brockhouse；效果取自「天体运行」（§3） |
| 1994 | 克利福德·沙尔 | 随从 | `laureate-clifford-g-shull` | 6★ | 现代 | `laureate-clifford-g-shull-skill` | Clifford G. Shull；效果取自「精密测量」（§3） |
| 1995 | 弗雷德里克·莱因斯 | 随从 | `laureate-frederick-reines` | 6★ | 现代 | `laureate-frederick-reines-skill` | Frederick Reines；效果取自「辐射」（§3） |
| 1995 | 马丁·佩尔 | 随从 | `laureate-martin-l-perl` | 6★ | 现代 | `laureate-martin-l-perl-skill` | Martin L. Perl；效果取自「量子跃迁」（§3） |
| 1996 | 戴维·李 | 随从 | `laureate-david-m-lee` | 6★ | 现代 | `laureate-david-m-lee-skill` | David M. Lee；效果取自「相对性」（§3） |
| 1996 | 道格拉斯·奥谢罗夫 | 随从 | `laureate-douglas-d-osheroff` | 6★ | 现代 | `laureate-douglas-d-osheroff-skill` | Douglas D. Osheroff；效果取自「凝聚态」（§3） |
| 1996 | 罗伯特·理查森 | 随从 | `laureate-robert-c-richardson` | 6★ | 现代 | `laureate-robert-c-richardson-skill` | Robert C. Richardson；效果取自「天体运行」（§3） |
| 1997 | 克洛德·科昂-唐努德日 | 随从 | `laureate-claude-cohen-tannoudji` | 6★ | 现代 | `laureate-claude-cohen-tannoudji-skill` | Claude Cohen-Tannoudji；效果取自「精密测量」（§3） |
| 1997 | 朱棣文 | 随从 | `laureate-steven-chu` | 6★ | 现代 | `laureate-steven-chu-skill` | Steven Chu；效果取自「辐射」（§3） |
| 1997 | 威廉·菲利普斯 | 随从 | `laureate-william-d-phillips` | 6★ | 现代 | `laureate-william-d-phillips-skill` | William D. Phillips；效果取自「量子跃迁」（§3） |
| 1998 | 崔琦 | 随从 | `laureate-daniel-c-tsui` | 6★ | 现代 | `laureate-daniel-c-tsui-skill` | Daniel C. Tsui；效果取自「相对性」（§3） |
| 1998 | 霍斯特·施特默 | 随从 | `laureate-horst-l-stormer` | 6★ | 现代 | `laureate-horst-l-stormer-skill` | Horst L. Störmer；效果取自「凝聚态」（§3） |
| 1998 | 罗伯特·劳克林 | 随从 | `laureate-robert-b-laughlin` | 6★ | 现代 | `laureate-robert-b-laughlin-skill` | Robert B. Laughlin；效果取自「天体运行」（§3） |
| 1999 | 杰拉德·特霍夫特 | 随从 | `laureate-gerardus-t-hooft` | 6★ | 现代 | `laureate-gerardus-t-hooft-skill` | Gerardus 't Hooft；效果取自「精密测量」（§3） |
| 1999 | 马丁努斯·费尔特曼 | 随从 | `laureate-martinus-j-g-veltman` | 6★ | 现代 | `laureate-martinus-j-g-veltman-skill` | Martinus J.G. Veltman；效果取自「辐射」（§3） |
| 2000 | 赫伯特·克勒默 | 随从 | `laureate-herbert-kroemer` | 7★ | 未来 | `laureate-herbert-kroemer-skill` | Herbert Kroemer；效果取自「量子跃迁」（§3） |
| 2000 | 杰克·基尔比 | 随从 | `laureate-jack-kilby` | 7★ | 未来 | `laureate-jack-kilby-skill` | Jack Kilby；效果取自「相对性」（§3） |
| 2000 | 若列斯·阿尔费罗夫 | 随从 | `laureate-zhores-alferov` | 7★ | 未来 | `laureate-zhores-alferov-skill` | Zhores Alferov；效果取自「凝聚态」（§3） |
| 2001 | 卡尔·维曼 | 随从 | `laureate-carl-wieman` | 7★ | 未来 | `laureate-carl-wieman-skill` | Carl Wieman；效果取自「天体运行」（§3） |
| 2001 | 埃里克·康奈尔 | 随从 | `laureate-eric-cornell` | 7★ | 未来 | `laureate-eric-cornell-skill` | Eric Cornell；效果取自「精密测量」（§3） |
| 2001 | 沃尔夫冈·克特勒 | 随从 | `laureate-wolfgang-ketterle` | 7★ | 未来 | `laureate-wolfgang-ketterle-skill` | Wolfgang Ketterle；效果取自「辐射」（§3） |
| 2002 | 小柴昌俊 | 随从 | `laureate-masatoshi-koshiba` | 7★ | 未来 | `laureate-masatoshi-koshiba-skill` | Masatoshi Koshiba；效果取自「量子跃迁」（§3） |
| 2002 | 小雷蒙德·戴维斯 | 随从 | `laureate-raymond-davis-jr` | 7★ | 未来 | `laureate-raymond-davis-jr-skill` | Raymond Davis Jr.；效果取自「相对性」（§3） |
| 2002 | 里卡尔多·贾科尼 | 随从 | `laureate-riccardo-giacconi` | 7★ | 未来 | `laureate-riccardo-giacconi-skill` | Riccardo Giacconi；效果取自「凝聚态」（§3） |
| 2003 | 阿列克谢·阿布里科索夫 | 随从 | `laureate-alexei-abrikosov` | 7★ | 未来 | `laureate-alexei-abrikosov-skill` | Alexei Abrikosov；效果取自「天体运行」（§3） |
| 2003 | 安东尼·莱格特 | 随从 | `laureate-anthony-j-leggett` | 7★ | 未来 | `laureate-anthony-j-leggett-skill` | Anthony J. Leggett；效果取自「精密测量」（§3） |
| 2003 | 维塔利·金兹堡 | 随从 | `laureate-vitaly-l-ginzburg` | 7★ | 未来 | `laureate-vitaly-l-ginzburg-skill` | Vitaly L. Ginzburg；效果取自「辐射」（§3） |
| 2004 | 戴维·格罗斯 | 随从 | `laureate-david-j-gross` | 7★ | 未来 | `laureate-david-j-gross-skill` | David J. Gross；效果取自「量子跃迁」（§3） |
| 2004 | 弗兰克·维尔切克 | 随从 | `laureate-frank-wilczek` | 7★ | 未来 | `laureate-frank-wilczek-skill` | Frank Wilczek；效果取自「相对性」（§3） |
| 2004 | 休·戴维·波利策 | 随从 | `laureate-h-david-politzer` | 7★ | 未来 | `laureate-h-david-politzer-skill` | H. David Politzer；效果取自「凝聚态」（§3） |
| 2005 | 约翰·霍尔 | 随从 | `laureate-john-l-hall` | 7★ | 未来 | `laureate-john-l-hall-skill` | John L. Hall；效果取自「天体运行」（§3） |
| 2005 | 罗伊·格劳伯 | 随从 | `laureate-roy-j-glauber` | 7★ | 未来 | `laureate-roy-j-glauber-skill` | Roy J. Glauber；效果取自「精密测量」（§3） |
| 2005 | 特奥多尔·亨施 | 随从 | `laureate-theodor-w-hansch` | 7★ | 未来 | `laureate-theodor-w-hansch-skill` | Theodor W. Hänsch；效果取自「辐射」（§3） |
| 2006 | 乔治·斯穆特 | 随从 | `laureate-george-f-smoot` | 7★ | 未来 | `laureate-george-f-smoot-skill` | George F. Smoot；效果取自「量子跃迁」（§3） |
| 2006 | 约翰·马瑟 | 随从 | `laureate-john-c-mather` | 7★ | 未来 | `laureate-john-c-mather-skill` | John C. Mather；效果取自「相对性」（§3） |
| 2007 | 阿尔贝·费尔 | 随从 | `laureate-albert-fert` | 7★ | 未来 | `laureate-albert-fert-skill` | Albert Fert；效果取自「凝聚态」（§3） |
| 2007 | 彼得·格林贝格 | 随从 | `laureate-peter-grunberg` | 7★ | 未来 | `laureate-peter-grunberg-skill` | Peter Grünberg；效果取自「天体运行」（§3） |
| 2008 | 小林诚 | 随从 | `laureate-makoto-kobayashi` | 7★ | 未来 | `laureate-makoto-kobayashi-skill` | Makoto Kobayashi；效果取自「精密测量」（§3） |
| 2008 | 益川敏英 | 随从 | `laureate-toshihide-maskawa` | 7★ | 未来 | `laureate-toshihide-maskawa-skill` | Toshihide Maskawa；效果取自「辐射」（§3） |
| 2008 | 南部阳一郎 | 随从 | `laureate-yoichiro-nambu` | 7★ | 未来 | `laureate-yoichiro-nambu-skill` | Yoichiro Nambu；效果取自「量子跃迁」（§3） |
| 2009 | 高锟 | 随从 | `laureate-charles-k-kao` | 7★ | 未来 | `laureate-charles-k-kao-skill` | Charles K. Kao；效果取自「相对性」（§3） |
| 2009 | 乔治·史密斯（物理） | 随从 | `laureate-george-e-smith` | 7★ | 未来 | `laureate-george-e-smith-skill` | George E. Smith；效果取自「凝聚态」（§3） |
| 2009 | 威拉德·博伊尔 | 随从 | `laureate-willard-s-boyle` | 7★ | 未来 | `laureate-willard-s-boyle-skill` | Willard S. Boyle；效果取自「天体运行」（§3） |
| 2010 | 安德烈·海姆 | 随从 | `laureate-andre-geim` | 7★ | 未来 | `laureate-andre-geim-skill` | Andre Geim；效果取自「精密测量」（§3） |
| 2010 | 康斯坦丁·诺沃肖洛夫 | 随从 | `laureate-konstantin-novoselov` | 7★ | 未来 | `laureate-konstantin-novoselov-skill` | Konstantin Novoselov；效果取自「辐射」（§3） |
| 2011 | 亚当·里斯 | 随从 | `laureate-adam-g-riess` | 7★ | 未来 | `laureate-adam-g-riess-skill` | Adam G. Riess；效果取自「量子跃迁」（§3） |
| 2011 | 布赖恩·施密特 | 随从 | `laureate-brian-p-schmidt` | 7★ | 未来 | `laureate-brian-p-schmidt-skill` | Brian P. Schmidt；效果取自「相对性」（§3） |
| 2011 | 索尔·珀尔马特 | 随从 | `laureate-saul-perlmutter` | 7★ | 未来 | `laureate-saul-perlmutter-skill` | Saul Perlmutter；效果取自「凝聚态」（§3） |
| 2012 | 戴维·瓦恩兰 | 随从 | `laureate-david-j-wineland` | 7★ | 未来 | `laureate-david-j-wineland-skill` | David J. Wineland；效果取自「天体运行」（§3） |
| 2012 | 塞尔日·阿罗什 | 随从 | `laureate-serge-haroche` | 7★ | 未来 | `laureate-serge-haroche-skill` | Serge Haroche；效果取自「精密测量」（§3） |
| 2013 | 弗朗索瓦·恩格勒 | 随从 | `laureate-francois-englert` | 7★ | 未来 | `laureate-francois-englert-skill` | François Englert；效果取自「辐射」（§3） |
| 2013 | 彼得·希格斯 | 随从 | `laureate-peter-higgs` | 7★ | 未来 | `laureate-peter-higgs-skill` | Peter Higgs；效果取自「量子跃迁」（§3） |
| 2014 | 天野浩 | 随从 | `laureate-hiroshi-amano` | 7★ | 未来 | `laureate-hiroshi-amano-skill` | Hiroshi Amano；效果取自「相对性」（§3） |
| 2014 | 赤崎勇 | 随从 | `laureate-isamu-akasaki` | 7★ | 未来 | `laureate-isamu-akasaki-skill` | Isamu Akasaki；效果取自「凝聚态」（§3） |
| 2014 | 中村修二 | 随从 | `laureate-shuji-nakamura` | 7★ | 未来 | `laureate-shuji-nakamura-skill` | Shuji Nakamura；效果取自「天体运行」（§3） |
| 2015 | 阿瑟·麦克唐纳 | 随从 | `laureate-arthur-b-mcdonald` | 7★ | 未来 | `laureate-arthur-b-mcdonald-skill` | Arthur B. McDonald；效果取自「精密测量」（§3） |
| 2015 | 梶田隆章 | 随从 | `laureate-takaaki-kajita` | 7★ | 未来 | `laureate-takaaki-kajita-skill` | Takaaki Kajita；效果取自「辐射」（§3） |
| 2016 | 戴维·索利斯 | 随从 | `laureate-david-j-thouless` | 7★ | 未来 | `laureate-david-j-thouless-skill` | David J. Thouless；效果取自「量子跃迁」（§3） |
| 2016 | 邓肯·霍尔丹 | 随从 | `laureate-f-duncan-m-haldane` | 7★ | 未来 | `laureate-f-duncan-m-haldane-skill` | F. Duncan M. Haldane；效果取自「相对性」（§3） |
| 2016 | 迈克尔·科斯特利茨 | 随从 | `laureate-j-michael-kosterlitz` | 7★ | 未来 | `laureate-j-michael-kosterlitz-skill` | J. Michael Kosterlitz；效果取自「凝聚态」（§3） |
| 2017 | 巴里·巴里什 | 随从 | `laureate-barry-c-barish` | 7★ | 未来 | `laureate-barry-c-barish-skill` | Barry C. Barish；效果取自「天体运行」（§3） |
| 2017 | 基普·索恩 | 随从 | `laureate-kip-s-thorne` | 7★ | 未来 | `laureate-kip-s-thorne-skill` | Kip S. Thorne；效果取自「精密测量」（§3） |
| 2017 | 雷纳·韦斯 | 随从 | `laureate-rainer-weiss` | 7★ | 未来 | `laureate-rainer-weiss-skill` | Rainer Weiss；效果取自「辐射」（§3） |
| 2018 | 阿瑟·阿什金 | 随从 | `laureate-arthur-ashkin` | 7★ | 未来 | `laureate-arthur-ashkin-skill` | Arthur Ashkin；效果取自「量子跃迁」（§3） |
| 2018 | 唐娜·斯特里克兰 | 随从 | `laureate-donna-strickland` | 7★ | 未来 | `laureate-donna-strickland-skill` | Donna Strickland；效果取自「相对性」（§3） |
| 2018 | 热拉尔·穆鲁 | 随从 | `laureate-gerard-mourou` | 7★ | 未来 | `laureate-gerard-mourou-skill` | Gérard Mourou；效果取自「凝聚态」（§3） |
| 2019 | 迪迪埃·奎洛兹 | 随从 | `laureate-didier-queloz` | 7★ | 未来 | `laureate-didier-queloz-skill` | Didier Queloz；效果取自「天体运行」（§3） |
| 2019 | 詹姆斯·皮布尔斯 | 随从 | `laureate-james-peebles` | 7★ | 未来 | `laureate-james-peebles-skill` | James Peebles；效果取自「精密测量」（§3） |
| 2019 | 米歇尔·马约尔 | 随从 | `laureate-michel-mayor` | 7★ | 未来 | `laureate-michel-mayor-skill` | Michel Mayor；效果取自「辐射」（§3） |
| 2020 | 安德烈娅·盖兹 | 随从 | `laureate-andrea-ghez` | 7★ | 未来 | `laureate-andrea-ghez-skill` | Andrea Ghez；效果取自「量子跃迁」（§3） |
| 2020 | 赖因哈德·根策尔 | 随从 | `laureate-reinhard-genzel` | 7★ | 未来 | `laureate-reinhard-genzel-skill` | Reinhard Genzel；效果取自「相对性」（§3） |
| 2020 | 罗杰·彭罗斯 | 随从 | `laureate-roger-penrose` | 7★ | 未来 | `laureate-roger-penrose-skill` | Roger Penrose；效果取自「凝聚态」（§3） |
| 2021 | 乔治·帕里西 | 随从 | `laureate-giorgio-parisi` | 7★ | 未来 | `laureate-giorgio-parisi-skill` | Giorgio Parisi；效果取自「天体运行」（§3） |
| 2021 | 克劳斯·哈塞尔曼 | 随从 | `laureate-klaus-hasselmann` | 7★ | 未来 | `laureate-klaus-hasselmann-skill` | Klaus Hasselmann；效果取自「精密测量」（§3） |
| 2021 | 真锅淑郎 | 随从 | `laureate-syukuro-manabe` | 7★ | 未来 | `laureate-syukuro-manabe-skill` | Syukuro Manabe；效果取自「辐射」（§3） |
| 2022 | 阿兰·阿斯佩 | 随从 | `laureate-alain-aspect` | 7★ | 未来 | `laureate-alain-aspect-skill` | Alain Aspect；效果取自「量子跃迁」（§3） |
| 2022 | 安东·蔡林格 | 随从 | `laureate-anton-zeilinger` | 7★ | 未来 | `laureate-anton-zeilinger-skill` | Anton Zeilinger；效果取自「相对性」（§3） |
| 2022 | 约翰·克劳泽 | 随从 | `laureate-john-clauser` | 7★ | 未来 | `laureate-john-clauser-skill` | John Clauser；效果取自「凝聚态」（§3） |
| 2023 | 安妮·吕利耶 | 随从 | `laureate-anne-lhuillier` | 7★ | 未来 | `laureate-anne-lhuillier-skill` | Anne L’Huillier；效果取自「天体运行」（§3） |
| 2023 | 费伦茨·克劳斯 | 随从 | `laureate-ferenc-krausz` | 7★ | 未来 | `laureate-ferenc-krausz-skill` | Ferenc Krausz；效果取自「精密测量」（§3） |
| 2023 | 皮埃尔·阿戈斯蒂尼 | 随从 | `laureate-pierre-agostini` | 7★ | 未来 | `laureate-pierre-agostini-skill` | Pierre Agostini；效果取自「辐射」（§3） |
| 2024 | 杰弗里·辛顿 | 随从 | `laureate-geoffrey-hinton` | 7★ | 未来 | `laureate-geoffrey-hinton-skill` | Geoffrey Hinton；效果取自「量子跃迁」（§3） |
| 2024 | 约翰·霍普菲尔德 | 随从 | `laureate-john-j-hopfield` | 7★ | 未来 | `laureate-john-j-hopfield-skill` | John J. Hopfield；效果取自「相对性」（§3） |
| 2025 | 约翰·克拉克 | 随从 | `laureate-john-clarke` | 7★ | 未来 | `laureate-john-clarke-skill` | John Clarke；效果取自「凝聚态」（§3） |
| 2025 | 约翰·马丁尼斯 | 随从 | `laureate-john-m-martinis` | 7★ | 未来 | `laureate-john-m-martinis-skill` | John M. Martinis；效果取自「天体运行」（§3） |
| 2025 | 米歇尔·德沃雷 | 随从 | `laureate-michel-h-devoret` | 7★ | 未来 | `laureate-michel-h-devoret-skill` | Michel H. Devoret；效果取自「精密测量」（§3） |
| 2026 | 弗朗西斯·哈尔岑 | 随从 | `laureate-francis-halzen` | 7★ | 未来 | `laureate-francis-halzen-skill` | Francis Halzen；效果取自「辐射」（§3） |

#### 5.2 化学奖（199）

| 年份 | 得主 | 类型 | 卡牌 id | 星级/费用 | 时代 | 技能 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1901 | 雅各布斯·范特霍夫 | 随从 | `laureate-jacobus-h-van-t-hoff` | 4★ | 工业 | `laureate-jacobus-h-van-t-hoff-skill` | Jacobus H. van 't Hoff；效果取自「合成」（§3） |
| 1902 | 埃米尔·费歇尔 | 随从 | `laureate-emil-fischer` | 4★ | 工业 | `laureate-emil-fischer-skill` | Emil Fischer；效果取自「催化」（§3） |
| 1903 | 斯万特·阿伦尼乌斯 | 随从 | `laureate-svante-arrhenius` | 4★ | 工业 | `laureate-svante-arrhenius-skill` | Svante Arrhenius；效果取自「结构」（§3） |
| 1904 | 威廉·拉姆齐 | 随从 | `laureate-sir-william-ramsay` | 4★ | 工业 | `laureate-sir-william-ramsay-skill` | Sir William Ramsay；效果取自「分析」（§3） |
| 1905 | 阿道夫·冯·拜尔 | 随从 | `laureate-adolf-von-baeyer` | 4★ | 工业 | `laureate-adolf-von-baeyer-skill` | Adolf von Baeyer；效果取自「材料」（§3） |
| 1906 | 亨利·莫瓦桑 | 随从 | `laureate-henri-moissan` | 4★ | 工业 | `laureate-henri-moissan-skill` | Henri Moissan；效果取自「提纯」（§3） |
| 1907 | 爱德华·布赫纳 | 随从 | `laureate-eduard-buchner` | 4★ | 工业 | `laureate-eduard-buchner-skill` | Eduard Buchner；效果取自「合成」（§3） |
| 1908 | 欧内斯特·卢瑟福 | 现有随从 | `rutherford` | 6★ | 现代 | `[combat-random-enemy-damage-3]` | 复用现有卡，不新增 |
| 1909 | 威廉·奥斯特瓦尔德 | 随从 | `laureate-wilhelm-ostwald` | 4★ | 工业 | `laureate-wilhelm-ostwald-skill` | Wilhelm Ostwald；效果取自「催化」（§3） |
| 1910 | 奥托·瓦拉赫 | 随从 | `laureate-otto-wallach` | 4★ | 工业 | `laureate-otto-wallach-skill` | Otto Wallach；效果取自「结构」（§3） |
| 1912 | 保罗·萨巴捷 | 随从 | `laureate-paul-sabatier` | 4★ | 工业 | `laureate-paul-sabatier-skill` | Paul Sabatier；效果取自「分析」（§3） |
| 1912 | 维克托·格林尼亚 | 随从 | `laureate-victor-grignard` | 4★ | 工业 | `laureate-victor-grignard-skill` | Victor Grignard；效果取自「材料」（§3） |
| 1913 | 阿尔弗雷德·维尔纳 | 随从 | `laureate-alfred-werner` | 4★ | 工业 | `laureate-alfred-werner-skill` | Alfred Werner；效果取自「提纯」（§3） |
| 1914 | 西奥多·理查兹 | 随从 | `laureate-theodore-w-richards` | 4★ | 工业 | `laureate-theodore-w-richards-skill` | Theodore W. Richards；效果取自「合成」（§3） |
| 1915 | 理查德·维尔施泰特 | 随从 | `laureate-richard-willstatter` | 4★ | 工业 | `laureate-richard-willstatter-skill` | Richard Willstätter；效果取自「催化」（§3） |
| 1918 | 弗里茨·哈伯 | 随从 | `laureate-fritz-haber` | 4★ | 工业 | `laureate-fritz-haber-skill` | Fritz Haber；效果取自「结构」（§3） |
| 1920 | 瓦尔特·能斯特 | 随从 | `laureate-walther-nernst` | 4★ | 工业 | `laureate-walther-nernst-skill` | Walther Nernst；效果取自「分析」（§3） |
| 1921 | 弗雷德里克·索迪 | 随从 | `laureate-frederick-soddy` | 4★ | 工业 | `laureate-frederick-soddy-skill` | Frederick Soddy；效果取自「材料」（§3） |
| 1922 | 弗朗西斯·阿斯顿 | 随从 | `laureate-francis-w-aston` | 4★ | 工业 | `laureate-francis-w-aston-skill` | Francis W. Aston；效果取自「提纯」（§3） |
| 1923 | 弗里茨·普雷格尔 | 随从 | `laureate-fritz-pregl` | 4★ | 工业 | `laureate-fritz-pregl-skill` | Fritz Pregl；效果取自「合成」（§3） |
| 1925 | 理查德·席格蒙迪 | 随从 | `laureate-richard-zsigmondy` | 4★ | 工业 | `laureate-richard-zsigmondy-skill` | Richard Zsigmondy；效果取自「催化」（§3） |
| 1926 | 西奥多·斯韦德贝里 | 随从 | `laureate-the-svedberg` | 4★ | 工业 | `laureate-the-svedberg-skill` | The Svedberg；效果取自「结构」（§3） |
| 1927 | 海因里希·维兰德 | 随从 | `laureate-heinrich-wieland` | 4★ | 工业 | `laureate-heinrich-wieland-skill` | Heinrich Wieland；效果取自「分析」（§3） |
| 1928 | 阿道夫·温道斯 | 随从 | `laureate-adolf-windaus` | 4★ | 工业 | `laureate-adolf-windaus-skill` | Adolf Windaus；效果取自「材料」（§3） |
| 1929 | 阿瑟·哈登 | 随从 | `laureate-arthur-harden` | 4★ | 工业 | `laureate-arthur-harden-skill` | Arthur Harden；效果取自「提纯」（§3） |
| 1929 | 汉斯·冯·奥伊勒-切尔平 | 随从 | `laureate-hans-von-euler-chelpin` | 4★ | 工业 | `laureate-hans-von-euler-chelpin-skill` | Hans von Euler-Chelpin；效果取自「合成」（§3） |
| 1930 | 汉斯·费歇尔 | 随从 | `laureate-hans-fischer` | 4★ | 工业 | `laureate-hans-fischer-skill` | Hans Fischer；效果取自「催化」（§3） |
| 1931 | 卡尔·博施 | 随从 | `laureate-carl-bosch` | 4★ | 工业 | `laureate-carl-bosch-skill` | Carl Bosch；效果取自「结构」（§3） |
| 1931 | 弗里德里希·贝吉乌斯 | 随从 | `laureate-friedrich-bergius` | 4★ | 工业 | `laureate-friedrich-bergius-skill` | Friedrich Bergius；效果取自「分析」（§3） |
| 1932 | 欧文·朗缪尔 | 随从 | `laureate-irving-langmuir` | 4★ | 工业 | `laureate-irving-langmuir-skill` | Irving Langmuir；效果取自「材料」（§3） |
| 1934 | 哈罗德·尤里 | 随从 | `laureate-harold-c-urey` | 4★ | 工业 | `laureate-harold-c-urey-skill` | Harold C. Urey；效果取自「提纯」（§3） |
| 1935 | 弗雷德里克·约里奥 | 随从 | `laureate-frederic-joliot` | 4★ | 工业 | `laureate-frederic-joliot-skill` | Frédéric Joliot；效果取自「合成」（§3） |
| 1935 | 伊雷娜·约里奥-居里 | 随从 | `laureate-irene-joliot-curie` | 4★ | 工业 | `laureate-irene-joliot-curie-skill` | Irène Joliot-Curie；效果取自「催化」（§3） |
| 1936 | 彼得·德拜 | 随从 | `laureate-peter-debye` | 4★ | 工业 | `laureate-peter-debye-skill` | Peter Debye；效果取自「结构」（§3） |
| 1937 | 诺曼·霍沃思 | 随从 | `laureate-norman-haworth` | 4★ | 工业 | `laureate-norman-haworth-skill` | Norman Haworth；效果取自「分析」（§3） |
| 1937 | 保罗·卡勒 | 随从 | `laureate-paul-karrer` | 4★ | 工业 | `laureate-paul-karrer-skill` | Paul Karrer；效果取自「材料」（§3） |
| 1938 | 理查德·库恩 | 随从 | `laureate-richard-kuhn` | 4★ | 工业 | `laureate-richard-kuhn-skill` | Richard Kuhn；效果取自「提纯」（§3） |
| 1939 | 阿道夫·布特南特 | 随从 | `laureate-adolf-butenandt` | 4★ | 工业 | `laureate-adolf-butenandt-skill` | Adolf Butenandt；效果取自「合成」（§3） |
| 1939 | 利奥波德·鲁日奇卡 | 随从 | `laureate-leopold-ruzicka` | 4★ | 工业 | `laureate-leopold-ruzicka-skill` | Leopold Ruzicka；效果取自「催化」（§3） |
| 1943 | 乔治·德海韦西 | 随从 | `laureate-george-de-hevesy` | 5★ | 工业 | `laureate-george-de-hevesy-skill` | George de Hevesy；效果取自「结构」（§3） |
| 1944 | 奥托·哈恩 | 随从 | `laureate-otto-hahn` | 5★ | 工业 | `laureate-otto-hahn-skill` | Otto Hahn；效果取自「分析」（§3） |
| 1945 | 阿尔图里·维尔塔宁 | 随从 | `laureate-artturi-virtanen` | 5★ | 工业 | `laureate-artturi-virtanen-skill` | Artturi Virtanen；效果取自「材料」（§3） |
| 1946 | 詹姆斯·萨姆纳 | 随从 | `laureate-james-b-sumner` | 5★ | 现代 | `laureate-james-b-sumner-skill` | James B. Sumner；效果取自「提纯」（§3） |
| 1946 | 约翰·诺思罗普 | 随从 | `laureate-john-h-northrop` | 5★ | 现代 | `laureate-john-h-northrop-skill` | John H. Northrop；效果取自「合成」（§3） |
| 1946 | 温德尔·斯坦利 | 随从 | `laureate-wendell-m-stanley` | 5★ | 现代 | `laureate-wendell-m-stanley-skill` | Wendell M. Stanley；效果取自「催化」（§3） |
| 1947 | 罗伯特·鲁宾逊 | 随从 | `laureate-sir-robert-robinson` | 5★ | 现代 | `laureate-sir-robert-robinson-skill` | Sir Robert Robinson；效果取自「结构」（§3） |
| 1948 | 阿尔内·蒂塞利乌斯 | 随从 | `laureate-arne-tiselius` | 5★ | 现代 | `laureate-arne-tiselius-skill` | Arne Tiselius；效果取自「分析」（§3） |
| 1949 | 威廉·吉奥克 | 随从 | `laureate-william-f-giauque` | 5★ | 现代 | `laureate-william-f-giauque-skill` | William F. Giauque；效果取自「材料」（§3） |
| 1950 | 库尔特·阿尔德 | 随从 | `laureate-kurt-alder` | 5★ | 现代 | `laureate-kurt-alder-skill` | Kurt Alder；效果取自「提纯」（§3） |
| 1950 | 奥托·狄尔斯 | 随从 | `laureate-otto-diels` | 5★ | 现代 | `laureate-otto-diels-skill` | Otto Diels；效果取自「合成」（§3） |
| 1951 | 埃德温·麦克米伦 | 随从 | `laureate-edwin-m-mcmillan` | 5★ | 现代 | `laureate-edwin-m-mcmillan-skill` | Edwin M. McMillan；效果取自「催化」（§3） |
| 1951 | 格伦·西博格 | 随从 | `laureate-glenn-t-seaborg` | 5★ | 现代 | `laureate-glenn-t-seaborg-skill` | Glenn T. Seaborg；效果取自「结构」（§3） |
| 1952 | 阿彻·马丁 | 随从 | `laureate-archer-j-p-martin` | 5★ | 现代 | `laureate-archer-j-p-martin-skill` | Archer J.P. Martin；效果取自「分析」（§3） |
| 1952 | 理查德·辛格 | 随从 | `laureate-richard-l-m-synge` | 5★ | 现代 | `laureate-richard-l-m-synge-skill` | Richard L.M. Synge；效果取自「材料」（§3） |
| 1953 | 赫尔曼·施陶丁格 | 随从 | `laureate-hermann-staudinger` | 5★ | 现代 | `laureate-hermann-staudinger-skill` | Hermann Staudinger；效果取自「提纯」（§3） |
| 1954 | 莱纳斯·鲍林 | 随从 | `laureate-linus-pauling` | 5★ | 现代 | `laureate-linus-pauling-skill` | Linus Pauling；效果取自「合成」（§3）；1954 化学奖、1962 和平奖 |
| 1955 | 文森特·迪维尼奥 | 随从 | `laureate-vincent-du-vigneaud` | 5★ | 现代 | `laureate-vincent-du-vigneaud-skill` | Vincent du Vigneaud；效果取自「催化」（§3） |
| 1956 | 尼古拉·谢苗诺夫 | 随从 | `laureate-nikolay-semenov` | 5★ | 现代 | `laureate-nikolay-semenov-skill` | Nikolay Semenov；效果取自「结构」（§3） |
| 1956 | 西里尔·欣谢尔伍德 | 随从 | `laureate-sir-cyril-hinshelwood` | 5★ | 现代 | `laureate-sir-cyril-hinshelwood-skill` | Sir Cyril Hinshelwood；效果取自「分析」（§3） |
| 1957 | 托德勋爵 | 随从 | `laureate-lord-todd` | 5★ | 现代 | `laureate-lord-todd-skill` | Lord Todd；效果取自「材料」（§3） |
| 1958 | 弗雷德里克·桑格 | 随从 | `laureate-frederick-sanger` | 5★ | 现代 | `laureate-frederick-sanger-skill` | Frederick Sanger；效果取自「提纯」（§3）；1958 化学奖、1980 化学奖 |
| 1959 | 雅罗斯拉夫·海罗夫斯基 | 随从 | `laureate-jaroslav-heyrovsky` | 5★ | 现代 | `laureate-jaroslav-heyrovsky-skill` | Jaroslav Heyrovsky；效果取自「合成」（§3） |
| 1960 | 威拉德·利比 | 随从 | `laureate-willard-f-libby` | 5★ | 现代 | `laureate-willard-f-libby-skill` | Willard F. Libby；效果取自「催化」（§3） |
| 1961 | 梅尔文·卡尔文 | 随从 | `laureate-melvin-calvin` | 5★ | 现代 | `laureate-melvin-calvin-skill` | Melvin Calvin；效果取自「结构」（§3） |
| 1962 | 约翰·肯德鲁 | 随从 | `laureate-john-c-kendrew` | 5★ | 现代 | `laureate-john-c-kendrew-skill` | John C. Kendrew；效果取自「分析」（§3） |
| 1962 | 马克斯·佩鲁茨 | 随从 | `laureate-max-f-perutz` | 5★ | 现代 | `laureate-max-f-perutz-skill` | Max F. Perutz；效果取自「材料」（§3） |
| 1963 | 朱利奥·纳塔 | 随从 | `laureate-giulio-natta` | 5★ | 现代 | `laureate-giulio-natta-skill` | Giulio Natta；效果取自「提纯」（§3） |
| 1963 | 卡尔·齐格勒 | 随从 | `laureate-karl-ziegler` | 5★ | 现代 | `laureate-karl-ziegler-skill` | Karl Ziegler；效果取自「合成」（§3） |
| 1964 | 多萝西·克劳福特·霍奇金 | 随从 | `laureate-dorothy-crowfoot-hodgkin` | 5★ | 现代 | `laureate-dorothy-crowfoot-hodgkin-skill` | Dorothy Crowfoot Hodgkin；效果取自「催化」（§3） |
| 1965 | 罗伯特·伍德沃德 | 随从 | `laureate-robert-b-woodward` | 5★ | 现代 | `laureate-robert-b-woodward-skill` | Robert B. Woodward；效果取自「结构」（§3） |
| 1966 | 罗伯特·马利肯 | 随从 | `laureate-robert-s-mulliken` | 5★ | 现代 | `laureate-robert-s-mulliken-skill` | Robert S. Mulliken；效果取自「分析」（§3） |
| 1967 | 乔治·波特 | 随从 | `laureate-george-porter` | 5★ | 现代 | `laureate-george-porter-skill` | George Porter；效果取自「材料」（§3） |
| 1967 | 曼弗雷德·艾根 | 随从 | `laureate-manfred-eigen` | 5★ | 现代 | `laureate-manfred-eigen-skill` | Manfred Eigen；效果取自「提纯」（§3） |
| 1967 | 罗纳德·诺里什 | 随从 | `laureate-ronald-g-w-norrish` | 5★ | 现代 | `laureate-ronald-g-w-norrish-skill` | Ronald G.W. Norrish；效果取自「合成」（§3） |
| 1968 | 拉斯·昂萨格 | 随从 | `laureate-lars-onsager` | 5★ | 现代 | `laureate-lars-onsager-skill` | Lars Onsager；效果取自「催化」（§3） |
| 1969 | 德里克·巴顿 | 随从 | `laureate-derek-barton` | 5★ | 现代 | `laureate-derek-barton-skill` | Derek Barton；效果取自「结构」（§3） |
| 1969 | 奥德·哈塞尔 | 随从 | `laureate-odd-hassel` | 5★ | 现代 | `laureate-odd-hassel-skill` | Odd Hassel；效果取自「分析」（§3） |
| 1970 | 路易斯·莱洛伊尔 | 随从 | `laureate-luis-leloir` | 6★ | 现代 | `laureate-luis-leloir-skill` | Luis Leloir；效果取自「材料」（§3） |
| 1971 | 格哈德·赫茨伯格 | 随从 | `laureate-gerhard-herzberg` | 6★ | 现代 | `laureate-gerhard-herzberg-skill` | Gerhard Herzberg；效果取自「提纯」（§3） |
| 1972 | 克里斯蒂安·安芬森 | 随从 | `laureate-christian-anfinsen` | 6★ | 现代 | `laureate-christian-anfinsen-skill` | Christian Anfinsen；效果取自「合成」（§3） |
| 1972 | 斯坦福·摩尔 | 随从 | `laureate-stanford-moore` | 6★ | 现代 | `laureate-stanford-moore-skill` | Stanford Moore；效果取自「催化」（§3） |
| 1972 | 威廉·斯坦 | 随从 | `laureate-william-h-stein` | 6★ | 现代 | `laureate-william-h-stein-skill` | William H. Stein；效果取自「结构」（§3） |
| 1973 | 恩斯特·奥托·费歇尔 | 随从 | `laureate-ernst-otto-fischer` | 6★ | 现代 | `laureate-ernst-otto-fischer-skill` | Ernst Otto Fischer；效果取自「分析」（§3） |
| 1973 | 杰弗里·威尔金森 | 随从 | `laureate-geoffrey-wilkinson` | 6★ | 现代 | `laureate-geoffrey-wilkinson-skill` | Geoffrey Wilkinson；效果取自「材料」（§3） |
| 1974 | 保罗·弗洛里 | 随从 | `laureate-paul-j-flory` | 6★ | 现代 | `laureate-paul-j-flory-skill` | Paul J. Flory；效果取自「提纯」（§3） |
| 1975 | 约翰·康福思 | 随从 | `laureate-john-cornforth` | 6★ | 现代 | `laureate-john-cornforth-skill` | John Cornforth；效果取自「合成」（§3） |
| 1975 | 弗拉基米尔·普雷洛格 | 随从 | `laureate-vladimir-prelog` | 6★ | 现代 | `laureate-vladimir-prelog-skill` | Vladimir Prelog；效果取自「催化」（§3） |
| 1976 | 威廉·利普斯科姆 | 随从 | `laureate-william-lipscomb` | 6★ | 现代 | `laureate-william-lipscomb-skill` | William Lipscomb；效果取自「结构」（§3） |
| 1977 | 伊利亚·普里戈金 | 随从 | `laureate-ilya-prigogine` | 6★ | 现代 | `laureate-ilya-prigogine-skill` | Ilya Prigogine；效果取自「分析」（§3） |
| 1978 | 彼得·米切尔 | 随从 | `laureate-peter-mitchell` | 6★ | 现代 | `laureate-peter-mitchell-skill` | Peter Mitchell；效果取自「材料」（§3） |
| 1979 | 格奥尔格·维蒂希 | 随从 | `laureate-georg-wittig` | 6★ | 现代 | `laureate-georg-wittig-skill` | Georg Wittig；效果取自「提纯」（§3） |
| 1979 | 赫伯特·布朗 | 随从 | `laureate-herbert-c-brown` | 6★ | 现代 | `laureate-herbert-c-brown-skill` | Herbert C. Brown；效果取自「合成」（§3） |
| 1980 | 保罗·伯格 | 随从 | `laureate-paul-berg` | 6★ | 现代 | `laureate-paul-berg-skill` | Paul Berg；效果取自「催化」（§3） |
| 1980 | 沃尔特·吉尔伯特 | 随从 | `laureate-walter-gilbert` | 6★ | 现代 | `laureate-walter-gilbert-skill` | Walter Gilbert；效果取自「结构」（§3） |
| 1981 | 福井谦一 | 随从 | `laureate-kenichi-fukui` | 6★ | 现代 | `laureate-kenichi-fukui-skill` | Kenichi Fukui；效果取自「分析」（§3） |
| 1981 | 罗尔德·霍夫曼 | 随从 | `laureate-roald-hoffmann` | 6★ | 现代 | `laureate-roald-hoffmann-skill` | Roald Hoffmann；效果取自「材料」（§3） |
| 1982 | 阿龙·克卢格 | 随从 | `laureate-aaron-klug` | 6★ | 现代 | `laureate-aaron-klug-skill` | Aaron Klug；效果取自「提纯」（§3） |
| 1983 | 亨利·陶布 | 随从 | `laureate-henry-taube` | 6★ | 现代 | `laureate-henry-taube-skill` | Henry Taube；效果取自「合成」（§3） |
| 1984 | 布鲁斯·梅里菲尔德 | 随从 | `laureate-bruce-merrifield` | 6★ | 现代 | `laureate-bruce-merrifield-skill` | Bruce Merrifield；效果取自「催化」（§3） |
| 1985 | 赫伯特·豪普特曼 | 随从 | `laureate-herbert-a-hauptman` | 6★ | 现代 | `laureate-herbert-a-hauptman-skill` | Herbert A. Hauptman；效果取自「结构」（§3） |
| 1985 | 杰罗姆·卡尔勒 | 随从 | `laureate-jerome-karle` | 6★ | 现代 | `laureate-jerome-karle-skill` | Jerome Karle；效果取自「分析」（§3） |
| 1986 | 达德利·赫施巴赫 | 随从 | `laureate-dudley-r-herschbach` | 6★ | 现代 | `laureate-dudley-r-herschbach-skill` | Dudley R. Herschbach；效果取自「材料」（§3） |
| 1986 | 约翰·波拉尼 | 随从 | `laureate-john-c-polanyi` | 6★ | 现代 | `laureate-john-c-polanyi-skill` | John C. Polanyi；效果取自「提纯」（§3） |
| 1986 | 李远哲 | 随从 | `laureate-yuan-t-lee` | 6★ | 现代 | `laureate-yuan-t-lee-skill` | Yuan T. Lee；效果取自「合成」（§3） |
| 1987 | 查尔斯·佩德森 | 随从 | `laureate-charles-j-pedersen` | 6★ | 现代 | `laureate-charles-j-pedersen-skill` | Charles J. Pedersen；效果取自「催化」（§3） |
| 1987 | 唐纳德·克拉姆 | 随从 | `laureate-donald-j-cram` | 6★ | 现代 | `laureate-donald-j-cram-skill` | Donald J. Cram；效果取自「结构」（§3） |
| 1987 | 让-马里·莱恩 | 随从 | `laureate-jean-marie-lehn` | 6★ | 现代 | `laureate-jean-marie-lehn-skill` | Jean-Marie Lehn；效果取自「分析」（§3） |
| 1988 | 哈特穆特·米歇尔 | 随从 | `laureate-hartmut-michel` | 6★ | 现代 | `laureate-hartmut-michel-skill` | Hartmut Michel；效果取自「材料」（§3） |
| 1988 | 约翰·戴森霍费尔 | 随从 | `laureate-johann-deisenhofer` | 6★ | 现代 | `laureate-johann-deisenhofer-skill` | Johann Deisenhofer；效果取自「提纯」（§3） |
| 1988 | 罗伯特·胡贝尔 | 随从 | `laureate-robert-huber` | 6★ | 现代 | `laureate-robert-huber-skill` | Robert Huber；效果取自「合成」（§3） |
| 1989 | 西德尼·奥尔特曼 | 随从 | `laureate-sidney-altman` | 6★ | 现代 | `laureate-sidney-altman-skill` | Sidney Altman；效果取自「催化」（§3） |
| 1989 | 托马斯·切赫 | 随从 | `laureate-thomas-r-cech` | 6★ | 现代 | `laureate-thomas-r-cech-skill` | Thomas R. Cech；效果取自「结构」（§3） |
| 1990 | 伊莱亚斯·詹姆斯·科里 | 随从 | `laureate-elias-james-corey` | 6★ | 现代 | `laureate-elias-james-corey-skill` | Elias James Corey；效果取自「分析」（§3） |
| 1991 | 理查德·恩斯特 | 随从 | `laureate-richard-r-ernst` | 6★ | 现代 | `laureate-richard-r-ernst-skill` | Richard R. Ernst；效果取自「材料」（§3） |
| 1992 | 鲁道夫·马库斯 | 随从 | `laureate-rudolph-a-marcus` | 6★ | 现代 | `laureate-rudolph-a-marcus-skill` | Rudolph A. Marcus；效果取自「提纯」（§3） |
| 1993 | 卡里·穆利斯 | 随从 | `laureate-kary-b-mullis` | 6★ | 现代 | `laureate-kary-b-mullis-skill` | Kary B. Mullis；效果取自「合成」（§3） |
| 1993 | 迈克尔·史密斯 | 随从 | `laureate-michael-smith` | 6★ | 现代 | `laureate-michael-smith-skill` | Michael Smith；效果取自「催化」（§3） |
| 1994 | 乔治·奥拉 | 随从 | `laureate-george-a-olah` | 6★ | 现代 | `laureate-george-a-olah-skill` | George A. Olah；效果取自「结构」（§3） |
| 1995 | 舍伍德·罗兰 | 随从 | `laureate-f-sherwood-rowland` | 6★ | 现代 | `laureate-f-sherwood-rowland-skill` | F. Sherwood Rowland；效果取自「分析」（§3） |
| 1995 | 马里奥·莫利纳 | 随从 | `laureate-mario-j-molina` | 6★ | 现代 | `laureate-mario-j-molina-skill` | Mario J. Molina；效果取自「材料」（§3） |
| 1995 | 保罗·克鲁岑 | 随从 | `laureate-paul-j-crutzen` | 6★ | 现代 | `laureate-paul-j-crutzen-skill` | Paul J. Crutzen；效果取自「提纯」（§3） |
| 1996 | 理查德·斯莫利 | 随从 | `laureate-richard-e-smalley` | 6★ | 现代 | `laureate-richard-e-smalley-skill` | Richard E. Smalley；效果取自「合成」（§3） |
| 1996 | 小罗伯特·柯尔 | 随从 | `laureate-robert-f-curl-jr` | 6★ | 现代 | `laureate-robert-f-curl-jr-skill` | Robert F. Curl Jr.；效果取自「催化」（§3） |
| 1996 | 哈罗德·克罗托 | 随从 | `laureate-sir-harold-kroto` | 6★ | 现代 | `laureate-sir-harold-kroto-skill` | Sir Harold Kroto；效果取自「结构」（§3） |
| 1997 | 延斯·斯科 | 随从 | `laureate-jens-c-skou` | 6★ | 现代 | `laureate-jens-c-skou-skill` | Jens C. Skou；效果取自「分析」（§3） |
| 1997 | 约翰·沃克 | 随从 | `laureate-john-e-walker` | 6★ | 现代 | `laureate-john-e-walker-skill` | John E. Walker；效果取自「材料」（§3） |
| 1997 | 保罗·博耶 | 随从 | `laureate-paul-d-boyer` | 6★ | 现代 | `laureate-paul-d-boyer-skill` | Paul D. Boyer；效果取自「提纯」（§3） |
| 1998 | 约翰·波普尔 | 随从 | `laureate-john-pople` | 6★ | 现代 | `laureate-john-pople-skill` | John Pople；效果取自「合成」（§3） |
| 1998 | 沃尔特·科恩 | 随从 | `laureate-walter-kohn` | 6★ | 现代 | `laureate-walter-kohn-skill` | Walter Kohn；效果取自「催化」（§3） |
| 1999 | 艾哈迈德·泽维尔 | 随从 | `laureate-ahmed-zewail` | 6★ | 现代 | `laureate-ahmed-zewail-skill` | Ahmed Zewail；效果取自「结构」（§3） |
| 2000 | 艾伦·黑格 | 随从 | `laureate-alan-heeger` | 7★ | 未来 | `laureate-alan-heeger-skill` | Alan Heeger；效果取自「分析」（§3） |
| 2000 | 艾伦·麦克迪尔米德 | 随从 | `laureate-alan-macdiarmid` | 7★ | 未来 | `laureate-alan-macdiarmid-skill` | Alan MacDiarmid；效果取自「材料」（§3） |
| 2000 | 白川英树 | 随从 | `laureate-hideki-shirakawa` | 7★ | 未来 | `laureate-hideki-shirakawa-skill` | Hideki Shirakawa；效果取自「提纯」（§3） |
| 2001 | 巴里·夏普莱斯 | 随从 | `laureate-k-barry-sharpless` | 7★ | 未来 | `laureate-k-barry-sharpless-skill` | K. Barry Sharpless；效果取自「合成」（§3）；2001 化学奖、2022 化学奖 |
| 2001 | 野依良治 | 随从 | `laureate-ryoji-noyori` | 7★ | 未来 | `laureate-ryoji-noyori-skill` | Ryoji Noyori；效果取自「催化」（§3） |
| 2001 | 威廉·诺尔斯 | 随从 | `laureate-william-knowles` | 7★ | 未来 | `laureate-william-knowles-skill` | William Knowles；效果取自「结构」（§3） |
| 2002 | 约翰·芬恩 | 随从 | `laureate-john-b-fenn` | 7★ | 未来 | `laureate-john-b-fenn-skill` | John B. Fenn；效果取自「分析」（§3） |
| 2002 | 田中耕一 | 随从 | `laureate-koichi-tanaka` | 7★ | 未来 | `laureate-koichi-tanaka-skill` | Koichi Tanaka；效果取自「材料」（§3） |
| 2002 | 库尔特·维特里希 | 随从 | `laureate-kurt-wuthrich` | 7★ | 未来 | `laureate-kurt-wuthrich-skill` | Kurt Wüthrich；效果取自「提纯」（§3） |
| 2003 | 彼得·阿格雷 | 随从 | `laureate-peter-agre` | 7★ | 未来 | `laureate-peter-agre-skill` | Peter Agre；效果取自「合成」（§3） |
| 2003 | 罗德里克·麦金农 | 随从 | `laureate-roderick-mackinnon` | 7★ | 未来 | `laureate-roderick-mackinnon-skill` | Roderick MacKinnon；效果取自「催化」（§3） |
| 2004 | 阿龙·切哈诺沃 | 随从 | `laureate-aaron-ciechanover` | 7★ | 未来 | `laureate-aaron-ciechanover-skill` | Aaron Ciechanover；效果取自「结构」（§3） |
| 2004 | 阿夫拉姆·赫什科 | 随从 | `laureate-avram-hershko` | 7★ | 未来 | `laureate-avram-hershko-skill` | Avram Hershko；效果取自「分析」（§3） |
| 2004 | 欧文·罗斯 | 随从 | `laureate-irwin-rose` | 7★ | 未来 | `laureate-irwin-rose-skill` | Irwin Rose；效果取自「材料」（§3） |
| 2005 | 理查德·施罗克 | 随从 | `laureate-richard-r-schrock` | 7★ | 未来 | `laureate-richard-r-schrock-skill` | Richard R. Schrock；效果取自「提纯」（§3） |
| 2005 | 罗伯特·格拉布斯 | 随从 | `laureate-robert-h-grubbs` | 7★ | 未来 | `laureate-robert-h-grubbs-skill` | Robert H. Grubbs；效果取自「合成」（§3） |
| 2005 | 伊夫·肖万 | 随从 | `laureate-yves-chauvin` | 7★ | 未来 | `laureate-yves-chauvin-skill` | Yves Chauvin；效果取自「催化」（§3） |
| 2006 | 罗杰·科恩伯格 | 随从 | `laureate-roger-d-kornberg` | 7★ | 未来 | `laureate-roger-d-kornberg-skill` | Roger D. Kornberg；效果取自「结构」（§3） |
| 2007 | 格哈德·埃特尔 | 随从 | `laureate-gerhard-ertl` | 7★ | 未来 | `laureate-gerhard-ertl-skill` | Gerhard Ertl；效果取自「分析」（§3） |
| 2008 | 马丁·查尔菲 | 随从 | `laureate-martin-chalfie` | 7★ | 未来 | `laureate-martin-chalfie-skill` | Martin Chalfie；效果取自「材料」（§3） |
| 2008 | 下村脩 | 随从 | `laureate-osamu-shimomura` | 7★ | 未来 | `laureate-osamu-shimomura-skill` | Osamu Shimomura；效果取自「提纯」（§3） |
| 2008 | 钱永健 | 随从 | `laureate-roger-y-tsien` | 7★ | 未来 | `laureate-roger-y-tsien-skill` | Roger Y. Tsien；效果取自「合成」（§3） |
| 2009 | 阿达·约纳特 | 随从 | `laureate-ada-e-yonath` | 7★ | 未来 | `laureate-ada-e-yonath-skill` | Ada E. Yonath；效果取自「催化」（§3） |
| 2009 | 托马斯·施泰茨 | 随从 | `laureate-thomas-a-steitz` | 7★ | 未来 | `laureate-thomas-a-steitz-skill` | Thomas A. Steitz；效果取自「结构」（§3） |
| 2009 | 文卡特拉曼·拉马克里希南 | 随从 | `laureate-venkatraman-ramakrishnan` | 7★ | 未来 | `laureate-venkatraman-ramakrishnan-skill` | Venkatraman Ramakrishnan；效果取自「分析」（§3） |
| 2010 | 铃木章 | 随从 | `laureate-akira-suzuki` | 7★ | 未来 | `laureate-akira-suzuki-skill` | Akira Suzuki；效果取自「材料」（§3） |
| 2010 | 根岸英一 | 随从 | `laureate-ei-ichi-negishi` | 7★ | 未来 | `laureate-ei-ichi-negishi-skill` | Ei-ichi Negishi；效果取自「提纯」（§3） |
| 2010 | 理查德·赫克 | 随从 | `laureate-richard-f-heck` | 7★ | 未来 | `laureate-richard-f-heck-skill` | Richard F. Heck；效果取自「合成」（§3） |
| 2011 | 达尼埃尔·谢赫特曼 | 随从 | `laureate-dan-shechtman` | 7★ | 未来 | `laureate-dan-shechtman-skill` | Dan Shechtman；效果取自「催化」（§3） |
| 2012 | 布赖恩·科比尔卡 | 随从 | `laureate-brian-kobilka` | 7★ | 未来 | `laureate-brian-kobilka-skill` | Brian Kobilka；效果取自「结构」（§3） |
| 2012 | 罗伯特·莱夫科维茨 | 随从 | `laureate-robert-j-lefkowitz` | 7★ | 未来 | `laureate-robert-j-lefkowitz-skill` | Robert J. Lefkowitz；效果取自「分析」（§3） |
| 2013 | 阿里耶·瓦谢尔 | 随从 | `laureate-arieh-warshel` | 7★ | 未来 | `laureate-arieh-warshel-skill` | Arieh Warshel；效果取自「材料」（§3） |
| 2013 | 马丁·卡普拉斯 | 随从 | `laureate-martin-karplus` | 7★ | 未来 | `laureate-martin-karplus-skill` | Martin Karplus；效果取自「提纯」（§3） |
| 2013 | 迈克尔·莱维特 | 随从 | `laureate-michael-levitt` | 7★ | 未来 | `laureate-michael-levitt-skill` | Michael Levitt；效果取自「合成」（§3） |
| 2014 | 埃里克·贝齐格 | 随从 | `laureate-eric-betzig` | 7★ | 未来 | `laureate-eric-betzig-skill` | Eric Betzig；效果取自「催化」（§3） |
| 2014 | 斯特凡·黑尔 | 随从 | `laureate-stefan-w-hell` | 7★ | 未来 | `laureate-stefan-w-hell-skill` | Stefan W. Hell；效果取自「结构」（§3） |
| 2014 | 威廉·默纳 | 随从 | `laureate-william-e-moerner` | 7★ | 未来 | `laureate-william-e-moerner-skill` | William E. Moerner；效果取自「分析」（§3） |
| 2015 | 阿齐兹·桑贾尔 | 随从 | `laureate-aziz-sancar` | 7★ | 未来 | `laureate-aziz-sancar-skill` | Aziz Sancar；效果取自「材料」（§3） |
| 2015 | 保罗·莫德里奇 | 随从 | `laureate-paul-modrich` | 7★ | 未来 | `laureate-paul-modrich-skill` | Paul Modrich；效果取自「提纯」（§3） |
| 2015 | 托马斯·林达尔 | 随从 | `laureate-tomas-lindahl` | 7★ | 未来 | `laureate-tomas-lindahl-skill` | Tomas Lindahl；效果取自「合成」（§3） |
| 2016 | 伯纳德·费林加 | 随从 | `laureate-bernard-l-feringa` | 7★ | 未来 | `laureate-bernard-l-feringa-skill` | Bernard L. Feringa；效果取自「催化」（§3） |
| 2016 | 让-皮埃尔·索瓦日 | 随从 | `laureate-jean-pierre-sauvage` | 7★ | 未来 | `laureate-jean-pierre-sauvage-skill` | Jean-Pierre Sauvage；效果取自「结构」（§3） |
| 2016 | 詹姆斯·弗雷泽·斯托达特 | 随从 | `laureate-sir-j-fraser-stoddart` | 7★ | 未来 | `laureate-sir-j-fraser-stoddart-skill` | Sir J. Fraser Stoddart；效果取自「分析」（§3） |
| 2017 | 雅克·杜博歇 | 随从 | `laureate-jacques-dubochet` | 7★ | 未来 | `laureate-jacques-dubochet-skill` | Jacques Dubochet；效果取自「材料」（§3） |
| 2017 | 约阿希姆·弗兰克 | 随从 | `laureate-joachim-frank` | 7★ | 未来 | `laureate-joachim-frank-skill` | Joachim Frank；效果取自「提纯」（§3） |
| 2017 | 理查德·亨德森 | 随从 | `laureate-richard-henderson` | 7★ | 未来 | `laureate-richard-henderson-skill` | Richard Henderson；效果取自「合成」（§3） |
| 2018 | 弗朗西丝·阿诺德 | 随从 | `laureate-frances-h-arnold` | 7★ | 未来 | `laureate-frances-h-arnold-skill` | Frances H. Arnold；效果取自「催化」（§3） |
| 2018 | 乔治·史密斯（化学） | 随从 | `laureate-george-p-smith` | 7★ | 未来 | `laureate-george-p-smith-skill` | George P. Smith；效果取自「结构」（§3） |
| 2018 | 格雷戈里·温特 | 随从 | `laureate-sir-gregory-p-winter` | 7★ | 未来 | `laureate-sir-gregory-p-winter-skill` | Sir Gregory P. Winter；效果取自「分析」（§3） |
| 2019 | 吉野彰 | 随从 | `laureate-akira-yoshino` | 7★ | 未来 | `laureate-akira-yoshino-skill` | Akira Yoshino；效果取自「材料」（§3） |
| 2019 | 约翰·古迪纳夫 | 随从 | `laureate-john-b-goodenough` | 7★ | 未来 | `laureate-john-b-goodenough-skill` | John B. Goodenough；效果取自「提纯」（§3） |
| 2019 | 斯坦利·惠廷厄姆 | 随从 | `laureate-m-stanley-whittingham` | 7★ | 未来 | `laureate-m-stanley-whittingham-skill` | M. Stanley Whittingham；效果取自「合成」（§3） |
| 2020 | 埃玛纽埃勒·沙尔庞捷 | 随从 | `laureate-emmanuelle-charpentier` | 7★ | 未来 | `laureate-emmanuelle-charpentier-skill` | Emmanuelle Charpentier；效果取自「催化」（§3） |
| 2020 | 珍妮弗·杜德纳 | 随从 | `laureate-jennifer-a-doudna` | 7★ | 未来 | `laureate-jennifer-a-doudna-skill` | Jennifer A. Doudna；效果取自「结构」（§3） |
| 2021 | 本杰明·利斯特 | 随从 | `laureate-benjamin-list` | 7★ | 未来 | `laureate-benjamin-list-skill` | Benjamin List；效果取自「分析」（§3） |
| 2021 | 戴维·麦克米伦 | 随从 | `laureate-david-w-c-macmillan` | 7★ | 未来 | `laureate-david-w-c-macmillan-skill` | David W.C. MacMillan；效果取自「材料」（§3） |
| 2022 | 卡罗琳·贝尔托齐 | 随从 | `laureate-carolyn-bertozzi` | 7★ | 未来 | `laureate-carolyn-bertozzi-skill` | Carolyn Bertozzi；效果取自「提纯」（§3） |
| 2022 | 莫滕·梅尔达尔 | 随从 | `laureate-morten-meldal` | 7★ | 未来 | `laureate-morten-meldal-skill` | Morten Meldal；效果取自「合成」（§3） |
| 2023 | 阿列克谢·叶基莫夫 | 随从 | `laureate-aleksey-yekimov` | 7★ | 未来 | `laureate-aleksey-yekimov-skill` | Aleksey Yekimov；效果取自「催化」（§3） |
| 2023 | 路易斯·布鲁斯 | 随从 | `laureate-louis-brus` | 7★ | 未来 | `laureate-louis-brus-skill` | Louis Brus；效果取自「结构」（§3） |
| 2023 | 蒙吉·巴文迪 | 随从 | `laureate-moungi-bawendi` | 7★ | 未来 | `laureate-moungi-bawendi-skill` | Moungi Bawendi；效果取自「分析」（§3） |
| 2024 | 戴维·贝克 | 随从 | `laureate-david-baker` | 7★ | 未来 | `laureate-david-baker-skill` | David Baker；效果取自「材料」（§3） |
| 2024 | 德米斯·哈萨比斯 | 随从 | `laureate-demis-hassabis` | 7★ | 未来 | `laureate-demis-hassabis-skill` | Demis Hassabis；效果取自「提纯」（§3） |
| 2024 | 约翰·江珀 | 随从 | `laureate-john-jumper` | 7★ | 未来 | `laureate-john-jumper-skill` | John Jumper；效果取自「合成」（§3） |
| 2025 | 奥马尔·亚吉 | 随从 | `laureate-omar-m-yaghi` | 7★ | 未来 | `laureate-omar-m-yaghi-skill` | Omar M. Yaghi；效果取自「催化」（§3） |
| 2025 | 理查德·罗布森 | 随从 | `laureate-richard-robson` | 7★ | 未来 | `laureate-richard-robson-skill` | Richard Robson；效果取自「结构」（§3） |
| 2025 | 北川进 | 随从 | `laureate-susumu-kitagawa` | 7★ | 未来 | `laureate-susumu-kitagawa-skill` | Susumu Kitagawa；效果取自「分析」（§3） |
| 2026 | 亨利·卡根 | 随从 | `laureate-henri-b-kagan` | 7★ | 未来 | `laureate-henri-b-kagan-skill` | Henri B. Kagan；效果取自「材料」（§3） |
| 2026 | 硤合宪三 | 随从 | `laureate-kenso-soai` | 7★ | 未来 | `laureate-kenso-soai-skill` | Kenso Soai；效果取自「提纯」（§3） |

#### 5.3 生理学或医学奖（235）

| 年份 | 得主 | 类型 | 卡牌 id | 星级/费用 | 时代 | 技能 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1901 | 埃米尔·冯·贝林 | 随从 | `laureate-emil-von-behring` | 4★ | 工业 | `laureate-emil-von-behring-skill` | Emil von Behring；效果取自「免疫」（§3） |
| 1902 | 罗纳德·罗斯 | 随从 | `laureate-ronald-ross` | 4★ | 工业 | `laureate-ronald-ross-skill` | Ronald Ross；效果取自「遗传」（§3） |
| 1903 | 尼尔斯·吕贝里·芬森 | 随从 | `laureate-niels-ryberg-finsen` | 4★ | 工业 | `laureate-niels-ryberg-finsen-skill` | Niels Ryberg Finsen；效果取自「神经」（§3） |
| 1904 | 伊万·巴甫洛夫 | 随从 | `laureate-ivan-pavlov` | 4★ | 工业 | `laureate-ivan-pavlov-skill` | Ivan Pavlov；效果取自「微生物」（§3） |
| 1905 | 罗伯特·科赫 | 随从 | `laureate-robert-koch` | 4★ | 工业 | `laureate-robert-koch-skill` | Robert Koch；效果取自「生理」（§3） |
| 1906 | 卡米洛·高尔基 | 随从 | `laureate-camillo-golgi` | 4★ | 工业 | `laureate-camillo-golgi-skill` | Camillo Golgi；效果取自「药理」（§3） |
| 1906 | 圣地亚哥·拉蒙-卡哈尔 | 随从 | `laureate-santiago-ramon-y-cajal` | 4★ | 工业 | `laureate-santiago-ramon-y-cajal-skill` | Santiago Ramón y Cajal；效果取自「免疫」（§3） |
| 1907 | 阿方斯·拉韦朗 | 随从 | `laureate-alphonse-laveran` | 4★ | 工业 | `laureate-alphonse-laveran-skill` | Alphonse Laveran；效果取自「遗传」（§3） |
| 1908 | 伊利亚·梅契尼科夫 | 随从 | `laureate-ilya-mechnikov` | 4★ | 工业 | `laureate-ilya-mechnikov-skill` | Ilya Mechnikov；效果取自「神经」（§3） |
| 1908 | 保罗·埃利希 | 随从 | `laureate-paul-ehrlich` | 4★ | 工业 | `laureate-paul-ehrlich-skill` | Paul Ehrlich；效果取自「微生物」（§3） |
| 1909 | 特奥多尔·科赫尔 | 随从 | `laureate-theodor-kocher` | 4★ | 工业 | `laureate-theodor-kocher-skill` | Theodor Kocher；效果取自「生理」（§3） |
| 1910 | 阿尔布雷希特·科塞尔 | 随从 | `laureate-albrecht-kossel` | 4★ | 工业 | `laureate-albrecht-kossel-skill` | Albrecht Kossel；效果取自「药理」（§3） |
| 1911 | 阿尔瓦·古尔斯特兰德 | 随从 | `laureate-allvar-gullstrand` | 4★ | 工业 | `laureate-allvar-gullstrand-skill` | Allvar Gullstrand；效果取自「免疫」（§3） |
| 1912 | 亚历克西斯·卡雷尔 | 随从 | `laureate-alexis-carrel` | 4★ | 工业 | `laureate-alexis-carrel-skill` | Alexis Carrel；效果取自「遗传」（§3） |
| 1913 | 夏尔·里歇 | 随从 | `laureate-charles-richet` | 4★ | 工业 | `laureate-charles-richet-skill` | Charles Richet；效果取自「神经」（§3） |
| 1914 | 罗伯特·巴拉尼 | 随从 | `laureate-robert-barany` | 4★ | 工业 | `laureate-robert-barany-skill` | Robert Bárány；效果取自「微生物」（§3） |
| 1919 | 朱尔·博尔代 | 随从 | `laureate-jules-bordet` | 4★ | 工业 | `laureate-jules-bordet-skill` | Jules Bordet；效果取自「生理」（§3） |
| 1920 | 奥古斯特·克罗 | 随从 | `laureate-august-krogh` | 4★ | 工业 | `laureate-august-krogh-skill` | August Krogh；效果取自「药理」（§3） |
| 1922 | 阿奇博尔德·希尔 | 随从 | `laureate-archibald-v-hill` | 4★ | 工业 | `laureate-archibald-v-hill-skill` | Archibald V. Hill；效果取自「免疫」（§3） |
| 1922 | 奥托·迈尔霍夫 | 随从 | `laureate-otto-meyerhof` | 4★ | 工业 | `laureate-otto-meyerhof-skill` | Otto Meyerhof；效果取自「遗传」（§3） |
| 1923 | 弗雷德里克·班廷 | 随从 | `laureate-frederick-g-banting` | 4★ | 工业 | `laureate-frederick-g-banting-skill` | Frederick G. Banting；效果取自「神经」（§3） |
| 1923 | 约翰·麦克劳德 | 随从 | `laureate-john-macleod` | 4★ | 工业 | `laureate-john-macleod-skill` | John Macleod；效果取自「微生物」（§3） |
| 1924 | 威廉·埃因托芬 | 随从 | `laureate-willem-einthoven` | 4★ | 工业 | `laureate-willem-einthoven-skill` | Willem Einthoven；效果取自「生理」（§3） |
| 1926 | 约翰内斯·菲比格 | 随从 | `laureate-johannes-fibiger` | 4★ | 工业 | `laureate-johannes-fibiger-skill` | Johannes Fibiger；效果取自「药理」（§3） |
| 1927 | 尤利乌斯·瓦格纳-尧雷格 | 随从 | `laureate-julius-wagner-jauregg` | 4★ | 工业 | `laureate-julius-wagner-jauregg-skill` | Julius Wagner-Jauregg；效果取自「免疫」（§3） |
| 1928 | 夏尔·尼科尔 | 随从 | `laureate-charles-nicolle` | 4★ | 工业 | `laureate-charles-nicolle-skill` | Charles Nicolle；效果取自「遗传」（§3） |
| 1929 | 克里斯蒂安·艾克曼 | 随从 | `laureate-christiaan-eijkman` | 4★ | 工业 | `laureate-christiaan-eijkman-skill` | Christiaan Eijkman；效果取自「神经」（§3） |
| 1929 | 弗雷德里克·霍普金斯 | 随从 | `laureate-sir-frederick-hopkins` | 4★ | 工业 | `laureate-sir-frederick-hopkins-skill` | Sir Frederick Hopkins；效果取自「微生物」（§3） |
| 1930 | 卡尔·兰德施泰纳 | 随从 | `laureate-karl-landsteiner` | 4★ | 工业 | `laureate-karl-landsteiner-skill` | Karl Landsteiner；效果取自「生理」（§3） |
| 1931 | 奥托·瓦尔堡 | 随从 | `laureate-otto-warburg` | 4★ | 工业 | `laureate-otto-warburg-skill` | Otto Warburg；效果取自「药理」（§3） |
| 1932 | 埃德加·阿德里安 | 随从 | `laureate-edgar-adrian` | 4★ | 工业 | `laureate-edgar-adrian-skill` | Edgar Adrian；效果取自「免疫」（§3） |
| 1932 | 查尔斯·谢灵顿 | 随从 | `laureate-sir-charles-sherrington` | 4★ | 工业 | `laureate-sir-charles-sherrington-skill` | Sir Charles Sherrington；效果取自「遗传」（§3） |
| 1933 | 托马斯·摩根 | 现有随从 | `morgan` | 5★ | 现代 | `[aura-biology-plus-0-plus-1]` | 复用现有卡，不新增 |
| 1934 | 乔治·惠普尔 | 随从 | `laureate-george-h-whipple` | 4★ | 工业 | `laureate-george-h-whipple-skill` | George H. Whipple；效果取自「神经」（§3） |
| 1934 | 乔治·迈诺特 | 随从 | `laureate-george-r-minot` | 4★ | 工业 | `laureate-george-r-minot-skill` | George R. Minot；效果取自「微生物」（§3） |
| 1934 | 威廉·墨菲 | 随从 | `laureate-william-p-murphy` | 4★ | 工业 | `laureate-william-p-murphy-skill` | William P. Murphy；效果取自「生理」（§3） |
| 1935 | 汉斯·施佩曼 | 随从 | `laureate-hans-spemann` | 4★ | 工业 | `laureate-hans-spemann-skill` | Hans Spemann；效果取自「药理」（§3） |
| 1936 | 奥托·勒维 | 随从 | `laureate-otto-loewi` | 4★ | 工业 | `laureate-otto-loewi-skill` | Otto Loewi；效果取自「免疫」（§3） |
| 1936 | 亨利·戴尔 | 随从 | `laureate-sir-henry-dale` | 4★ | 工业 | `laureate-sir-henry-dale-skill` | Sir Henry Dale；效果取自「遗传」（§3） |
| 1937 | 阿尔伯特·圣捷尔吉 | 随从 | `laureate-albert-szent-gyorgyi` | 4★ | 工业 | `laureate-albert-szent-gyorgyi-skill` | Albert Szent-Györgyi；效果取自「神经」（§3） |
| 1938 | 科内耶·海曼斯 | 随从 | `laureate-corneille-heymans` | 4★ | 工业 | `laureate-corneille-heymans-skill` | Corneille Heymans；效果取自「微生物」（§3） |
| 1939 | 格哈德·多马克 | 随从 | `laureate-gerhard-domagk` | 4★ | 工业 | `laureate-gerhard-domagk-skill` | Gerhard Domagk；效果取自「生理」（§3） |
| 1943 | 爱德华·多伊西 | 随从 | `laureate-edward-a-doisy` | 5★ | 工业 | `laureate-edward-a-doisy-skill` | Edward A. Doisy；效果取自「药理」（§3） |
| 1943 | 亨里克·达姆 | 随从 | `laureate-henrik-dam` | 5★ | 工业 | `laureate-henrik-dam-skill` | Henrik Dam；效果取自「免疫」（§3） |
| 1944 | 赫伯特·加瑟 | 随从 | `laureate-herbert-s-gasser` | 5★ | 工业 | `laureate-herbert-s-gasser-skill` | Herbert S. Gasser；效果取自「遗传」（§3） |
| 1944 | 约瑟夫·厄兰格 | 随从 | `laureate-joseph-erlanger` | 5★ | 工业 | `laureate-joseph-erlanger-skill` | Joseph Erlanger；效果取自「神经」（§3） |
| 1945 | 恩斯特·钱恩 | 随从 | `laureate-ernst-b-chain` | 5★ | 工业 | `laureate-ernst-b-chain-skill` | Ernst B. Chain；效果取自「微生物」（§3） |
| 1945 | 亚历山大·弗莱明 | 现有随从 | `fleming` | 4★ | 现代 | `[virus-growth-once]` | 复用现有卡，不新增 |
| 1945 | 霍华德·弗洛里 | 随从 | `laureate-sir-howard-florey` | 5★ | 工业 | `laureate-sir-howard-florey-skill` | Sir Howard Florey；效果取自「生理」（§3） |
| 1946 | 赫尔曼·穆勒 | 随从 | `laureate-hermann-j-muller` | 5★ | 现代 | `laureate-hermann-j-muller-skill` | Hermann J. Muller；效果取自「药理」（§3） |
| 1947 | 贝尔纳多·奥赛 | 随从 | `laureate-bernardo-houssay` | 5★ | 现代 | `laureate-bernardo-houssay-skill` | Bernardo Houssay；效果取自「免疫」（§3） |
| 1947 | 卡尔·科里 | 随从 | `laureate-carl-cori` | 5★ | 现代 | `laureate-carl-cori-skill` | Carl Cori；效果取自「遗传」（§3） |
| 1947 | 格蒂·科里 | 随从 | `laureate-gerty-cori` | 5★ | 现代 | `laureate-gerty-cori-skill` | Gerty Cori；效果取自「神经」（§3） |
| 1948 | 保罗·米勒 | 随从 | `laureate-paul-muller` | 5★ | 现代 | `laureate-paul-muller-skill` | Paul Müller；效果取自「微生物」（§3） |
| 1949 | 埃加斯·莫尼斯 | 随从 | `laureate-egas-moniz` | 5★ | 现代 | `laureate-egas-moniz-skill` | Egas Moniz；效果取自「生理」（§3） |
| 1949 | 瓦尔特·赫斯 | 随从 | `laureate-walter-hess` | 5★ | 现代 | `laureate-walter-hess-skill` | Walter Hess；效果取自「药理」（§3） |
| 1950 | 爱德华·肯德尔 | 随从 | `laureate-edward-c-kendall` | 5★ | 现代 | `laureate-edward-c-kendall-skill` | Edward C. Kendall；效果取自「免疫」（§3） |
| 1950 | 菲利普·亨奇 | 随从 | `laureate-philip-s-hench` | 5★ | 现代 | `laureate-philip-s-hench-skill` | Philip S. Hench；效果取自「遗传」（§3） |
| 1950 | 塔德乌什·赖希施泰因 | 随从 | `laureate-tadeus-reichstein` | 5★ | 现代 | `laureate-tadeus-reichstein-skill` | Tadeus Reichstein；效果取自「神经」（§3） |
| 1951 | 马克斯·泰勒 | 随从 | `laureate-max-theiler` | 5★ | 现代 | `laureate-max-theiler-skill` | Max Theiler；效果取自「微生物」（§3） |
| 1952 | 塞尔曼·瓦克斯曼 | 随从 | `laureate-selman-a-waksman` | 5★ | 现代 | `laureate-selman-a-waksman-skill` | Selman A. Waksman；效果取自「生理」（§3） |
| 1953 | 弗里茨·李普曼 | 随从 | `laureate-fritz-lipmann` | 5★ | 现代 | `laureate-fritz-lipmann-skill` | Fritz Lipmann；效果取自「药理」（§3） |
| 1953 | 汉斯·克雷布斯 | 随从 | `laureate-hans-krebs` | 5★ | 现代 | `laureate-hans-krebs-skill` | Hans Krebs；效果取自「免疫」（§3） |
| 1954 | 弗雷德里克·罗宾斯 | 随从 | `laureate-frederick-c-robbins` | 5★ | 现代 | `laureate-frederick-c-robbins-skill` | Frederick C. Robbins；效果取自「遗传」（§3） |
| 1954 | 约翰·恩德斯 | 随从 | `laureate-john-f-enders` | 5★ | 现代 | `laureate-john-f-enders-skill` | John F. Enders；效果取自「神经」（§3） |
| 1954 | 托马斯·韦勒 | 随从 | `laureate-thomas-h-weller` | 5★ | 现代 | `laureate-thomas-h-weller-skill` | Thomas H. Weller；效果取自「微生物」（§3） |
| 1955 | 胡戈·特奥雷尔 | 随从 | `laureate-hugo-theorell` | 5★ | 现代 | `laureate-hugo-theorell-skill` | Hugo Theorell；效果取自「生理」（§3） |
| 1956 | 安德烈·库尔南 | 随从 | `laureate-andre-f-cournand` | 5★ | 现代 | `laureate-andre-f-cournand-skill` | André F. Cournand；效果取自「药理」（§3） |
| 1956 | 迪金森·理查兹 | 随从 | `laureate-dickinson-w-richards` | 5★ | 现代 | `laureate-dickinson-w-richards-skill` | Dickinson W. Richards；效果取自「免疫」（§3） |
| 1956 | 维尔纳·福斯曼 | 随从 | `laureate-werner-forssmann` | 5★ | 现代 | `laureate-werner-forssmann-skill` | Werner Forssmann；效果取自「遗传」（§3） |
| 1957 | 达尼埃尔·博韦 | 随从 | `laureate-daniel-bovet` | 5★ | 现代 | `laureate-daniel-bovet-skill` | Daniel Bovet；效果取自「神经」（§3） |
| 1958 | 爱德华·塔图姆 | 随从 | `laureate-edward-tatum` | 5★ | 现代 | `laureate-edward-tatum-skill` | Edward Tatum；效果取自「微生物」（§3） |
| 1958 | 乔治·比德尔 | 随从 | `laureate-george-beadle` | 5★ | 现代 | `laureate-george-beadle-skill` | George Beadle；效果取自「生理」（§3） |
| 1958 | 乔舒亚·莱德伯格 | 随从 | `laureate-joshua-lederberg` | 5★ | 现代 | `laureate-joshua-lederberg-skill` | Joshua Lederberg；效果取自「药理」（§3） |
| 1959 | 阿瑟·科恩伯格 | 随从 | `laureate-arthur-kornberg` | 5★ | 现代 | `laureate-arthur-kornberg-skill` | Arthur Kornberg；效果取自「免疫」（§3） |
| 1959 | 塞韦罗·奥乔亚 | 随从 | `laureate-severo-ochoa` | 5★ | 现代 | `laureate-severo-ochoa-skill` | Severo Ochoa；效果取自「遗传」（§3） |
| 1960 | 彼得·梅达沃 | 随从 | `laureate-peter-medawar` | 5★ | 现代 | `laureate-peter-medawar-skill` | Peter Medawar；效果取自「神经」（§3） |
| 1960 | 弗兰克·麦克法兰·伯内特 | 随从 | `laureate-sir-frank-macfarlane-burnet` | 5★ | 现代 | `laureate-sir-frank-macfarlane-burnet-skill` | Sir Frank Macfarlane Burnet；效果取自「微生物」（§3） |
| 1961 | 格奥尔格·冯·贝凯西 | 随从 | `laureate-georg-von-bekesy` | 5★ | 现代 | `laureate-georg-von-bekesy-skill` | Georg von Békésy；效果取自「生理」（§3） |
| 1962 | 弗朗西斯·克里克 | 随从 | `laureate-francis-crick` | 5★ | 现代 | `laureate-francis-crick-skill` | Francis Crick；效果取自「药理」（§3） |
| 1962 | 詹姆斯·沃森 | 随从 | `laureate-james-watson` | 5★ | 现代 | `laureate-james-watson-skill` | James Watson；效果取自「免疫」（§3） |
| 1962 | 莫里斯·威尔金斯 | 随从 | `laureate-maurice-wilkins` | 5★ | 现代 | `laureate-maurice-wilkins-skill` | Maurice Wilkins；效果取自「遗传」（§3） |
| 1963 | 艾伦·霍奇金 | 随从 | `laureate-alan-hodgkin` | 5★ | 现代 | `laureate-alan-hodgkin-skill` | Alan Hodgkin；效果取自「神经」（§3） |
| 1963 | 安德鲁·赫胥黎 | 随从 | `laureate-andrew-huxley` | 5★ | 现代 | `laureate-andrew-huxley-skill` | Andrew Huxley；效果取自「微生物」（§3） |
| 1963 | 约翰·埃克尔斯 | 随从 | `laureate-sir-john-eccles` | 5★ | 现代 | `laureate-sir-john-eccles-skill` | Sir John Eccles；效果取自「生理」（§3） |
| 1964 | 费奥多尔·吕嫩 | 随从 | `laureate-feodor-lynen` | 5★ | 现代 | `laureate-feodor-lynen-skill` | Feodor Lynen；效果取自「药理」（§3） |
| 1964 | 康拉德·布洛赫 | 随从 | `laureate-konrad-bloch` | 5★ | 现代 | `laureate-konrad-bloch-skill` | Konrad Bloch；效果取自「免疫」（§3） |
| 1965 | 安德烈·利沃夫 | 随从 | `laureate-andre-lwoff` | 5★ | 现代 | `laureate-andre-lwoff-skill` | André Lwoff；效果取自「遗传」（§3） |
| 1965 | 弗朗索瓦·雅各布 | 随从 | `laureate-francois-jacob` | 5★ | 现代 | `laureate-francois-jacob-skill` | François Jacob；效果取自「神经」（§3） |
| 1965 | 雅克·莫诺 | 随从 | `laureate-jacques-monod` | 5★ | 现代 | `laureate-jacques-monod-skill` | Jacques Monod；效果取自「微生物」（§3） |
| 1966 | 查尔斯·哈金斯 | 随从 | `laureate-charles-b-huggins` | 5★ | 现代 | `laureate-charles-b-huggins-skill` | Charles B. Huggins；效果取自「生理」（§3） |
| 1966 | 佩顿·劳斯 | 随从 | `laureate-peyton-rous` | 5★ | 现代 | `laureate-peyton-rous-skill` | Peyton Rous；效果取自「药理」（§3） |
| 1967 | 乔治·沃尔德 | 随从 | `laureate-george-wald` | 5★ | 现代 | `laureate-george-wald-skill` | George Wald；效果取自「免疫」（§3） |
| 1967 | 基弗·哈特兰 | 随从 | `laureate-keffer-hartline` | 5★ | 现代 | `laureate-keffer-hartline-skill` | Keffer Hartline；效果取自「遗传」（§3） |
| 1967 | 拉格纳·格拉尼特 | 随从 | `laureate-ragnar-granit` | 5★ | 现代 | `laureate-ragnar-granit-skill` | Ragnar Granit；效果取自「神经」（§3） |
| 1968 | 戈宾德·霍拉纳 | 随从 | `laureate-h-gobind-khorana` | 5★ | 现代 | `laureate-h-gobind-khorana-skill` | H. Gobind Khorana；效果取自「微生物」（§3） |
| 1968 | 马歇尔·尼伦伯格 | 随从 | `laureate-marshall-w-nirenberg` | 5★ | 现代 | `laureate-marshall-w-nirenberg-skill` | Marshall W. Nirenberg；效果取自「生理」（§3） |
| 1968 | 罗伯特·霍利 | 随从 | `laureate-robert-w-holley` | 5★ | 现代 | `laureate-robert-w-holley-skill` | Robert W. Holley；效果取自「药理」（§3） |
| 1969 | 阿尔弗雷德·赫尔希 | 随从 | `laureate-alfred-d-hershey` | 5★ | 现代 | `laureate-alfred-d-hershey-skill` | Alfred D. Hershey；效果取自「免疫」（§3） |
| 1969 | 马克斯·德尔布吕克 | 随从 | `laureate-max-delbruck` | 5★ | 现代 | `laureate-max-delbruck-skill` | Max Delbrück；效果取自「遗传」（§3） |
| 1969 | 萨尔瓦多·卢里亚 | 随从 | `laureate-salvador-e-luria` | 5★ | 现代 | `laureate-salvador-e-luria-skill` | Salvador E. Luria；效果取自「神经」（§3） |
| 1970 | 朱利叶斯·阿克塞尔罗德 | 随从 | `laureate-julius-axelrod` | 6★ | 现代 | `laureate-julius-axelrod-skill` | Julius Axelrod；效果取自「微生物」（§3） |
| 1970 | 伯纳德·卡茨 | 随从 | `laureate-sir-bernard-katz` | 6★ | 现代 | `laureate-sir-bernard-katz-skill` | Sir Bernard Katz；效果取自「生理」（§3） |
| 1970 | 乌尔夫·冯·奥伊勒 | 随从 | `laureate-ulf-von-euler` | 6★ | 现代 | `laureate-ulf-von-euler-skill` | Ulf von Euler；效果取自「药理」（§3） |
| 1971 | 小厄尔·萨瑟兰 | 随从 | `laureate-earl-w-sutherland-jr` | 6★ | 现代 | `laureate-earl-w-sutherland-jr-skill` | Earl W. Sutherland, Jr.；效果取自「免疫」（§3） |
| 1972 | 杰拉尔德·埃德尔曼 | 随从 | `laureate-gerald-m-edelman` | 6★ | 现代 | `laureate-gerald-m-edelman-skill` | Gerald M. Edelman；效果取自「遗传」（§3） |
| 1972 | 罗德尼·波特 | 随从 | `laureate-rodney-r-porter` | 6★ | 现代 | `laureate-rodney-r-porter-skill` | Rodney R. Porter；效果取自「神经」（§3） |
| 1973 | 卡尔·冯·弗里施 | 随从 | `laureate-karl-von-frisch` | 6★ | 现代 | `laureate-karl-von-frisch-skill` | Karl von Frisch；效果取自「微生物」（§3） |
| 1973 | 康拉德·洛伦茨 | 随从 | `laureate-konrad-lorenz` | 6★ | 现代 | `laureate-konrad-lorenz-skill` | Konrad Lorenz；效果取自「生理」（§3） |
| 1973 | 尼古拉斯·廷伯根 | 随从 | `laureate-nikolaas-tinbergen` | 6★ | 现代 | `laureate-nikolaas-tinbergen-skill` | Nikolaas Tinbergen；效果取自「药理」（§3） |
| 1974 | 阿尔伯特·克劳德 | 随从 | `laureate-albert-claude` | 6★ | 现代 | `laureate-albert-claude-skill` | Albert Claude；效果取自「免疫」（§3） |
| 1974 | 克里斯蒂安·德迪夫 | 随从 | `laureate-christian-de-duve` | 6★ | 现代 | `laureate-christian-de-duve-skill` | Christian de Duve；效果取自「遗传」（§3） |
| 1974 | 乔治·帕拉德 | 随从 | `laureate-george-e-palade` | 6★ | 现代 | `laureate-george-e-palade-skill` | George E. Palade；效果取自「神经」（§3） |
| 1975 | 戴维·巴尔的摩 | 随从 | `laureate-david-baltimore` | 6★ | 现代 | `laureate-david-baltimore-skill` | David Baltimore；效果取自「微生物」（§3） |
| 1975 | 霍华德·特明 | 随从 | `laureate-howard-m-temin` | 6★ | 现代 | `laureate-howard-m-temin-skill` | Howard M. Temin；效果取自「生理」（§3） |
| 1975 | 雷纳托·杜尔贝科 | 随从 | `laureate-renato-dulbecco` | 6★ | 现代 | `laureate-renato-dulbecco-skill` | Renato Dulbecco；效果取自「药理」（§3） |
| 1976 | 巴鲁克·布隆伯格 | 随从 | `laureate-baruch-s-blumberg` | 6★ | 现代 | `laureate-baruch-s-blumberg-skill` | Baruch S. Blumberg；效果取自「免疫」（§3） |
| 1976 | 卡尔顿·盖杜谢克 | 随从 | `laureate-d-carleton-gajdusek` | 6★ | 现代 | `laureate-d-carleton-gajdusek-skill` | D. Carleton Gajdusek；效果取自「遗传」（§3） |
| 1977 | 安德鲁·沙利 | 随从 | `laureate-andrew-v-schally` | 6★ | 现代 | `laureate-andrew-v-schally-skill` | Andrew V. Schally；效果取自「神经」（§3） |
| 1977 | 罗杰·吉耶曼 | 随从 | `laureate-roger-guillemin` | 6★ | 现代 | `laureate-roger-guillemin-skill` | Roger Guillemin；效果取自「微生物」（§3） |
| 1977 | 罗莎琳·雅洛 | 随从 | `laureate-rosalyn-yalow` | 6★ | 现代 | `laureate-rosalyn-yalow-skill` | Rosalyn Yalow；效果取自「生理」（§3） |
| 1978 | 丹尼尔·内森斯 | 随从 | `laureate-daniel-nathans` | 6★ | 现代 | `laureate-daniel-nathans-skill` | Daniel Nathans；效果取自「药理」（§3） |
| 1978 | 汉密尔顿·史密斯 | 随从 | `laureate-hamilton-o-smith` | 6★ | 现代 | `laureate-hamilton-o-smith-skill` | Hamilton O. Smith；效果取自「免疫」（§3） |
| 1978 | 维尔纳·阿尔伯 | 随从 | `laureate-werner-arber` | 6★ | 现代 | `laureate-werner-arber-skill` | Werner Arber；效果取自「遗传」（§3） |
| 1979 | 阿兰·科马克 | 随从 | `laureate-allan-m-cormack` | 6★ | 现代 | `laureate-allan-m-cormack-skill` | Allan M. Cormack；效果取自「神经」（§3） |
| 1979 | 戈弗雷·豪斯菲尔德 | 随从 | `laureate-godfrey-n-hounsfield` | 6★ | 现代 | `laureate-godfrey-n-hounsfield-skill` | Godfrey N. Hounsfield；效果取自「微生物」（§3） |
| 1980 | 巴鲁赫·贝纳塞拉夫 | 随从 | `laureate-baruj-benacerraf` | 6★ | 现代 | `laureate-baruj-benacerraf-skill` | Baruj Benacerraf；效果取自「生理」（§3） |
| 1980 | 乔治·斯内尔 | 随从 | `laureate-george-d-snell` | 6★ | 现代 | `laureate-george-d-snell-skill` | George D. Snell；效果取自「药理」（§3） |
| 1980 | 让·多塞 | 随从 | `laureate-jean-dausset` | 6★ | 现代 | `laureate-jean-dausset-skill` | Jean Dausset；效果取自「免疫」（§3） |
| 1981 | 戴维·休伯尔 | 随从 | `laureate-david-h-hubel` | 6★ | 现代 | `laureate-david-h-hubel-skill` | David H. Hubel；效果取自「遗传」（§3） |
| 1981 | 罗杰·斯佩里 | 随从 | `laureate-roger-w-sperry` | 6★ | 现代 | `laureate-roger-w-sperry-skill` | Roger W. Sperry；效果取自「神经」（§3） |
| 1981 | 托斯滕·维泽尔 | 随从 | `laureate-torsten-n-wiesel` | 6★ | 现代 | `laureate-torsten-n-wiesel-skill` | Torsten N. Wiesel；效果取自「微生物」（§3） |
| 1982 | 本特·萨穆埃尔松 | 随从 | `laureate-bengt-i-samuelsson` | 6★ | 现代 | `laureate-bengt-i-samuelsson-skill` | Bengt I. Samuelsson；效果取自「生理」（§3） |
| 1982 | 约翰·文 | 随从 | `laureate-john-r-vane` | 6★ | 现代 | `laureate-john-r-vane-skill` | John R. Vane；效果取自「药理」（§3） |
| 1982 | 苏内·贝里斯特伦 | 随从 | `laureate-sune-k-bergstrom` | 6★ | 现代 | `laureate-sune-k-bergstrom-skill` | Sune K. Bergström；效果取自「免疫」（§3） |
| 1983 | 芭芭拉·麦克林托克 | 随从 | `laureate-barbara-mcclintock` | 6★ | 现代 | `laureate-barbara-mcclintock-skill` | Barbara McClintock；效果取自「遗传」（§3） |
| 1984 | 塞萨尔·米尔斯坦 | 随从 | `laureate-cesar-milstein` | 6★ | 现代 | `laureate-cesar-milstein-skill` | César Milstein；效果取自「神经」（§3） |
| 1984 | 格奥尔格·克勒 | 随从 | `laureate-georges-j-f-kohler` | 6★ | 现代 | `laureate-georges-j-f-kohler-skill` | Georges J.F. Köhler；效果取自「微生物」（§3） |
| 1984 | 尼尔斯·耶纳 | 随从 | `laureate-niels-k-jerne` | 6★ | 现代 | `laureate-niels-k-jerne-skill` | Niels K. Jerne；效果取自「生理」（§3） |
| 1985 | 约瑟夫·戈德斯坦 | 随从 | `laureate-joseph-l-goldstein` | 6★ | 现代 | `laureate-joseph-l-goldstein-skill` | Joseph L. Goldstein；效果取自「药理」（§3） |
| 1985 | 迈克尔·布朗 | 随从 | `laureate-michael-s-brown` | 6★ | 现代 | `laureate-michael-s-brown-skill` | Michael S. Brown；效果取自「免疫」（§3） |
| 1986 | 丽塔·列维-蒙塔尔奇尼 | 随从 | `laureate-rita-levi-montalcini` | 6★ | 现代 | `laureate-rita-levi-montalcini-skill` | Rita Levi-Montalcini；效果取自「遗传」（§3） |
| 1986 | 斯坦利·科恩 | 随从 | `laureate-stanley-cohen` | 6★ | 现代 | `laureate-stanley-cohen-skill` | Stanley Cohen；效果取自「神经」（§3） |
| 1987 | 利根川进 | 随从 | `laureate-susumu-tonegawa` | 6★ | 现代 | `laureate-susumu-tonegawa-skill` | Susumu Tonegawa；效果取自「微生物」（§3） |
| 1988 | 乔治·希钦斯 | 随从 | `laureate-george-h-hitchings` | 6★ | 现代 | `laureate-george-h-hitchings-skill` | George H. Hitchings；效果取自「生理」（§3） |
| 1988 | 格特鲁德·埃利恩 | 随从 | `laureate-gertrude-b-elion` | 6★ | 现代 | `laureate-gertrude-b-elion-skill` | Gertrude B. Elion；效果取自「药理」（§3） |
| 1988 | 詹姆斯·布莱克 | 随从 | `laureate-sir-james-w-black` | 6★ | 现代 | `laureate-sir-james-w-black-skill` | Sir James W. Black；效果取自「免疫」（§3） |
| 1989 | 哈罗德·瓦尔姆斯 | 随从 | `laureate-harold-e-varmus` | 6★ | 现代 | `laureate-harold-e-varmus-skill` | Harold E. Varmus；效果取自「遗传」（§3） |
| 1989 | 迈克尔·毕晓普 | 随从 | `laureate-j-michael-bishop` | 6★ | 现代 | `laureate-j-michael-bishop-skill` | J. Michael Bishop；效果取自「神经」（§3） |
| 1990 | 唐纳尔·托马斯 | 随从 | `laureate-e-donnall-thomas` | 6★ | 现代 | `laureate-e-donnall-thomas-skill` | E. Donnall Thomas；效果取自「微生物」（§3） |
| 1990 | 约瑟夫·默里 | 随从 | `laureate-joseph-e-murray` | 6★ | 现代 | `laureate-joseph-e-murray-skill` | Joseph E. Murray；效果取自「生理」（§3） |
| 1991 | 伯特·萨克曼 | 随从 | `laureate-bert-sakmann` | 6★ | 现代 | `laureate-bert-sakmann-skill` | Bert Sakmann；效果取自「药理」（§3） |
| 1991 | 埃尔温·内尔 | 随从 | `laureate-erwin-neher` | 6★ | 现代 | `laureate-erwin-neher-skill` | Erwin Neher；效果取自「免疫」（§3） |
| 1992 | 埃德蒙·费歇尔 | 随从 | `laureate-edmond-h-fischer` | 6★ | 现代 | `laureate-edmond-h-fischer-skill` | Edmond H. Fischer；效果取自「遗传」（§3） |
| 1992 | 埃德温·克雷布斯 | 随从 | `laureate-edwin-g-krebs` | 6★ | 现代 | `laureate-edwin-g-krebs-skill` | Edwin G. Krebs；效果取自「神经」（§3） |
| 1993 | 菲利普·夏普 | 随从 | `laureate-phillip-a-sharp` | 6★ | 现代 | `laureate-phillip-a-sharp-skill` | Phillip A. Sharp；效果取自「微生物」（§3） |
| 1993 | 理查德·罗伯茨 | 随从 | `laureate-richard-j-roberts` | 6★ | 现代 | `laureate-richard-j-roberts-skill` | Richard J. Roberts；效果取自「生理」（§3） |
| 1994 | 阿尔弗雷德·吉尔曼 | 随从 | `laureate-alfred-g-gilman` | 6★ | 现代 | `laureate-alfred-g-gilman-skill` | Alfred G. Gilman；效果取自「药理」（§3） |
| 1994 | 马丁·罗德贝尔 | 随从 | `laureate-martin-rodbell` | 6★ | 现代 | `laureate-martin-rodbell-skill` | Martin Rodbell；效果取自「免疫」（§3） |
| 1995 | 克里斯蒂安娜·尼斯莱因-福尔哈德 | 随从 | `laureate-christiane-nusslein-volhard` | 6★ | 现代 | `laureate-christiane-nusslein-volhard-skill` | Christiane Nüsslein-Volhard；效果取自「遗传」（§3） |
| 1995 | 爱德华·刘易斯 | 随从 | `laureate-edward-b-lewis` | 6★ | 现代 | `laureate-edward-b-lewis-skill` | Edward B. Lewis；效果取自「神经」（§3） |
| 1995 | 埃里克·维绍斯 | 随从 | `laureate-eric-f-wieschaus` | 6★ | 现代 | `laureate-eric-f-wieschaus-skill` | Eric F. Wieschaus；效果取自「微生物」（§3） |
| 1996 | 彼得·多尔蒂 | 随从 | `laureate-peter-c-doherty` | 6★ | 现代 | `laureate-peter-c-doherty-skill` | Peter C. Doherty；效果取自「生理」（§3） |
| 1996 | 罗尔夫·青克纳格尔 | 随从 | `laureate-rolf-m-zinkernagel` | 6★ | 现代 | `laureate-rolf-m-zinkernagel-skill` | Rolf M. Zinkernagel；效果取自「药理」（§3） |
| 1997 | 斯坦利·普鲁西纳 | 随从 | `laureate-stanley-b-prusiner` | 6★ | 现代 | `laureate-stanley-b-prusiner-skill` | Stanley B. Prusiner；效果取自「免疫」（§3） |
| 1998 | 费里德·穆拉德 | 随从 | `laureate-ferid-murad` | 6★ | 现代 | `laureate-ferid-murad-skill` | Ferid Murad；效果取自「遗传」（§3） |
| 1998 | 路易斯·伊格纳罗 | 随从 | `laureate-louis-j-ignarro` | 6★ | 现代 | `laureate-louis-j-ignarro-skill` | Louis J. Ignarro；效果取自「神经」（§3） |
| 1998 | 罗伯特·弗奇戈特 | 随从 | `laureate-robert-f-furchgott` | 6★ | 现代 | `laureate-robert-f-furchgott-skill` | Robert F. Furchgott；效果取自「微生物」（§3） |
| 1999 | 京特·布洛贝尔 | 随从 | `laureate-gunter-blobel` | 6★ | 现代 | `laureate-gunter-blobel-skill` | Günter Blobel；效果取自「生理」（§3） |
| 2000 | 阿尔维德·卡尔松 | 随从 | `laureate-arvid-carlsson` | 7★ | 未来 | `laureate-arvid-carlsson-skill` | Arvid Carlsson；效果取自「药理」（§3） |
| 2000 | 埃里克·坎德尔 | 随从 | `laureate-eric-kandel` | 7★ | 未来 | `laureate-eric-kandel-skill` | Eric Kandel；效果取自「免疫」（§3） |
| 2000 | 保罗·格林加德 | 随从 | `laureate-paul-greengard` | 7★ | 未来 | `laureate-paul-greengard-skill` | Paul Greengard；效果取自「遗传」（§3） |
| 2001 | 利兰·哈特韦尔 | 随从 | `laureate-leland-hartwell` | 7★ | 未来 | `laureate-leland-hartwell-skill` | Leland Hartwell；效果取自「神经」（§3） |
| 2001 | 保罗·纳斯 | 随从 | `laureate-sir-paul-nurse` | 7★ | 未来 | `laureate-sir-paul-nurse-skill` | Sir Paul Nurse；效果取自「微生物」（§3） |
| 2001 | 蒂姆·亨特 | 随从 | `laureate-tim-hunt` | 7★ | 未来 | `laureate-tim-hunt-skill` | Tim Hunt；效果取自「生理」（§3） |
| 2002 | 罗伯特·霍维茨 | 随从 | `laureate-h-robert-horvitz` | 7★ | 未来 | `laureate-h-robert-horvitz-skill` | H. Robert Horvitz；效果取自「药理」（§3） |
| 2002 | 约翰·苏尔斯顿 | 随从 | `laureate-john-e-sulston` | 7★ | 未来 | `laureate-john-e-sulston-skill` | John E. Sulston；效果取自「免疫」（§3） |
| 2002 | 悉尼·布伦纳 | 随从 | `laureate-sydney-brenner` | 7★ | 未来 | `laureate-sydney-brenner-skill` | Sydney Brenner；效果取自「遗传」（§3） |
| 2003 | 保罗·劳特伯 | 随从 | `laureate-paul-c-lauterbur` | 7★ | 未来 | `laureate-paul-c-lauterbur-skill` | Paul C. Lauterbur；效果取自「神经」（§3） |
| 2003 | 彼得·曼斯菲尔德 | 随从 | `laureate-sir-peter-mansfield` | 7★ | 未来 | `laureate-sir-peter-mansfield-skill` | Sir Peter Mansfield；效果取自「微生物」（§3） |
| 2004 | 琳达·巴克 | 随从 | `laureate-linda-b-buck` | 7★ | 未来 | `laureate-linda-b-buck-skill` | Linda B. Buck；效果取自「生理」（§3） |
| 2004 | 理查德·阿克塞尔 | 随从 | `laureate-richard-axel` | 7★ | 未来 | `laureate-richard-axel-skill` | Richard Axel；效果取自「药理」（§3） |
| 2005 | 巴里·马歇尔 | 随从 | `laureate-barry-j-marshall` | 7★ | 未来 | `laureate-barry-j-marshall-skill` | Barry J. Marshall；效果取自「免疫」（§3） |
| 2005 | 罗宾·沃伦 | 随从 | `laureate-j-robin-warren` | 7★ | 未来 | `laureate-j-robin-warren-skill` | J. Robin Warren；效果取自「遗传」（§3） |
| 2006 | 安德鲁·法尔 | 随从 | `laureate-andrew-z-fire` | 7★ | 未来 | `laureate-andrew-z-fire-skill` | Andrew Z. Fire；效果取自「神经」（§3） |
| 2006 | 克雷格·梅洛 | 随从 | `laureate-craig-c-mello` | 7★ | 未来 | `laureate-craig-c-mello-skill` | Craig C. Mello；效果取自「微生物」（§3） |
| 2007 | 马里奥·卡佩奇 | 随从 | `laureate-mario-r-capecchi` | 7★ | 未来 | `laureate-mario-r-capecchi-skill` | Mario R. Capecchi；效果取自「生理」（§3） |
| 2007 | 奥利弗·史密斯 | 随从 | `laureate-oliver-smithies` | 7★ | 未来 | `laureate-oliver-smithies-skill` | Oliver Smithies；效果取自「药理」（§3） |
| 2007 | 马丁·埃文斯 | 随从 | `laureate-sir-martin-j-evans` | 7★ | 未来 | `laureate-sir-martin-j-evans-skill` | Sir Martin J. Evans；效果取自「免疫」（§3） |
| 2008 | 弗朗索瓦丝·巴尔-西诺西 | 随从 | `laureate-francoise-barre-sinoussi` | 7★ | 未来 | `laureate-francoise-barre-sinoussi-skill` | Françoise Barré-Sinoussi；效果取自「遗传」（§3） |
| 2008 | 哈拉尔德·楚尔·豪森 | 随从 | `laureate-harald-zur-hausen` | 7★ | 未来 | `laureate-harald-zur-hausen-skill` | Harald zur Hausen；效果取自「神经」（§3） |
| 2008 | 吕克·蒙塔尼耶 | 随从 | `laureate-luc-montagnier` | 7★ | 未来 | `laureate-luc-montagnier-skill` | Luc Montagnier；效果取自「微生物」（§3） |
| 2009 | 卡罗尔·格雷德 | 随从 | `laureate-carol-w-greider` | 7★ | 未来 | `laureate-carol-w-greider-skill` | Carol W. Greider；效果取自「生理」（§3） |
| 2009 | 伊丽莎白·布莱克本 | 随从 | `laureate-elizabeth-h-blackburn` | 7★ | 未来 | `laureate-elizabeth-h-blackburn-skill` | Elizabeth H. Blackburn；效果取自「药理」（§3） |
| 2009 | 杰克·绍斯塔克 | 随从 | `laureate-jack-w-szostak` | 7★ | 未来 | `laureate-jack-w-szostak-skill` | Jack W. Szostak；效果取自「免疫」（§3） |
| 2010 | 罗伯特·爱德华兹 | 随从 | `laureate-robert-g-edwards` | 7★ | 未来 | `laureate-robert-g-edwards-skill` | Robert G. Edwards；效果取自「遗传」（§3） |
| 2011 | 布鲁斯·博伊特勒 | 随从 | `laureate-bruce-a-beutler` | 7★ | 未来 | `laureate-bruce-a-beutler-skill` | Bruce A. Beutler；效果取自「神经」（§3） |
| 2011 | 朱尔·霍夫曼 | 随从 | `laureate-jules-a-hoffmann` | 7★ | 未来 | `laureate-jules-a-hoffmann-skill` | Jules A. Hoffmann；效果取自「微生物」（§3） |
| 2011 | 拉尔夫·斯坦曼 | 随从 | `laureate-ralph-m-steinman` | 7★ | 未来 | `laureate-ralph-m-steinman-skill` | Ralph M. Steinman；效果取自「生理」（§3） |
| 2012 | 山中伸弥 | 随从 | `laureate-shinya-yamanaka` | 7★ | 未来 | `laureate-shinya-yamanaka-skill` | Shinya Yamanaka；效果取自「药理」（§3） |
| 2012 | 约翰·格登 | 随从 | `laureate-sir-john-b-gurdon` | 7★ | 未来 | `laureate-sir-john-b-gurdon-skill` | Sir John B. Gurdon；效果取自「免疫」（§3） |
| 2013 | 詹姆斯·罗斯曼 | 随从 | `laureate-james-e-rothman` | 7★ | 未来 | `laureate-james-e-rothman-skill` | James E. Rothman；效果取自「遗传」（§3） |
| 2013 | 兰迪·谢克曼 | 随从 | `laureate-randy-w-schekman` | 7★ | 未来 | `laureate-randy-w-schekman-skill` | Randy W. Schekman；效果取自「神经」（§3） |
| 2013 | 托马斯·聚德霍夫 | 随从 | `laureate-thomas-c-sudhof` | 7★ | 未来 | `laureate-thomas-c-sudhof-skill` | Thomas C. Südhof；效果取自「微生物」（§3） |
| 2014 | 爱德华·莫泽 | 随从 | `laureate-edvard-i-moser` | 7★ | 未来 | `laureate-edvard-i-moser-skill` | Edvard I. Moser；效果取自「生理」（§3） |
| 2014 | 约翰·奥基夫 | 随从 | `laureate-john-o-keefe` | 7★ | 未来 | `laureate-john-o-keefe-skill` | John O'Keefe；效果取自「药理」（§3） |
| 2014 | 迈-布里特·莫泽 | 随从 | `laureate-may-britt-moser` | 7★ | 未来 | `laureate-may-britt-moser-skill` | May-Britt Moser；效果取自「免疫」（§3） |
| 2015 | 大村智 | 随从 | `laureate-satoshi-omura` | 7★ | 未来 | `laureate-satoshi-omura-skill` | Satoshi Ōmura；效果取自「遗传」（§3） |
| 2015 | 屠呦呦 | 随从 | `laureate-tu-youyou` | 7★ | 未来 | `laureate-tu-youyou-skill` | Tu Youyou；效果取自「神经」（§3） |
| 2015 | 威廉·坎贝尔 | 随从 | `laureate-william-c-campbell` | 7★ | 未来 | `laureate-william-c-campbell-skill` | William C. Campbell；效果取自「微生物」（§3） |
| 2016 | 大隅良典 | 随从 | `laureate-yoshinori-ohsumi` | 7★ | 未来 | `laureate-yoshinori-ohsumi-skill` | Yoshinori Ohsumi；效果取自「生理」（§3） |
| 2017 | 杰弗里·霍尔 | 随从 | `laureate-jeffrey-c-hall` | 7★ | 未来 | `laureate-jeffrey-c-hall-skill` | Jeffrey C. Hall；效果取自「药理」（§3） |
| 2017 | 迈克尔·罗斯巴什 | 随从 | `laureate-michael-rosbash` | 7★ | 未来 | `laureate-michael-rosbash-skill` | Michael Rosbash；效果取自「免疫」（§3） |
| 2017 | 迈克尔·扬 | 随从 | `laureate-michael-w-young` | 7★ | 未来 | `laureate-michael-w-young-skill` | Michael W. Young；效果取自「遗传」（§3） |
| 2018 | 詹姆斯·艾利森 | 随从 | `laureate-james-p-allison` | 7★ | 未来 | `laureate-james-p-allison-skill` | James P. Allison；效果取自「神经」（§3） |
| 2018 | 本庶佑 | 随从 | `laureate-tasuku-honjo` | 7★ | 未来 | `laureate-tasuku-honjo-skill` | Tasuku Honjo；效果取自「微生物」（§3） |
| 2019 | 格雷格·塞门扎 | 随从 | `laureate-gregg-l-semenza` | 7★ | 未来 | `laureate-gregg-l-semenza-skill` | Gregg L. Semenza；效果取自「生理」（§3） |
| 2019 | 彼得·拉特克利夫 | 随从 | `laureate-sir-peter-j-ratcliffe` | 7★ | 未来 | `laureate-sir-peter-j-ratcliffe-skill` | Sir Peter J. Ratcliffe；效果取自「药理」（§3） |
| 2019 | 小威廉·凯林 | 随从 | `laureate-william-g-kaelin-jr` | 7★ | 未来 | `laureate-william-g-kaelin-jr-skill` | William G. Kaelin Jr；效果取自「免疫」（§3） |
| 2020 | 查尔斯·赖斯 | 随从 | `laureate-charles-m-rice` | 7★ | 未来 | `laureate-charles-m-rice-skill` | Charles M. Rice；效果取自「遗传」（§3） |
| 2020 | 哈维·奥尔特 | 随从 | `laureate-harvey-j-alter` | 7★ | 未来 | `laureate-harvey-j-alter-skill` | Harvey J. Alter；效果取自「神经」（§3） |
| 2020 | 迈克尔·霍顿 | 随从 | `laureate-michael-houghton` | 7★ | 未来 | `laureate-michael-houghton-skill` | Michael Houghton；效果取自「微生物」（§3） |
| 2021 | 阿德姆·帕塔普蒂安 | 随从 | `laureate-ardem-patapoutian` | 7★ | 未来 | `laureate-ardem-patapoutian-skill` | Ardem Patapoutian；效果取自「生理」（§3） |
| 2021 | 戴维·朱利叶斯 | 随从 | `laureate-david-julius` | 7★ | 未来 | `laureate-david-julius-skill` | David Julius；效果取自「药理」（§3） |
| 2022 | 斯万特·帕博 | 随从 | `laureate-svante-paabo` | 7★ | 未来 | `laureate-svante-paabo-skill` | Svante Pääbo；效果取自「免疫」（§3） |
| 2023 | 德鲁·韦斯曼 | 随从 | `laureate-drew-weissman` | 7★ | 未来 | `laureate-drew-weissman-skill` | Drew Weissman；效果取自「遗传」（§3） |
| 2023 | 卡塔琳·考里科 | 随从 | `laureate-katalin-kariko` | 7★ | 未来 | `laureate-katalin-kariko-skill` | Katalin Karikó；效果取自「神经」（§3） |
| 2024 | 加里·鲁夫昆 | 随从 | `laureate-gary-ruvkun` | 7★ | 未来 | `laureate-gary-ruvkun-skill` | Gary Ruvkun；效果取自「微生物」（§3） |
| 2024 | 维克托·安布罗斯 | 随从 | `laureate-victor-ambros` | 7★ | 未来 | `laureate-victor-ambros-skill` | Victor Ambros；效果取自「生理」（§3） |
| 2025 | 弗雷德·拉姆斯德尔 | 随从 | `laureate-fred-ramsdell` | 7★ | 未来 | `laureate-fred-ramsdell-skill` | Fred Ramsdell；效果取自「药理」（§3） |
| 2025 | 玛丽·布伦科 | 随从 | `laureate-mary-e-brunkow` | 7★ | 未来 | `laureate-mary-e-brunkow-skill` | Mary E. Brunkow；效果取自「免疫」（§3） |
| 2025 | 坂口志文 | 随从 | `laureate-shimon-sakaguchi` | 7★ | 未来 | `laureate-shimon-sakaguchi-skill` | Shimon Sakaguchi；效果取自「遗传」（§3） |
| 2026 | 格奥尔格·纳格尔 | 随从 | `laureate-georg-nagel` | 7★ | 未来 | `laureate-georg-nagel-skill` | Georg Nagel；效果取自「神经」（§3） |
| 2026 | 卡尔·戴瑟罗斯 | 随从 | `laureate-karl-deisseroth` | 7★ | 未来 | `laureate-karl-deisseroth-skill` | Karl Deisseroth；效果取自「微生物」（§3） |
| 2026 | 彼得·黑格曼 | 随从 | `laureate-peter-hegemann` | 7★ | 未来 | `laureate-peter-hegemann-skill` | Peter Hegemann；效果取自「生理」（§3） |

#### 5.4 文学奖（123）

| 年份 | 得主 | 类型 | 卡牌 id | 星级/费用 | 时代 | 技能 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1901 | 苏利·普吕多姆 | 效果 | `laureate-sully-prudhomme` | 2 费 | 工业 | `laureate-sully-prudhomme-skill` | Sully Prudhomme；效果取自「史诗」（§3） |
| 1902 | 特奥多尔·蒙森 | 效果 | `laureate-theodor-mommsen` | 2 费 | 工业 | `laureate-theodor-mommsen-skill` | Theodor Mommsen；效果取自「抒情」（§3） |
| 1903 | 比约恩斯彻纳·比约恩松 | 效果 | `laureate-bjornstjerne-bjornson` | 2 费 | 工业 | `laureate-bjornstjerne-bjornson-skill` | Bjørnstjerne Bjørnson；效果取自「讽刺」（§3） |
| 1904 | 弗雷德里克·米斯特拉尔 | 效果 | `laureate-frederic-mistral` | 2 费 | 工业 | `laureate-frederic-mistral-skill` | Frédéric Mistral；效果取自「写实」（§3） |
| 1904 | 何塞·埃切加赖 | 效果 | `laureate-jose-echegaray` | 2 费 | 工业 | `laureate-jose-echegaray-skill` | José Echegaray；效果取自「现代主义」（§3） |
| 1905 | 亨利克·显克维奇 | 效果 | `laureate-henryk-sienkiewicz` | 2 费 | 工业 | `laureate-henryk-sienkiewicz-skill` | Henryk Sienkiewicz；效果取自「记忆」（§3） |
| 1906 | 焦苏埃·卡尔杜奇 | 效果 | `laureate-giosue-carducci` | 2 费 | 工业 | `laureate-giosue-carducci-skill` | Giosuè Carducci；效果取自「史诗」（§3） |
| 1907 | 鲁德亚德·吉卜林 | 效果 | `laureate-rudyard-kipling` | 2 费 | 工业 | `laureate-rudyard-kipling-skill` | Rudyard Kipling；效果取自「抒情」（§3） |
| 1908 | 鲁道夫·奥伊肯 | 效果 | `laureate-rudolf-eucken` | 2 费 | 工业 | `laureate-rudolf-eucken-skill` | Rudolf Eucken；效果取自「讽刺」（§3） |
| 1909 | 塞尔玛·拉格洛夫 | 效果 | `laureate-selma-lagerlof` | 2 费 | 工业 | `laureate-selma-lagerlof-skill` | Selma Lagerlöf；效果取自「写实」（§3） |
| 1910 | 保罗·海泽 | 效果 | `laureate-paul-heyse` | 2 费 | 工业 | `laureate-paul-heyse-skill` | Paul Heyse；效果取自「现代主义」（§3） |
| 1911 | 莫里斯·梅特林克 | 效果 | `laureate-maurice-maeterlinck` | 2 费 | 工业 | `laureate-maurice-maeterlinck-skill` | Maurice Maeterlinck；效果取自「记忆」（§3） |
| 1912 | 格哈德·豪普特曼 | 效果 | `laureate-gerhart-hauptmann` | 2 费 | 工业 | `laureate-gerhart-hauptmann-skill` | Gerhart Hauptmann；效果取自「史诗」（§3） |
| 1913 | 拉宾德拉纳特·泰戈尔 | 效果 | `laureate-rabindranath-tagore` | 2 费 | 工业 | `laureate-rabindranath-tagore-skill` | Rabindranath Tagore；效果取自「抒情」（§3） |
| 1915 | 罗曼·罗兰 | 效果 | `laureate-romain-rolland` | 2 费 | 工业 | `laureate-romain-rolland-skill` | Romain Rolland；效果取自「讽刺」（§3） |
| 1916 | 维尔纳·冯·海登斯塔姆 | 效果 | `laureate-verner-von-heidenstam` | 2 费 | 工业 | `laureate-verner-von-heidenstam-skill` | Verner von Heidenstam；效果取自「写实」（§3） |
| 1917 | 亨里克·彭托皮丹 | 效果 | `laureate-henrik-pontoppidan` | 2 费 | 工业 | `laureate-henrik-pontoppidan-skill` | Henrik Pontoppidan；效果取自「现代主义」（§3） |
| 1917 | 卡尔·吉勒鲁普 | 效果 | `laureate-karl-gjellerup` | 2 费 | 工业 | `laureate-karl-gjellerup-skill` | Karl Gjellerup；效果取自「记忆」（§3） |
| 1919 | 卡尔·施皮特勒 | 效果 | `laureate-carl-spitteler` | 2 费 | 工业 | `laureate-carl-spitteler-skill` | Carl Spitteler；效果取自「史诗」（§3） |
| 1920 | 克努特·汉姆生 | 效果 | `laureate-knut-hamsun` | 2 费 | 工业 | `laureate-knut-hamsun-skill` | Knut Hamsun；效果取自「抒情」（§3） |
| 1921 | 阿纳托尔·法朗士 | 效果 | `laureate-anatole-france` | 2 费 | 工业 | `laureate-anatole-france-skill` | Anatole France；效果取自「讽刺」（§3） |
| 1922 | 哈辛托·贝纳文特 | 效果 | `laureate-jacinto-benavente` | 2 费 | 工业 | `laureate-jacinto-benavente-skill` | Jacinto Benavente；效果取自「写实」（§3） |
| 1923 | 威廉·巴特勒·叶芝 | 效果 | `laureate-william-butler-yeats` | 2 费 | 工业 | `laureate-william-butler-yeats-skill` | William Butler Yeats；效果取自「现代主义」（§3） |
| 1924 | 瓦迪斯瓦夫·雷蒙特 | 效果 | `laureate-wladyslaw-reymont` | 2 费 | 工业 | `laureate-wladyslaw-reymont-skill` | Władysław Reymont；效果取自「记忆」（§3） |
| 1925 | 乔治·萧伯纳 | 效果 | `laureate-george-bernard-shaw` | 2 费 | 工业 | `laureate-george-bernard-shaw-skill` | George Bernard Shaw；效果取自「史诗」（§3） |
| 1926 | 格拉齐亚·黛莱达 | 效果 | `laureate-grazia-deledda` | 2 费 | 工业 | `laureate-grazia-deledda-skill` | Grazia Deledda；效果取自「抒情」（§3） |
| 1927 | 亨利·柏格森 | 效果 | `laureate-henri-bergson` | 2 费 | 工业 | `laureate-henri-bergson-skill` | Henri Bergson；效果取自「讽刺」（§3） |
| 1928 | 西格丽德·温塞特 | 效果 | `laureate-sigrid-undset` | 2 费 | 工业 | `laureate-sigrid-undset-skill` | Sigrid Undset；效果取自「写实」（§3） |
| 1929 | 托马斯·曼 | 效果 | `laureate-thomas-mann` | 2 费 | 工业 | `laureate-thomas-mann-skill` | Thomas Mann；效果取自「现代主义」（§3） |
| 1930 | 辛克莱·刘易斯 | 效果 | `laureate-sinclair-lewis` | 2 费 | 工业 | `laureate-sinclair-lewis-skill` | Sinclair Lewis；效果取自「记忆」（§3） |
| 1931 | 埃里克·阿克塞尔·卡尔费尔特 | 效果 | `laureate-erik-axel-karlfeldt` | 2 费 | 工业 | `laureate-erik-axel-karlfeldt-skill` | Erik Axel Karlfeldt；效果取自「史诗」（§3） |
| 1932 | 约翰·高尔斯华绥 | 效果 | `laureate-john-galsworthy` | 2 费 | 工业 | `laureate-john-galsworthy-skill` | John Galsworthy；效果取自「抒情」（§3） |
| 1933 | 伊万·布宁 | 效果 | `laureate-ivan-bunin` | 2 费 | 工业 | `laureate-ivan-bunin-skill` | Ivan Bunin；效果取自「讽刺」（§3） |
| 1934 | 路易吉·皮兰德娄 | 效果 | `laureate-luigi-pirandello` | 2 费 | 工业 | `laureate-luigi-pirandello-skill` | Luigi Pirandello；效果取自「写实」（§3） |
| 1936 | 尤金·奥尼尔 | 效果 | `laureate-eugene-o-neill` | 2 费 | 工业 | `laureate-eugene-o-neill-skill` | Eugene O'Neill；效果取自「现代主义」（§3） |
| 1937 | 罗杰·马丁·杜加尔 | 效果 | `laureate-roger-martin-du-gard` | 2 费 | 工业 | `laureate-roger-martin-du-gard-skill` | Roger Martin du Gard；效果取自「记忆」（§3） |
| 1938 | 赛珍珠 | 效果 | `laureate-pearl-buck` | 2 费 | 工业 | `laureate-pearl-buck-skill` | Pearl Buck；效果取自「史诗」（§3） |
| 1939 | 弗兰斯·埃米尔·西兰帕 | 效果 | `laureate-frans-eemil-sillanpaa` | 2 费 | 工业 | `laureate-frans-eemil-sillanpaa-skill` | Frans Eemil Sillanpää；效果取自「抒情」（§3） |
| 1944 | 约翰内斯·延森 | 效果 | `laureate-johannes-v-jensen` | 3 费 | 工业 | `laureate-johannes-v-jensen-skill` | Johannes V. Jensen；效果取自「讽刺」（§3） |
| 1945 | 加夫列拉·米斯特拉尔 | 效果 | `laureate-gabriela-mistral` | 3 费 | 工业 | `laureate-gabriela-mistral-skill` | Gabriela Mistral；效果取自「写实」（§3） |
| 1946 | 赫尔曼·黑塞 | 效果 | `laureate-hermann-hesse` | 3 费 | 现代 | `laureate-hermann-hesse-skill` | Hermann Hesse；效果取自「现代主义」（§3） |
| 1947 | 安德烈·纪德 | 效果 | `laureate-andre-gide` | 3 费 | 现代 | `laureate-andre-gide-skill` | André Gide；效果取自「记忆」（§3） |
| 1948 | 托·斯·艾略特 | 效果 | `laureate-t-s-eliot` | 3 费 | 现代 | `laureate-t-s-eliot-skill` | T.S. Eliot；效果取自「史诗」（§3） |
| 1949 | 威廉·福克纳 | 效果 | `laureate-william-faulkner` | 3 费 | 现代 | `laureate-william-faulkner-skill` | William Faulkner；效果取自「抒情」（§3） |
| 1950 | 伯特兰·罗素 | 效果 | `laureate-bertrand-russell` | 3 费 | 现代 | `laureate-bertrand-russell-skill` | Bertrand Russell；效果取自「讽刺」（§3） |
| 1951 | 佩尔·拉格奎斯特 | 效果 | `laureate-par-lagerkvist` | 3 费 | 现代 | `laureate-par-lagerkvist-skill` | Pär Lagerkvist；效果取自「写实」（§3） |
| 1952 | 弗朗索瓦·莫里亚克 | 效果 | `laureate-francois-mauriac` | 3 费 | 现代 | `laureate-francois-mauriac-skill` | François Mauriac；效果取自「现代主义」（§3） |
| 1953 | 温斯顿·丘吉尔 | 效果 | `laureate-winston-churchill` | 3 费 | 现代 | `laureate-winston-churchill-skill` | Winston Churchill；效果取自「记忆」（§3） |
| 1954 | 欧内斯特·海明威 | 效果 | `laureate-ernest-hemingway` | 3 费 | 现代 | `laureate-ernest-hemingway-skill` | Ernest Hemingway；效果取自「史诗」（§3） |
| 1955 | 哈尔多尔·拉克斯内斯 | 效果 | `laureate-halldor-laxness` | 3 费 | 现代 | `laureate-halldor-laxness-skill` | Halldór Laxness；效果取自「抒情」（§3） |
| 1956 | 胡安·拉蒙·希梅内斯 | 效果 | `laureate-juan-ramon-jimenez` | 3 费 | 现代 | `laureate-juan-ramon-jimenez-skill` | Juan Ramón Jiménez；效果取自「讽刺」（§3） |
| 1957 | 阿尔贝·加缪 | 效果 | `laureate-albert-camus` | 3 费 | 现代 | `laureate-albert-camus-skill` | Albert Camus；效果取自「写实」（§3） |
| 1958 | 鲍里斯·帕斯捷尔纳克 | 效果 | `laureate-boris-pasternak` | 3 费 | 现代 | `laureate-boris-pasternak-skill` | Boris Pasternak；效果取自「现代主义」（§3） |
| 1959 | 萨尔瓦托雷·夸西莫多 | 效果 | `laureate-salvatore-quasimodo` | 3 费 | 现代 | `laureate-salvatore-quasimodo-skill` | Salvatore Quasimodo；效果取自「记忆」（§3） |
| 1960 | 圣-琼·佩斯 | 效果 | `laureate-saint-john-perse` | 3 费 | 现代 | `laureate-saint-john-perse-skill` | Saint-John Perse；效果取自「史诗」（§3） |
| 1961 | 伊沃·安德里奇 | 效果 | `laureate-ivo-andric` | 3 费 | 现代 | `laureate-ivo-andric-skill` | Ivo Andrić；效果取自「抒情」（§3） |
| 1962 | 约翰·斯坦贝克 | 效果 | `laureate-john-steinbeck` | 3 费 | 现代 | `laureate-john-steinbeck-skill` | John Steinbeck；效果取自「讽刺」（§3） |
| 1963 | 乔治·塞菲里斯 | 效果 | `laureate-giorgos-seferis` | 3 费 | 现代 | `laureate-giorgos-seferis-skill` | Giorgos Seferis；效果取自「写实」（§3） |
| 1964 | 让-保罗·萨特 | 效果 | `laureate-jean-paul-sartre` | 3 费 | 现代 | `laureate-jean-paul-sartre-skill` | Jean-Paul Sartre；效果取自「现代主义」（§3） |
| 1965 | 米哈伊尔·肖洛霍夫 | 效果 | `laureate-mikhail-sholokhov` | 3 费 | 现代 | `laureate-mikhail-sholokhov-skill` | Mikhail Sholokhov；效果取自「记忆」（§3） |
| 1966 | 内莉·萨克斯 | 效果 | `laureate-nelly-sachs` | 3 费 | 现代 | `laureate-nelly-sachs-skill` | Nelly Sachs；效果取自「史诗」（§3） |
| 1966 | 什穆埃尔·阿格农 | 效果 | `laureate-shmuel-agnon` | 3 费 | 现代 | `laureate-shmuel-agnon-skill` | Shmuel Agnon；效果取自「抒情」（§3） |
| 1967 | 米格尔·安赫尔·阿斯图里亚斯 | 效果 | `laureate-miguel-angel-asturias` | 3 费 | 现代 | `laureate-miguel-angel-asturias-skill` | Miguel Angel Asturias；效果取自「讽刺」（§3） |
| 1968 | 川端康成 | 效果 | `laureate-yasunari-kawabata` | 3 费 | 现代 | `laureate-yasunari-kawabata-skill` | Yasunari Kawabata；效果取自「写实」（§3） |
| 1969 | 塞缪尔·贝克特 | 效果 | `laureate-samuel-beckett` | 3 费 | 现代 | `laureate-samuel-beckett-skill` | Samuel Beckett；效果取自「现代主义」（§3） |
| 1970 | 亚历山大·索尔仁尼琴 | 效果 | `laureate-aleksandr-solzhenitsyn` | 3 费 | 现代 | `laureate-aleksandr-solzhenitsyn-skill` | Aleksandr Solzhenitsyn；效果取自「记忆」（§3） |
| 1971 | 巴勃罗·聂鲁达 | 效果 | `laureate-pablo-neruda` | 3 费 | 现代 | `laureate-pablo-neruda-skill` | Pablo Neruda；效果取自「史诗」（§3） |
| 1972 | 海因里希·伯尔 | 效果 | `laureate-heinrich-boll` | 3 费 | 现代 | `laureate-heinrich-boll-skill` | Heinrich Böll；效果取自「抒情」（§3） |
| 1973 | 帕特里克·怀特 | 效果 | `laureate-patrick-white` | 3 费 | 现代 | `laureate-patrick-white-skill` | Patrick White；效果取自「讽刺」（§3） |
| 1974 | 埃温德·约翰松 | 效果 | `laureate-eyvind-johnson` | 3 费 | 现代 | `laureate-eyvind-johnson-skill` | Eyvind Johnson；效果取自「写实」（§3） |
| 1974 | 哈里·马丁松 | 效果 | `laureate-harry-martinson` | 3 费 | 现代 | `laureate-harry-martinson-skill` | Harry Martinson；效果取自「现代主义」（§3） |
| 1975 | 埃乌杰尼奥·蒙塔莱 | 效果 | `laureate-eugenio-montale` | 3 费 | 现代 | `laureate-eugenio-montale-skill` | Eugenio Montale；效果取自「记忆」（§3） |
| 1976 | 索尔·贝娄 | 效果 | `laureate-saul-bellow` | 3 费 | 现代 | `laureate-saul-bellow-skill` | Saul Bellow；效果取自「史诗」（§3） |
| 1977 | 维森特·阿莱克桑德雷 | 效果 | `laureate-vicente-aleixandre` | 3 费 | 现代 | `laureate-vicente-aleixandre-skill` | Vicente Aleixandre；效果取自「抒情」（§3） |
| 1978 | 艾萨克·巴什维斯·辛格 | 效果 | `laureate-isaac-bashevis-singer` | 3 费 | 现代 | `laureate-isaac-bashevis-singer-skill` | Isaac Bashevis Singer；效果取自「讽刺」（§3） |
| 1979 | 奥季塞夫斯·埃利蒂斯 | 效果 | `laureate-odysseus-elytis` | 3 费 | 现代 | `laureate-odysseus-elytis-skill` | Odysseus Elytis；效果取自「写实」（§3） |
| 1980 | 切斯瓦夫·米沃什 | 效果 | `laureate-czeslaw-milosz` | 3 费 | 现代 | `laureate-czeslaw-milosz-skill` | Czesław Miłosz；效果取自「现代主义」（§3） |
| 1981 | 埃利亚斯·卡内蒂 | 效果 | `laureate-elias-canetti` | 3 费 | 现代 | `laureate-elias-canetti-skill` | Elias Canetti；效果取自「记忆」（§3） |
| 1982 | 加夫列尔·加西亚·马尔克斯 | 效果 | `laureate-gabriel-garcia-marquez` | 3 费 | 现代 | `laureate-gabriel-garcia-marquez-skill` | Gabriel García Márquez；效果取自「史诗」（§3） |
| 1983 | 威廉·戈尔丁 | 效果 | `laureate-william-golding` | 3 费 | 现代 | `laureate-william-golding-skill` | William Golding；效果取自「抒情」（§3） |
| 1984 | 雅罗斯拉夫·塞弗尔特 | 效果 | `laureate-jaroslav-seifert` | 3 费 | 现代 | `laureate-jaroslav-seifert-skill` | Jaroslav Seifert；效果取自「讽刺」（§3） |
| 1985 | 克洛德·西蒙 | 效果 | `laureate-claude-simon` | 3 费 | 现代 | `laureate-claude-simon-skill` | Claude Simon；效果取自「写实」（§3） |
| 1986 | 沃莱·索因卡 | 效果 | `laureate-wole-soyinka` | 3 费 | 现代 | `laureate-wole-soyinka-skill` | Wole Soyinka；效果取自「现代主义」（§3） |
| 1987 | 约瑟夫·布罗茨基 | 效果 | `laureate-joseph-brodsky` | 3 费 | 现代 | `laureate-joseph-brodsky-skill` | Joseph Brodsky；效果取自「记忆」（§3） |
| 1988 | 纳吉布·马哈福兹 | 效果 | `laureate-naguib-mahfouz` | 3 费 | 现代 | `laureate-naguib-mahfouz-skill` | Naguib Mahfouz；效果取自「史诗」（§3） |
| 1989 | 卡米洛·何塞·塞拉 | 效果 | `laureate-camilo-jose-cela` | 3 费 | 现代 | `laureate-camilo-jose-cela-skill` | Camilo José Cela；效果取自「抒情」（§3） |
| 1990 | 奥克塔维奥·帕斯 | 效果 | `laureate-octavio-paz` | 3 费 | 现代 | `laureate-octavio-paz-skill` | Octavio Paz；效果取自「讽刺」（§3） |
| 1991 | 纳丁·戈迪默 | 效果 | `laureate-nadine-gordimer` | 3 费 | 现代 | `laureate-nadine-gordimer-skill` | Nadine Gordimer；效果取自「写实」（§3） |
| 1992 | 德里克·沃尔科特 | 效果 | `laureate-derek-walcott` | 3 费 | 现代 | `laureate-derek-walcott-skill` | Derek Walcott；效果取自「现代主义」（§3） |
| 1993 | 托妮·莫里森 | 效果 | `laureate-toni-morrison` | 3 费 | 现代 | `laureate-toni-morrison-skill` | Toni Morrison；效果取自「记忆」（§3） |
| 1994 | 大江健三郎 | 效果 | `laureate-kenzaburo-oe` | 3 费 | 现代 | `laureate-kenzaburo-oe-skill` | Kenzaburo Oe；效果取自「史诗」（§3） |
| 1995 | 谢默斯·希尼 | 效果 | `laureate-seamus-heaney` | 3 费 | 现代 | `laureate-seamus-heaney-skill` | Seamus Heaney；效果取自「抒情」（§3） |
| 1996 | 维斯瓦娃·辛波丝卡 | 效果 | `laureate-wislawa-szymborska` | 3 费 | 现代 | `laureate-wislawa-szymborska-skill` | Wisława Szymborska；效果取自「讽刺」（§3） |
| 1997 | 达里奥·福 | 效果 | `laureate-dario-fo` | 3 费 | 现代 | `laureate-dario-fo-skill` | Dario Fo；效果取自「写实」（§3） |
| 1998 | 若泽·萨拉马戈 | 效果 | `laureate-jose-saramago` | 3 费 | 现代 | `laureate-jose-saramago-skill` | José Saramago；效果取自「现代主义」（§3） |
| 1999 | 君特·格拉斯 | 效果 | `laureate-gunter-grass` | 3 费 | 现代 | `laureate-gunter-grass-skill` | Günter Grass；效果取自「记忆」（§3） |
| 2000 | 高行健 | 效果 | `laureate-gao-xingjian` | 4 费 | 未来 | `laureate-gao-xingjian-skill` | Gao Xingjian；效果取自「史诗」（§3） |
| 2001 | 维·苏·奈保尔 | 效果 | `laureate-v-s-naipaul` | 4 费 | 未来 | `laureate-v-s-naipaul-skill` | V. S. Naipaul；效果取自「抒情」（§3） |
| 2002 | 伊姆雷·凯尔泰斯 | 效果 | `laureate-imre-kertesz` | 4 费 | 未来 | `laureate-imre-kertesz-skill` | Imre Kertész；效果取自「讽刺」（§3） |
| 2003 | 约·马·库切 | 效果 | `laureate-j-m-coetzee` | 4 费 | 未来 | `laureate-j-m-coetzee-skill` | J. M. Coetzee；效果取自「写实」（§3） |
| 2004 | 埃尔弗里德·耶利内克 | 效果 | `laureate-elfriede-jelinek` | 4 费 | 未来 | `laureate-elfriede-jelinek-skill` | Elfriede Jelinek；效果取自「现代主义」（§3） |
| 2005 | 哈罗德·品特 | 效果 | `laureate-harold-pinter` | 4 费 | 未来 | `laureate-harold-pinter-skill` | Harold Pinter；效果取自「记忆」（§3） |
| 2006 | 奥尔罕·帕慕克 | 效果 | `laureate-orhan-pamuk` | 4 费 | 未来 | `laureate-orhan-pamuk-skill` | Orhan Pamuk；效果取自「史诗」（§3） |
| 2007 | 多丽丝·莱辛 | 效果 | `laureate-doris-lessing` | 4 费 | 未来 | `laureate-doris-lessing-skill` | Doris Lessing；效果取自「抒情」（§3） |
| 2008 | 让-马里·古斯塔夫·勒克莱齐奥 | 效果 | `laureate-jean-marie-gustave-le-clezio` | 4 费 | 未来 | `laureate-jean-marie-gustave-le-clezio-skill` | Jean-Marie Gustave Le Clézio；效果取自「讽刺」（§3） |
| 2009 | 赫塔·米勒 | 效果 | `laureate-herta-muller` | 4 费 | 未来 | `laureate-herta-muller-skill` | Herta Müller；效果取自「写实」（§3） |
| 2010 | 马里奥·巴尔加斯·略萨 | 效果 | `laureate-mario-vargas-llosa` | 4 费 | 未来 | `laureate-mario-vargas-llosa-skill` | Mario Vargas Llosa；效果取自「现代主义」（§3） |
| 2011 | 托马斯·特朗斯特罗姆 | 效果 | `laureate-tomas-transtromer` | 4 费 | 未来 | `laureate-tomas-transtromer-skill` | Tomas Tranströmer；效果取自「记忆」（§3） |
| 2012 | 莫言 | 效果 | `laureate-mo-yan` | 4 费 | 未来 | `laureate-mo-yan-skill` | Mo Yan；效果取自「史诗」（§3） |
| 2013 | 艾丽斯·芒罗 | 效果 | `laureate-alice-munro` | 4 费 | 未来 | `laureate-alice-munro-skill` | Alice Munro；效果取自「抒情」（§3） |
| 2014 | 帕特里克·莫迪亚诺 | 效果 | `laureate-patrick-modiano` | 4 费 | 未来 | `laureate-patrick-modiano-skill` | Patrick Modiano；效果取自「讽刺」（§3） |
| 2015 | 斯韦特兰娜·阿列克谢耶维奇 | 效果 | `laureate-svetlana-alexievich` | 4 费 | 未来 | `laureate-svetlana-alexievich-skill` | Svetlana Alexievich；效果取自「写实」（§3） |
| 2016 | 鲍勃·迪伦 | 效果 | `laureate-bob-dylan` | 4 费 | 未来 | `laureate-bob-dylan-skill` | Bob Dylan；效果取自「现代主义」（§3） |
| 2017 | 石黑一雄 | 效果 | `laureate-kazuo-ishiguro` | 4 费 | 未来 | `laureate-kazuo-ishiguro-skill` | Kazuo Ishiguro；效果取自「记忆」（§3） |
| 2018 | 奥尔加·托卡尔丘克 | 效果 | `laureate-olga-tokarczuk` | 4 费 | 未来 | `laureate-olga-tokarczuk-skill` | Olga Tokarczuk；效果取自「史诗」（§3） |
| 2019 | 彼得·汉德克 | 效果 | `laureate-peter-handke` | 4 费 | 未来 | `laureate-peter-handke-skill` | Peter Handke；效果取自「抒情」（§3） |
| 2020 | 路易丝·格吕克 | 效果 | `laureate-louise-gluck` | 4 费 | 未来 | `laureate-louise-gluck-skill` | Louise Glück；效果取自「讽刺」（§3） |
| 2021 | 阿卜杜勒拉扎克·古尔纳 | 效果 | `laureate-abdulrazak-gurnah` | 4 费 | 未来 | `laureate-abdulrazak-gurnah-skill` | Abdulrazak Gurnah；效果取自「写实」（§3） |
| 2022 | 安妮·埃尔诺 | 效果 | `laureate-annie-ernaux` | 4 费 | 未来 | `laureate-annie-ernaux-skill` | Annie Ernaux；效果取自「现代主义」（§3） |
| 2023 | 约恩·福瑟 | 效果 | `laureate-jon-fosse` | 4 费 | 未来 | `laureate-jon-fosse-skill` | Jon Fosse；效果取自「记忆」（§3） |
| 2024 | 韩江 | 效果 | `laureate-han-kang` | 4 费 | 未来 | `laureate-han-kang-skill` | Han Kang；效果取自「史诗」（§3） |
| 2025 | 拉斯洛·克拉斯诺霍尔卡伊 | 效果 | `laureate-laszlo-krasznahorkai` | 4 费 | 未来 | `laureate-laszlo-krasznahorkai-skill` | László Krasznahorkai；效果取自「抒情」（§3） |
| 2026 | 安妮·卡森 | 效果 | `laureate-anne-carson` | 4 费 | 未来 | `laureate-anne-carson-skill` | Anne Carson；效果取自「讽刺」（§3） |

#### 5.5 和平奖（140）

| 年份 | 得主 | 类型 | 卡牌 id | 星级/费用 | 时代 | 技能 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1901 | 弗雷德里克·帕西 | 效果 | `laureate-frederic-passy` | 2 费 | 工业 | `laureate-frederic-passy-skill` | Frédéric Passy；效果取自「调停」（§3） |
| 1901 | 亨利·杜南 | 效果 | `laureate-henry-dunant` | 2 费 | 工业 | `laureate-henry-dunant-skill` | Henry Dunant；效果取自「救济」（§3） |
| 1902 | 阿尔贝·戈巴 | 效果 | `laureate-albert-gobat` | 2 费 | 工业 | `laureate-albert-gobat-skill` | Albert Gobat；效果取自「裁军」（§3） |
| 1902 | 埃利·迪科蒙 | 效果 | `laureate-elie-ducommun` | 2 费 | 工业 | `laureate-elie-ducommun-skill` | Élie Ducommun；效果取自「权利」（§3） |
| 1903 | 兰德尔·克里默 | 效果 | `laureate-randal-cremer` | 2 费 | 工业 | `laureate-randal-cremer-skill` | Randal Cremer；效果取自「制度」（§3） |
| 1904 | 国际法学会 | 效果 | `laureate-institute-of-international-law` | 2 费 | 工业 | `laureate-institute-of-international-law-skill` | Institute of International Law；效果取自「团结」（§3） |
| 1905 | 贝尔塔·冯·苏特纳 | 效果 | `laureate-bertha-von-suttner` | 2 费 | 工业 | `laureate-bertha-von-suttner-skill` | Bertha von Suttner；效果取自「调停」（§3） |
| 1906 | 西奥多·罗斯福 | 效果 | `laureate-theodore-roosevelt` | 2 费 | 工业 | `laureate-theodore-roosevelt-skill` | Theodore Roosevelt；效果取自「救济」（§3） |
| 1907 | 埃内斯托·特奥多罗·莫内塔 | 效果 | `laureate-ernesto-teodoro-moneta` | 2 费 | 工业 | `laureate-ernesto-teodoro-moneta-skill` | Ernesto Teodoro Moneta；效果取自「裁军」（§3） |
| 1907 | 路易·雷诺 | 效果 | `laureate-louis-renault` | 2 费 | 工业 | `laureate-louis-renault-skill` | Louis Renault；效果取自「权利」（§3） |
| 1908 | 弗雷德里克·巴耶 | 效果 | `laureate-fredrik-bajer` | 2 费 | 工业 | `laureate-fredrik-bajer-skill` | Fredrik Bajer；效果取自「制度」（§3） |
| 1908 | 克拉斯·蓬图斯·阿诺德松 | 效果 | `laureate-klas-pontus-arnoldson` | 2 费 | 工业 | `laureate-klas-pontus-arnoldson-skill` | Klas Pontus Arnoldson；效果取自「团结」（§3） |
| 1909 | 奥古斯特·贝尔纳特 | 效果 | `laureate-auguste-beernaert` | 2 费 | 工业 | `laureate-auguste-beernaert-skill` | Auguste Beernaert；效果取自「调停」（§3） |
| 1909 | 保罗·亨利·德斯图内勒·德康斯坦 | 效果 | `laureate-paul-henri-d-estournelles-de-constant` | 2 费 | 工业 | `laureate-paul-henri-d-estournelles-de-constant-skill` | Paul Henri d'Estournelles de Constant；效果取自「救济」（§3） |
| 1910 | 国际和平常设局 | 效果 | `laureate-permanent-international-peace-bureau` | 2 费 | 工业 | `laureate-permanent-international-peace-bureau-skill` | Permanent International Peace Bureau；效果取自「裁军」（§3） |
| 1911 | 阿尔弗雷德·弗里德 | 效果 | `laureate-alfred-fried` | 2 费 | 工业 | `laureate-alfred-fried-skill` | Alfred Fried；效果取自「权利」（§3） |
| 1911 | 托比亚斯·阿塞尔 | 效果 | `laureate-tobias-asser` | 2 费 | 工业 | `laureate-tobias-asser-skill` | Tobias Asser；效果取自「制度」（§3） |
| 1912 | 伊莱休·鲁特 | 效果 | `laureate-elihu-root` | 2 费 | 工业 | `laureate-elihu-root-skill` | Elihu Root；效果取自「团结」（§3） |
| 1913 | 亨利·拉方丹 | 效果 | `laureate-henri-la-fontaine` | 2 费 | 工业 | `laureate-henri-la-fontaine-skill` | Henri La Fontaine；效果取自「调停」（§3） |
| 1917 | 红十字国际委员会 | 效果 | `laureate-international-committee-of-the-red-cross` | 2 费 | 工业 | `laureate-international-committee-of-the-red-cross-skill` | International Committee of the Red Cross；效果取自「救济」（§3）；1917 和平奖、1944 和平奖、1963 和平奖 |
| 1919 | 伍德罗·威尔逊 | 效果 | `laureate-woodrow-wilson` | 2 费 | 工业 | `laureate-woodrow-wilson-skill` | Woodrow Wilson；效果取自「裁军」（§3） |
| 1920 | 莱昂·布尔茹瓦 | 效果 | `laureate-leon-bourgeois` | 2 费 | 工业 | `laureate-leon-bourgeois-skill` | Léon Bourgeois；效果取自「权利」（§3） |
| 1921 | 克里斯蒂安·兰格 | 效果 | `laureate-christian-lange` | 2 费 | 工业 | `laureate-christian-lange-skill` | Christian Lange；效果取自「制度」（§3） |
| 1921 | 亚尔马·布兰廷 | 效果 | `laureate-hjalmar-branting` | 2 费 | 工业 | `laureate-hjalmar-branting-skill` | Hjalmar Branting；效果取自「团结」（§3） |
| 1922 | 弗里乔夫·南森 | 效果 | `laureate-fridtjof-nansen` | 2 费 | 工业 | `laureate-fridtjof-nansen-skill` | Fridtjof Nansen；效果取自「调停」（§3） |
| 1925 | 查尔斯·道威斯 | 效果 | `laureate-charles-g-dawes` | 2 费 | 工业 | `laureate-charles-g-dawes-skill` | Charles G. Dawes；效果取自「救济」（§3） |
| 1925 | 奥斯汀·张伯伦 | 效果 | `laureate-sir-austen-chamberlain` | 2 费 | 工业 | `laureate-sir-austen-chamberlain-skill` | Sir Austen Chamberlain；效果取自「裁军」（§3） |
| 1926 | 阿里斯蒂德·白里安 | 效果 | `laureate-aristide-briand` | 2 费 | 工业 | `laureate-aristide-briand-skill` | Aristide Briand；效果取自「权利」（§3） |
| 1926 | 古斯塔夫·施特雷泽曼 | 效果 | `laureate-gustav-stresemann` | 2 费 | 工业 | `laureate-gustav-stresemann-skill` | Gustav Stresemann；效果取自「制度」（§3） |
| 1927 | 费迪南·比松 | 效果 | `laureate-ferdinand-buisson` | 2 费 | 工业 | `laureate-ferdinand-buisson-skill` | Ferdinand Buisson；效果取自「团结」（§3） |
| 1927 | 路德维希·奎德 | 效果 | `laureate-ludwig-quidde` | 2 费 | 工业 | `laureate-ludwig-quidde-skill` | Ludwig Quidde；效果取自「调停」（§3） |
| 1929 | 弗兰克·凯洛格 | 效果 | `laureate-frank-b-kellogg` | 2 费 | 工业 | `laureate-frank-b-kellogg-skill` | Frank B. Kellogg；效果取自「救济」（§3） |
| 1930 | 纳坦·瑟德布卢姆 | 效果 | `laureate-nathan-soderblom` | 2 费 | 工业 | `laureate-nathan-soderblom-skill` | Nathan Söderblom；效果取自「裁军」（§3） |
| 1931 | 简·亚当斯 | 效果 | `laureate-jane-addams` | 2 费 | 工业 | `laureate-jane-addams-skill` | Jane Addams；效果取自「权利」（§3） |
| 1931 | 尼古拉斯·默里·巴特勒 | 效果 | `laureate-nicholas-murray-butler` | 2 费 | 工业 | `laureate-nicholas-murray-butler-skill` | Nicholas Murray Butler；效果取自「制度」（§3） |
| 1933 | 诺曼·安吉尔 | 效果 | `laureate-sir-norman-angell` | 2 费 | 工业 | `laureate-sir-norman-angell-skill` | Sir Norman Angell；效果取自「团结」（§3） |
| 1934 | 阿瑟·亨德森 | 效果 | `laureate-arthur-henderson` | 2 费 | 工业 | `laureate-arthur-henderson-skill` | Arthur Henderson；效果取自「调停」（§3） |
| 1935 | 卡尔·冯·奥西茨基 | 效果 | `laureate-carl-von-ossietzky` | 2 费 | 工业 | `laureate-carl-von-ossietzky-skill` | Carl von Ossietzky；效果取自「救济」（§3） |
| 1936 | 卡洛斯·萨韦德拉·拉马斯 | 效果 | `laureate-carlos-saavedra-lamas` | 2 费 | 工业 | `laureate-carlos-saavedra-lamas-skill` | Carlos Saavedra Lamas；效果取自「裁军」（§3） |
| 1937 | 罗伯特·塞西尔 | 效果 | `laureate-robert-cecil-viscount-cecil-of-chelwood` | 2 费 | 工业 | `laureate-robert-cecil-viscount-cecil-of-chelwood-skill` | 切尔伍德的塞西尔子爵）（Robert Cecil, Viscount Cecil of Chelwood；效果取自「权利」（§3） |
| 1938 | 南森国际难民办公室 | 效果 | `laureate-nansen-international-office-for-refugees` | 2 费 | 工业 | `laureate-nansen-international-office-for-refugees-skill` | Nansen International Office for Refugees；效果取自「制度」（§3） |
| 1945 | 科德尔·赫尔 | 效果 | `laureate-cordell-hull` | 3 费 | 工业 | `laureate-cordell-hull-skill` | Cordell Hull；效果取自「团结」（§3） |
| 1946 | 埃米莉·格林·巴尔奇 | 效果 | `laureate-emily-greene-balch` | 3 费 | 现代 | `laureate-emily-greene-balch-skill` | Emily Greene Balch；效果取自「调停」（§3） |
| 1946 | 约翰·穆德 | 效果 | `laureate-john-r-mott` | 3 费 | 现代 | `laureate-john-r-mott-skill` | John R. Mott；效果取自「救济」（§3） |
| 1947 | 美国公谊服务委员会 | 效果 | `laureate-american-friends-service-committee` | 3 费 | 现代 | `laureate-american-friends-service-committee-skill` | American Friends Service Committee；效果取自「裁军」（§3） |
| 1947 | 公谊服务委员会 | 效果 | `laureate-friends-service-council` | 3 费 | 现代 | `laureate-friends-service-council-skill` | Friends Service Council；效果取自「权利」（§3） |
| 1949 | 博伊德·奥尔勋爵 | 效果 | `laureate-lord-boyd-orr` | 3 费 | 现代 | `laureate-lord-boyd-orr-skill` | Lord Boyd Orr；效果取自「制度」（§3） |
| 1950 | 拉尔夫·本奇 | 效果 | `laureate-ralph-bunche` | 3 费 | 现代 | `laureate-ralph-bunche-skill` | Ralph Bunche；效果取自「团结」（§3） |
| 1951 | 莱昂·茹奥 | 效果 | `laureate-leon-jouhaux` | 3 费 | 现代 | `laureate-leon-jouhaux-skill` | Léon Jouhaux；效果取自「调停」（§3） |
| 1952 | 阿尔伯特·施韦泽 | 效果 | `laureate-albert-schweitzer` | 3 费 | 现代 | `laureate-albert-schweitzer-skill` | Albert Schweitzer；效果取自「救济」（§3） |
| 1953 | 乔治·马歇尔 | 效果 | `laureate-george-c-marshall` | 3 费 | 现代 | `laureate-george-c-marshall-skill` | George C. Marshall；效果取自「裁军」（§3） |
| 1954 | 联合国难民事务高级专员办事处 | 效果 | `laureate-office-of-the-united-nations-high-commissioner-for-refugees` | 3 费 | 现代 | `laureate-office-of-the-united-nations-high-commissioner-for-refugees-skill` | Office of the United Nations High Commissioner for Refugees；效果取自「权利」（§3）；1954 和平奖、1981 和平奖 |
| 1957 | 莱斯特·鲍尔斯·皮尔逊 | 效果 | `laureate-lester-bowles-pearson` | 3 费 | 现代 | `laureate-lester-bowles-pearson-skill` | Lester Bowles Pearson；效果取自「制度」（§3） |
| 1958 | 乔治·皮尔 | 效果 | `laureate-georges-pire` | 3 费 | 现代 | `laureate-georges-pire-skill` | Georges Pire；效果取自「团结」（§3） |
| 1959 | 菲利普·诺埃尔-贝克 | 效果 | `laureate-philip-noel-baker` | 3 费 | 现代 | `laureate-philip-noel-baker-skill` | Philip Noel-Baker；效果取自「调停」（§3） |
| 1960 | 阿尔伯特·卢图利 | 效果 | `laureate-albert-lutuli` | 3 费 | 现代 | `laureate-albert-lutuli-skill` | Albert Lutuli；效果取自「救济」（§3） |
| 1961 | 达格·哈马舍尔德 | 效果 | `laureate-dag-hammarskjold` | 3 费 | 现代 | `laureate-dag-hammarskjold-skill` | Dag Hammarskjöld；效果取自「裁军」（§3） |
| 1963 | 红十字会与红新月会协会 | 效果 | `laureate-league-of-red-cross-societies` | 3 费 | 现代 | `laureate-league-of-red-cross-societies-skill` | League of Red Cross Societies；效果取自「权利」（§3） |
| 1964 | 马丁·路德·金 | 效果 | `laureate-martin-luther-king-jr` | 3 费 | 现代 | `laureate-martin-luther-king-jr-skill` | Martin Luther King Jr.；效果取自「制度」（§3） |
| 1965 | 联合国儿童基金会 | 效果 | `laureate-united-nations-children-s-fund` | 3 费 | 现代 | `laureate-united-nations-children-s-fund-skill` | United Nations Children's Fund；效果取自「团结」（§3） |
| 1968 | 勒内·卡森 | 效果 | `laureate-rene-cassin` | 3 费 | 现代 | `laureate-rene-cassin-skill` | René Cassin；效果取自「调停」（§3） |
| 1969 | 国际劳工组织 | 效果 | `laureate-international-labour-organization` | 3 费 | 现代 | `laureate-international-labour-organization-skill` | International Labour Organization；效果取自「救济」（§3） |
| 1970 | 诺曼·博洛格 | 效果 | `laureate-norman-borlaug` | 3 费 | 现代 | `laureate-norman-borlaug-skill` | Norman Borlaug；效果取自「裁军」（§3） |
| 1971 | 维利·勃兰特 | 效果 | `laureate-willy-brandt` | 3 费 | 现代 | `laureate-willy-brandt-skill` | Willy Brandt；效果取自「权利」（§3） |
| 1973 | 亨利·基辛格 | 效果 | `laureate-henry-kissinger` | 3 费 | 现代 | `laureate-henry-kissinger-skill` | Henry Kissinger；效果取自「制度」（§3） |
| 1973 | 黎德寿 | 效果 | `laureate-le-duc-tho` | 3 费 | 现代 | `laureate-le-duc-tho-skill` | Le Duc Tho；效果取自「团结」（§3） |
| 1974 | 佐藤荣作 | 效果 | `laureate-eisaku-sato` | 3 费 | 现代 | `laureate-eisaku-sato-skill` | Eisaku Satō；效果取自「调停」（§3） |
| 1974 | 肖恩·麦克布赖德 | 效果 | `laureate-sean-macbride` | 3 费 | 现代 | `laureate-sean-macbride-skill` | Seán MacBride；效果取自「救济」（§3） |
| 1975 | 安德烈·萨哈罗夫 | 效果 | `laureate-andrei-sakharov` | 3 费 | 现代 | `laureate-andrei-sakharov-skill` | Andrei Sakharov；效果取自「裁军」（§3） |
| 1976 | 贝蒂·威廉斯 | 效果 | `laureate-betty-williams` | 3 费 | 现代 | `laureate-betty-williams-skill` | Betty Williams；效果取自「权利」（§3） |
| 1976 | 梅里德·科里根 | 效果 | `laureate-mairead-corrigan` | 3 费 | 现代 | `laureate-mairead-corrigan-skill` | Mairead Corrigan；效果取自「制度」（§3） |
| 1977 | 国际特赦组织 | 效果 | `laureate-amnesty-international` | 3 费 | 现代 | `laureate-amnesty-international-skill` | Amnesty International；效果取自「团结」（§3） |
| 1978 | 安瓦尔·萨达特 | 效果 | `laureate-anwar-al-sadat` | 3 费 | 现代 | `laureate-anwar-al-sadat-skill` | Anwar al-Sadat；效果取自「调停」（§3） |
| 1978 | 梅纳赫姆·贝京 | 效果 | `laureate-menachem-begin` | 3 费 | 现代 | `laureate-menachem-begin-skill` | Menachem Begin；效果取自「救济」（§3） |
| 1979 | 特蕾莎修女 | 效果 | `laureate-mother-teresa` | 3 费 | 现代 | `laureate-mother-teresa-skill` | Mother Teresa；效果取自「裁军」（§3） |
| 1980 | 阿道弗·佩雷斯·埃斯基维尔 | 效果 | `laureate-adolfo-perez-esquivel` | 3 费 | 现代 | `laureate-adolfo-perez-esquivel-skill` | Adolfo Pérez Esquivel；效果取自「权利」（§3） |
| 1982 | 阿方索·加西亚·罗夫莱斯 | 效果 | `laureate-alfonso-garcia-robles` | 3 费 | 现代 | `laureate-alfonso-garcia-robles-skill` | Alfonso García Robles；效果取自「制度」（§3） |
| 1982 | 阿尔瓦·米达尔 | 效果 | `laureate-alva-myrdal` | 3 费 | 现代 | `laureate-alva-myrdal-skill` | Alva Myrdal；效果取自「团结」（§3） |
| 1983 | 莱赫·瓦文萨 | 效果 | `laureate-lech-walesa` | 3 费 | 现代 | `laureate-lech-walesa-skill` | Lech Wałęsa；效果取自「调停」（§3） |
| 1984 | 德斯蒙德·图图 | 效果 | `laureate-desmond-tutu` | 3 费 | 现代 | `laureate-desmond-tutu-skill` | Desmond Tutu；效果取自「救济」（§3） |
| 1985 | 国际防止核战争医生组织 | 效果 | `laureate-international-physicians-for-the-prevention-of-nuclear-war` | 3 费 | 现代 | `laureate-international-physicians-for-the-prevention-of-nuclear-war-skill` | International Physicians for the Prevention of Nuclear War；效果取自「裁军」（§3） |
| 1986 | 埃利·维瑟尔 | 效果 | `laureate-elie-wiesel` | 3 费 | 现代 | `laureate-elie-wiesel-skill` | Elie Wiesel；效果取自「权利」（§3） |
| 1987 | 奥斯卡·阿里亚斯·桑切斯 | 效果 | `laureate-oscar-arias-sanchez` | 3 费 | 现代 | `laureate-oscar-arias-sanchez-skill` | Oscar Arias Sánchez；效果取自「制度」（§3） |
| 1988 | 联合国维持和平部队 | 效果 | `laureate-united-nations-peacekeeping-forces` | 3 费 | 现代 | `laureate-united-nations-peacekeeping-forces-skill` | United Nations Peacekeeping Forces；效果取自「团结」（§3） |
| 1989 | 第十四世达赖喇嘛 | 效果 | `laureate-the-14th-dalai-lama` | 3 费 | 现代 | `laureate-the-14th-dalai-lama-skill` | The 14th Dalai Lama；效果取自「调停」（§3） |
| 1990 | 米哈伊尔·戈尔巴乔夫 | 效果 | `laureate-mikhail-gorbachev` | 3 费 | 现代 | `laureate-mikhail-gorbachev-skill` | Mikhail Gorbachev；效果取自「救济」（§3） |
| 1991 | 昂山素季 | 效果 | `laureate-aung-san-suu-kyi` | 3 费 | 现代 | `laureate-aung-san-suu-kyi-skill` | Aung San Suu Kyi；效果取自「裁军」（§3） |
| 1992 | 里戈韦塔·门楚·图姆 | 效果 | `laureate-rigoberta-menchu-tum` | 3 费 | 现代 | `laureate-rigoberta-menchu-tum-skill` | Rigoberta Menchú Tum；效果取自「权利」（§3） |
| 1993 | 弗·威·德克勒克 | 效果 | `laureate-f-w-de-klerk` | 3 费 | 现代 | `laureate-f-w-de-klerk-skill` | F.W. de Klerk；效果取自「制度」（§3） |
| 1993 | 纳尔逊·曼德拉 | 效果 | `laureate-nelson-mandela` | 3 费 | 现代 | `laureate-nelson-mandela-skill` | Nelson Mandela；效果取自「团结」（§3） |
| 1994 | 希蒙·佩雷斯 | 效果 | `laureate-shimon-peres` | 3 费 | 现代 | `laureate-shimon-peres-skill` | Shimon Peres；效果取自「调停」（§3） |
| 1994 | 亚西尔·阿拉法特 | 效果 | `laureate-yasser-arafat` | 3 费 | 现代 | `laureate-yasser-arafat-skill` | Yasser Arafat；效果取自「救济」（§3） |
| 1994 | 伊扎克·拉宾 | 效果 | `laureate-yitzhak-rabin` | 3 费 | 现代 | `laureate-yitzhak-rabin-skill` | Yitzhak Rabin；效果取自「裁军」（§3） |
| 1995 | 约瑟夫·罗特布拉特 | 效果 | `laureate-joseph-rotblat` | 3 费 | 现代 | `laureate-joseph-rotblat-skill` | Joseph Rotblat；效果取自「权利」（§3） |
| 1995 | 帕格沃什科学与世界事务会议 | 效果 | `laureate-pugwash-conferences-on-science-and-world-affairs` | 3 费 | 现代 | `laureate-pugwash-conferences-on-science-and-world-affairs-skill` | Pugwash Conferences on Science and World Affairs；效果取自「制度」（§3） |
| 1996 | 卡洛斯·菲利佩·希梅内斯·贝洛 | 效果 | `laureate-carlos-filipe-ximenes-belo` | 3 费 | 现代 | `laureate-carlos-filipe-ximenes-belo-skill` | Carlos Filipe Ximenes Belo；效果取自「团结」（§3） |
| 1996 | 若泽·拉莫斯-奥尔塔 | 效果 | `laureate-jose-ramos-horta` | 3 费 | 现代 | `laureate-jose-ramos-horta-skill` | José Ramos-Horta；效果取自「调停」（§3） |
| 1997 | 国际禁止地雷运动 | 效果 | `laureate-international-campaign-to-ban-landmines` | 3 费 | 现代 | `laureate-international-campaign-to-ban-landmines-skill` | International Campaign to Ban Landmines；效果取自「救济」（§3） |
| 1997 | 乔迪·威廉斯 | 效果 | `laureate-jody-williams` | 3 费 | 现代 | `laureate-jody-williams-skill` | Jody Williams；效果取自「裁军」（§3） |
| 1998 | 戴维·特林布尔 | 效果 | `laureate-david-trimble` | 3 费 | 现代 | `laureate-david-trimble-skill` | David Trimble；效果取自「权利」（§3） |
| 1998 | 约翰·休姆 | 效果 | `laureate-john-hume` | 3 费 | 现代 | `laureate-john-hume-skill` | John Hume；效果取自「制度」（§3） |
| 1999 | 无国界医生 | 效果 | `laureate-doctors-without-borders` | 3 费 | 现代 | `laureate-doctors-without-borders-skill` | Doctors Without Borders；效果取自「团结」（§3） |
| 2000 | 金大中 | 效果 | `laureate-kim-dae-jung` | 4 费 | 未来 | `laureate-kim-dae-jung-skill` | Kim Dae-jung；效果取自「调停」（§3） |
| 2001 | 科菲·安南 | 效果 | `laureate-kofi-annan` | 4 费 | 未来 | `laureate-kofi-annan-skill` | Kofi Annan；效果取自「救济」（§3） |
| 2001 | 联合国 | 效果 | `laureate-united-nations` | 4 费 | 未来 | `laureate-united-nations-skill` | United Nations；效果取自「裁军」（§3） |
| 2002 | 吉米·卡特 | 效果 | `laureate-jimmy-carter` | 4 费 | 未来 | `laureate-jimmy-carter-skill` | Jimmy Carter；效果取自「权利」（§3） |
| 2003 | 希林·伊巴迪 | 效果 | `laureate-shirin-ebadi` | 4 费 | 未来 | `laureate-shirin-ebadi-skill` | Shirin Ebadi；效果取自「制度」（§3） |
| 2004 | 旺加里·马塔伊 | 效果 | `laureate-wangari-maathai` | 4 费 | 未来 | `laureate-wangari-maathai-skill` | Wangari Maathai；效果取自「团结」（§3） |
| 2005 | 国际原子能机构 | 效果 | `laureate-international-atomic-energy-agency` | 4 费 | 未来 | `laureate-international-atomic-energy-agency-skill` | International Atomic Energy Agency；效果取自「调停」（§3） |
| 2005 | 穆罕默德·巴拉迪 | 效果 | `laureate-mohamed-elbaradei` | 4 费 | 未来 | `laureate-mohamed-elbaradei-skill` | Mohamed ElBaradei；效果取自「救济」（§3） |
| 2006 | 格莱珉银行 | 效果 | `laureate-grameen-bank` | 4 费 | 未来 | `laureate-grameen-bank-skill` | Grameen Bank；效果取自「裁军」（§3） |
| 2006 | 穆罕默德·尤努斯 | 效果 | `laureate-muhammad-yunus` | 4 费 | 未来 | `laureate-muhammad-yunus-skill` | Muhammad Yunus；效果取自「权利」（§3） |
| 2007 | 阿尔·戈尔 | 效果 | `laureate-al-gore` | 4 费 | 未来 | `laureate-al-gore-skill` | Al Gore；效果取自「制度」（§3） |
| 2007 | 政府间气候变化专门委员会 | 效果 | `laureate-intergovernmental-panel-on-climate-change` | 4 费 | 未来 | `laureate-intergovernmental-panel-on-climate-change-skill` | Intergovernmental Panel on Climate Change；效果取自「团结」（§3） |
| 2008 | 马尔蒂·阿赫蒂萨里 | 效果 | `laureate-martti-ahtisaari` | 4 费 | 未来 | `laureate-martti-ahtisaari-skill` | Martti Ahtisaari；效果取自「调停」（§3） |
| 2009 | 贝拉克·奥巴马 | 效果 | `laureate-barack-h-obama` | 4 费 | 未来 | `laureate-barack-h-obama-skill` | Barack H. Obama；效果取自「救济」（§3） |
| 2010 | 刘晓波 | 效果 | `laureate-liu-xiaobo` | 4 费 | 未来 | `laureate-liu-xiaobo-skill` | Liu Xiaobo；效果取自「裁军」（§3） |
| 2011 | 埃伦·约翰逊·瑟利夫 | 效果 | `laureate-ellen-johnson-sirleaf` | 4 费 | 未来 | `laureate-ellen-johnson-sirleaf-skill` | Ellen Johnson Sirleaf；效果取自「权利」（§3） |
| 2011 | 莱伊曼·古博韦 | 效果 | `laureate-leymah-gbowee` | 4 费 | 未来 | `laureate-leymah-gbowee-skill` | Leymah Gbowee；效果取自「制度」（§3） |
| 2011 | 塔瓦库勒·卡曼 | 效果 | `laureate-tawakkol-karman` | 4 费 | 未来 | `laureate-tawakkol-karman-skill` | Tawakkol Karman；效果取自「团结」（§3） |
| 2012 | 欧洲联盟 | 效果 | `laureate-european-union` | 4 费 | 未来 | `laureate-european-union-skill` | European Union；效果取自「调停」（§3） |
| 2013 | 禁止化学武器组织 | 效果 | `laureate-organisation-for-the-prohibition-of-chemical-weapons` | 4 费 | 未来 | `laureate-organisation-for-the-prohibition-of-chemical-weapons-skill` | Organisation for the Prohibition of Chemical Weapons；效果取自「救济」（§3） |
| 2014 | 凯拉什·萨蒂亚尔蒂 | 效果 | `laureate-kailash-satyarthi` | 4 费 | 未来 | `laureate-kailash-satyarthi-skill` | Kailash Satyarthi；效果取自「裁军」（§3） |
| 2014 | 马拉拉·优素福扎伊 | 效果 | `laureate-malala-yousafzai` | 4 费 | 未来 | `laureate-malala-yousafzai-skill` | Malala Yousafzai；效果取自「权利」（§3） |
| 2015 | 突尼斯全国对话四方 | 效果 | `laureate-national-dialogue-quartet` | 4 费 | 未来 | `laureate-national-dialogue-quartet-skill` | National Dialogue Quartet；效果取自「制度」（§3） |
| 2016 | 胡安·曼努埃尔·桑托斯 | 效果 | `laureate-juan-manuel-santos` | 4 费 | 未来 | `laureate-juan-manuel-santos-skill` | Juan Manuel Santos；效果取自「团结」（§3） |
| 2017 | 国际废除核武器运动 | 效果 | `laureate-international-campaign-to-abolish-nuclear-weapons` | 4 费 | 未来 | `laureate-international-campaign-to-abolish-nuclear-weapons-skill` | International Campaign to Abolish Nuclear Weapons；效果取自「调停」（§3） |
| 2018 | 德尼·穆克维格 | 效果 | `laureate-denis-mukwege` | 4 费 | 未来 | `laureate-denis-mukwege-skill` | Denis Mukwege；效果取自「救济」（§3） |
| 2018 | 纳迪娅·穆拉德 | 效果 | `laureate-nadia-murad` | 4 费 | 未来 | `laureate-nadia-murad-skill` | Nadia Murad；效果取自「裁军」（§3） |
| 2019 | 阿比·艾哈迈德·阿里 | 效果 | `laureate-abiy-ahmed-ali` | 4 费 | 未来 | `laureate-abiy-ahmed-ali-skill` | Abiy Ahmed Ali；效果取自「权利」（§3） |
| 2020 | 世界粮食计划署 | 效果 | `laureate-world-food-programme` | 4 费 | 未来 | `laureate-world-food-programme-skill` | World Food Programme；效果取自「制度」（§3） |
| 2021 | 德米特里·穆拉托夫 | 效果 | `laureate-dmitry-muratov` | 4 费 | 未来 | `laureate-dmitry-muratov-skill` | Dmitry Muratov；效果取自「团结」（§3） |
| 2021 | 玛丽亚·雷萨 | 效果 | `laureate-maria-ressa` | 4 费 | 未来 | `laureate-maria-ressa-skill` | Maria Ressa；效果取自「调停」（§3） |
| 2022 | 阿列斯·别利亚茨基 | 效果 | `laureate-ales-bialiatski` | 4 费 | 未来 | `laureate-ales-bialiatski-skill` | Ales Bialiatski；效果取自「救济」（§3） |
| 2022 | 公民自由中心 | 效果 | `laureate-center-for-civil-liberties` | 4 费 | 未来 | `laureate-center-for-civil-liberties-skill` | Center for Civil Liberties；效果取自「裁军」（§3） |
| 2022 | 纪念组织 | 效果 | `laureate-memorial` | 4 费 | 未来 | `laureate-memorial-skill` | Memorial；效果取自「权利」（§3） |
| 2023 | 纳尔吉斯·穆罕默迪 | 效果 | `laureate-narges-mohammadi` | 4 费 | 未来 | `laureate-narges-mohammadi-skill` | Narges Mohammadi；效果取自「制度」（§3） |
| 2024 | 日本原子弹氢弹受害者团体协议会 | 效果 | `laureate-nihon-hidankyo` | 4 费 | 未来 | `laureate-nihon-hidankyo-skill` | Nihon Hidankyo；效果取自「团结」（§3） |
| 2025 | 玛丽亚·科里纳·马查多 | 效果 | `laureate-maria-corina-machado` | 4 费 | 未来 | `laureate-maria-corina-machado-skill` | Maria Corina Machado；效果取自「调停」（§3） |
| 2026 | 纳维·皮莱 | 效果 | `laureate-navi-pillay` | 4 费 | 未来 | `laureate-navi-pillay-skill` | Navi Pillay；效果取自「救济」（§3） |

#### 5.6 经济学奖（101）

| 年份 | 得主 | 类型 | 卡牌 id | 星级/费用 | 时代 | 技能 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1969 | 扬·丁伯根 | 效果 | `laureate-jan-tinbergen` | 3 费 | 现代 | `laureate-jan-tinbergen-skill` | Jan Tinbergen；效果取自「激励」（§3） |
| 1969 | 拉格纳·弗里希 | 效果 | `laureate-ragnar-frisch` | 3 费 | 现代 | `laureate-ragnar-frisch-skill` | Ragnar Frisch；效果取自「均衡」（§3） |
| 1970 | 保罗·萨缪尔森 | 效果 | `laureate-paul-a-samuelson` | 3 费 | 现代 | `laureate-paul-a-samuelson-skill` | Paul A. Samuelson；效果取自「信息」（§3） |
| 1971 | 西蒙·库兹涅茨 | 效果 | `laureate-simon-kuznets` | 3 费 | 现代 | `laureate-simon-kuznets-skill` | Simon Kuznets；效果取自「制度」（§3） |
| 1972 | 约翰·希克斯 | 效果 | `laureate-john-r-hicks` | 3 费 | 现代 | `laureate-john-r-hicks-skill` | John R. Hicks；效果取自「增长」（§3） |
| 1972 | 肯尼斯·阿罗 | 效果 | `laureate-kenneth-j-arrow` | 3 费 | 现代 | `laureate-kenneth-j-arrow-skill` | Kenneth J. Arrow；效果取自「行为」（§3） |
| 1973 | 瓦西里·列昂惕夫 | 效果 | `laureate-wassily-leontief` | 3 费 | 现代 | `laureate-wassily-leontief-skill` | Wassily Leontief；效果取自「激励」（§3） |
| 1974 | 弗里德里希·冯·哈耶克 | 效果 | `laureate-friedrich-von-hayek` | 3 费 | 现代 | `laureate-friedrich-von-hayek-skill` | Friedrich von Hayek；效果取自「均衡」（§3） |
| 1974 | 贡纳尔·缪尔达尔 | 效果 | `laureate-gunnar-myrdal` | 3 费 | 现代 | `laureate-gunnar-myrdal-skill` | Gunnar Myrdal；效果取自「信息」（§3） |
| 1975 | 列昂尼德·维塔利耶维奇·康托罗维奇 | 效果 | `laureate-leonid-vitaliyevich-kantorovich` | 3 费 | 现代 | `laureate-leonid-vitaliyevich-kantorovich-skill` | Leonid Vitaliyevich Kantorovich；效果取自「制度」（§3） |
| 1975 | 恰林·库普曼斯 | 效果 | `laureate-tjalling-c-koopmans` | 3 费 | 现代 | `laureate-tjalling-c-koopmans-skill` | Tjalling C. Koopmans；效果取自「增长」（§3） |
| 1976 | 米尔顿·弗里德曼 | 效果 | `laureate-milton-friedman` | 3 费 | 现代 | `laureate-milton-friedman-skill` | Milton Friedman；效果取自「行为」（§3） |
| 1977 | 贝蒂尔·俄林 | 效果 | `laureate-bertil-ohlin` | 3 费 | 现代 | `laureate-bertil-ohlin-skill` | Bertil Ohlin；效果取自「激励」（§3） |
| 1977 | 詹姆斯·米德 | 效果 | `laureate-james-e-meade` | 3 费 | 现代 | `laureate-james-e-meade-skill` | James E. Meade；效果取自「均衡」（§3） |
| 1978 | 赫伯特·西蒙 | 效果 | `laureate-herbert-simon` | 3 费 | 现代 | `laureate-herbert-simon-skill` | Herbert Simon；效果取自「信息」（§3） |
| 1979 | 阿瑟·刘易斯 | 效果 | `laureate-sir-arthur-lewis` | 3 费 | 现代 | `laureate-sir-arthur-lewis-skill` | Sir Arthur Lewis；效果取自「制度」（§3） |
| 1979 | 西奥多·舒尔茨 | 效果 | `laureate-theodore-w-schultz` | 3 费 | 现代 | `laureate-theodore-w-schultz-skill` | Theodore W. Schultz；效果取自「增长」（§3） |
| 1980 | 劳伦斯·克莱因 | 效果 | `laureate-lawrence-r-klein` | 3 费 | 现代 | `laureate-lawrence-r-klein-skill` | Lawrence R. Klein；效果取自「行为」（§3） |
| 1981 | 詹姆斯·托宾 | 效果 | `laureate-james-tobin` | 3 费 | 现代 | `laureate-james-tobin-skill` | James Tobin；效果取自「激励」（§3） |
| 1982 | 乔治·施蒂格勒 | 效果 | `laureate-george-j-stigler` | 3 费 | 现代 | `laureate-george-j-stigler-skill` | George J. Stigler；效果取自「均衡」（§3） |
| 1983 | 热拉尔·德布鲁 | 效果 | `laureate-gerard-debreu` | 3 费 | 现代 | `laureate-gerard-debreu-skill` | Gerard Debreu；效果取自「信息」（§3） |
| 1984 | 理查德·斯通 | 效果 | `laureate-richard-stone` | 3 费 | 现代 | `laureate-richard-stone-skill` | Richard Stone；效果取自「制度」（§3） |
| 1985 | 佛朗哥·莫迪利亚尼 | 效果 | `laureate-franco-modigliani` | 3 费 | 现代 | `laureate-franco-modigliani-skill` | Franco Modigliani；效果取自「增长」（§3） |
| 1986 | 小詹姆斯·布坎南 | 效果 | `laureate-james-m-buchanan-jr` | 3 费 | 现代 | `laureate-james-m-buchanan-jr-skill` | James M. Buchanan Jr.；效果取自「行为」（§3） |
| 1987 | 罗伯特·索洛 | 效果 | `laureate-robert-m-solow` | 3 费 | 现代 | `laureate-robert-m-solow-skill` | Robert M. Solow；效果取自「激励」（§3） |
| 1988 | 莫里斯·阿莱 | 效果 | `laureate-maurice-allais` | 3 费 | 现代 | `laureate-maurice-allais-skill` | Maurice Allais；效果取自「均衡」（§3） |
| 1989 | 特里夫·哈维尔莫 | 效果 | `laureate-trygve-haavelmo` | 3 费 | 现代 | `laureate-trygve-haavelmo-skill` | Trygve Haavelmo；效果取自「信息」（§3） |
| 1990 | 哈里·马科维茨 | 效果 | `laureate-harry-m-markowitz` | 3 费 | 现代 | `laureate-harry-m-markowitz-skill` | Harry M. Markowitz；效果取自「制度」（§3） |
| 1990 | 默顿·米勒 | 效果 | `laureate-merton-h-miller` | 3 费 | 现代 | `laureate-merton-h-miller-skill` | Merton H. Miller；效果取自「增长」（§3） |
| 1990 | 威廉·夏普 | 效果 | `laureate-william-f-sharpe` | 3 费 | 现代 | `laureate-william-f-sharpe-skill` | William F. Sharpe；效果取自「行为」（§3） |
| 1991 | 罗纳德·科斯 | 效果 | `laureate-ronald-h-coase` | 3 费 | 现代 | `laureate-ronald-h-coase-skill` | Ronald H. Coase；效果取自「激励」（§3） |
| 1992 | 加里·贝克尔 | 效果 | `laureate-gary-becker` | 3 费 | 现代 | `laureate-gary-becker-skill` | Gary Becker；效果取自「均衡」（§3） |
| 1993 | 道格拉斯·诺斯 | 效果 | `laureate-douglass-c-north` | 3 费 | 现代 | `laureate-douglass-c-north-skill` | Douglass C. North；效果取自「信息」（§3） |
| 1993 | 罗伯特·福格尔 | 效果 | `laureate-robert-w-fogel` | 3 费 | 现代 | `laureate-robert-w-fogel-skill` | Robert W. Fogel；效果取自「制度」（§3） |
| 1994 | 约翰·海萨尼 | 效果 | `laureate-john-c-harsanyi` | 3 费 | 现代 | `laureate-john-c-harsanyi-skill` | John C. Harsanyi；效果取自「增长」（§3） |
| 1994 | 小约翰·纳什 | 效果 | `laureate-john-f-nash-jr` | 3 费 | 现代 | `laureate-john-f-nash-jr-skill` | John F. Nash Jr.；效果取自「行为」（§3） |
| 1994 | 赖因哈德·泽尔腾 | 效果 | `laureate-reinhard-selten` | 3 费 | 现代 | `laureate-reinhard-selten-skill` | Reinhard Selten；效果取自「激励」（§3） |
| 1995 | 小罗伯特·卢卡斯 | 效果 | `laureate-robert-e-lucas-jr` | 3 费 | 现代 | `laureate-robert-e-lucas-jr-skill` | Robert E. Lucas Jr.；效果取自「均衡」（§3） |
| 1996 | 詹姆斯·米尔利斯 | 效果 | `laureate-james-a-mirrlees` | 3 费 | 现代 | `laureate-james-a-mirrlees-skill` | James A. Mirrlees；效果取自「信息」（§3） |
| 1996 | 威廉·维克里 | 效果 | `laureate-william-vickrey` | 3 费 | 现代 | `laureate-william-vickrey-skill` | William Vickrey；效果取自「制度」（§3） |
| 1997 | 迈伦·斯科尔斯 | 效果 | `laureate-myron-scholes` | 3 费 | 现代 | `laureate-myron-scholes-skill` | Myron Scholes；效果取自「增长」（§3） |
| 1997 | 罗伯特·默顿 | 效果 | `laureate-robert-c-merton` | 3 费 | 现代 | `laureate-robert-c-merton-skill` | Robert C. Merton；效果取自「行为」（§3） |
| 1998 | 阿马蒂亚·森 | 效果 | `laureate-amartya-sen` | 3 费 | 现代 | `laureate-amartya-sen-skill` | Amartya Sen；效果取自「激励」（§3） |
| 1999 | 罗伯特·蒙代尔 | 效果 | `laureate-robert-mundell` | 3 费 | 现代 | `laureate-robert-mundell-skill` | Robert Mundell；效果取自「均衡」（§3） |
| 2000 | 丹尼尔·麦克法登 | 效果 | `laureate-daniel-l-mcfadden` | 4 费 | 未来 | `laureate-daniel-l-mcfadden-skill` | Daniel L. McFadden；效果取自「信息」（§3） |
| 2000 | 詹姆斯·赫克曼 | 效果 | `laureate-james-j-heckman` | 4 费 | 未来 | `laureate-james-j-heckman-skill` | James J. Heckman；效果取自「制度」（§3） |
| 2001 | 迈克尔·斯彭斯 | 效果 | `laureate-a-michael-spence` | 4 费 | 未来 | `laureate-a-michael-spence-skill` | A. Michael Spence；效果取自「增长」（§3） |
| 2001 | 乔治·阿克洛夫 | 效果 | `laureate-george-a-akerlof` | 4 费 | 未来 | `laureate-george-a-akerlof-skill` | George A. Akerlof；效果取自「行为」（§3） |
| 2001 | 约瑟夫·斯蒂格利茨 | 效果 | `laureate-joseph-e-stiglitz` | 4 费 | 未来 | `laureate-joseph-e-stiglitz-skill` | Joseph E. Stiglitz；效果取自「激励」（§3） |
| 2002 | 丹尼尔·卡尼曼 | 效果 | `laureate-daniel-kahneman` | 4 费 | 未来 | `laureate-daniel-kahneman-skill` | Daniel Kahneman；效果取自「均衡」（§3） |
| 2002 | 弗农·史密斯 | 效果 | `laureate-vernon-l-smith` | 4 费 | 未来 | `laureate-vernon-l-smith-skill` | Vernon L. Smith；效果取自「信息」（§3） |
| 2003 | 克莱夫·格兰杰 | 效果 | `laureate-clive-w-j-granger` | 4 费 | 未来 | `laureate-clive-w-j-granger-skill` | Clive W.J. Granger；效果取自「制度」（§3） |
| 2003 | 罗伯特·恩格尔三世 | 效果 | `laureate-robert-f-engle-iii` | 4 费 | 未来 | `laureate-robert-f-engle-iii-skill` | Robert F. Engle III；效果取自「增长」（§3） |
| 2004 | 爱德华·普雷斯科特 | 效果 | `laureate-edward-c-prescott` | 4 费 | 未来 | `laureate-edward-c-prescott-skill` | Edward C. Prescott；效果取自「行为」（§3） |
| 2004 | 芬恩·基德兰德 | 效果 | `laureate-finn-e-kydland` | 4 费 | 未来 | `laureate-finn-e-kydland-skill` | Finn E. Kydland；效果取自「激励」（§3） |
| 2005 | 罗伯特·奥曼 | 效果 | `laureate-robert-j-aumann` | 4 费 | 未来 | `laureate-robert-j-aumann-skill` | Robert J. Aumann；效果取自「均衡」（§3） |
| 2005 | 托马斯·谢林 | 效果 | `laureate-thomas-c-schelling` | 4 费 | 未来 | `laureate-thomas-c-schelling-skill` | Thomas C. Schelling；效果取自「信息」（§3） |
| 2006 | 埃德蒙·费尔普斯 | 效果 | `laureate-edmund-s-phelps` | 4 费 | 未来 | `laureate-edmund-s-phelps-skill` | Edmund S. Phelps；效果取自「制度」（§3） |
| 2007 | 埃里克·马斯金 | 效果 | `laureate-eric-s-maskin` | 4 费 | 未来 | `laureate-eric-s-maskin-skill` | Eric S. Maskin；效果取自「增长」（§3） |
| 2007 | 列昂尼德·赫维茨 | 效果 | `laureate-leonid-hurwicz` | 4 费 | 未来 | `laureate-leonid-hurwicz-skill` | Leonid Hurwicz；效果取自「行为」（§3） |
| 2007 | 罗杰·迈尔森 | 效果 | `laureate-roger-b-myerson` | 4 费 | 未来 | `laureate-roger-b-myerson-skill` | Roger B. Myerson；效果取自「激励」（§3） |
| 2008 | 保罗·克鲁格曼 | 效果 | `laureate-paul-krugman` | 4 费 | 未来 | `laureate-paul-krugman-skill` | Paul Krugman；效果取自「均衡」（§3） |
| 2009 | 埃莉诺·奥斯特罗姆 | 效果 | `laureate-elinor-ostrom` | 4 费 | 未来 | `laureate-elinor-ostrom-skill` | Elinor Ostrom；效果取自「信息」（§3） |
| 2009 | 奥利弗·威廉姆森 | 效果 | `laureate-oliver-e-williamson` | 4 费 | 未来 | `laureate-oliver-e-williamson-skill` | Oliver E. Williamson；效果取自「制度」（§3） |
| 2010 | 克里斯托弗·皮萨里德斯 | 效果 | `laureate-christopher-a-pissarides` | 4 费 | 未来 | `laureate-christopher-a-pissarides-skill` | Christopher A. Pissarides；效果取自「增长」（§3） |
| 2010 | 戴尔·莫滕森 | 效果 | `laureate-dale-t-mortensen` | 4 费 | 未来 | `laureate-dale-t-mortensen-skill` | Dale T. Mortensen；效果取自「行为」（§3） |
| 2010 | 彼得·戴蒙德 | 效果 | `laureate-peter-a-diamond` | 4 费 | 未来 | `laureate-peter-a-diamond-skill` | Peter A. Diamond；效果取自「激励」（§3） |
| 2011 | 克里斯托弗·西姆斯 | 效果 | `laureate-christopher-a-sims` | 4 费 | 未来 | `laureate-christopher-a-sims-skill` | Christopher A. Sims；效果取自「均衡」（§3） |
| 2011 | 托马斯·萨金特 | 效果 | `laureate-thomas-j-sargent` | 4 费 | 未来 | `laureate-thomas-j-sargent-skill` | Thomas J. Sargent；效果取自「信息」（§3） |
| 2012 | 阿尔文·罗思 | 效果 | `laureate-alvin-e-roth` | 4 费 | 未来 | `laureate-alvin-e-roth-skill` | Alvin E. Roth；效果取自「制度」（§3） |
| 2012 | 劳埃德·沙普利 | 效果 | `laureate-lloyd-s-shapley` | 4 费 | 未来 | `laureate-lloyd-s-shapley-skill` | Lloyd S. Shapley；效果取自「增长」（§3） |
| 2013 | 尤金·法马 | 效果 | `laureate-eugene-f-fama` | 4 费 | 未来 | `laureate-eugene-f-fama-skill` | Eugene F. Fama；效果取自「行为」（§3） |
| 2013 | 拉尔斯·彼得·汉森 | 效果 | `laureate-lars-peter-hansen` | 4 费 | 未来 | `laureate-lars-peter-hansen-skill` | Lars Peter Hansen；效果取自「激励」（§3） |
| 2013 | 罗伯特·席勒 | 效果 | `laureate-robert-j-shiller` | 4 费 | 未来 | `laureate-robert-j-shiller-skill` | Robert J. Shiller；效果取自「均衡」（§3） |
| 2014 | 让·梯若尔 | 效果 | `laureate-jean-tirole` | 4 费 | 未来 | `laureate-jean-tirole-skill` | Jean Tirole；效果取自「信息」（§3） |
| 2015 | 安格斯·迪顿 | 效果 | `laureate-angus-deaton` | 4 费 | 未来 | `laureate-angus-deaton-skill` | Angus Deaton；效果取自「制度」（§3） |
| 2016 | 本特·霍姆斯特伦 | 效果 | `laureate-bengt-holmstrom` | 4 费 | 未来 | `laureate-bengt-holmstrom-skill` | Bengt Holmström；效果取自「增长」（§3） |
| 2016 | 奥利弗·哈特 | 效果 | `laureate-oliver-hart` | 4 费 | 未来 | `laureate-oliver-hart-skill` | Oliver Hart；效果取自「行为」（§3） |
| 2017 | 理查德·塞勒 | 效果 | `laureate-richard-h-thaler` | 4 费 | 未来 | `laureate-richard-h-thaler-skill` | Richard H. Thaler；效果取自「激励」（§3） |
| 2018 | 保罗·罗默 | 效果 | `laureate-paul-m-romer` | 4 费 | 未来 | `laureate-paul-m-romer-skill` | Paul M. Romer；效果取自「均衡」（§3） |
| 2018 | 威廉·诺德豪斯 | 效果 | `laureate-william-d-nordhaus` | 4 费 | 未来 | `laureate-william-d-nordhaus-skill` | William D. Nordhaus；效果取自「信息」（§3） |
| 2019 | 阿比吉特·班纳吉 | 效果 | `laureate-abhijit-banerjee` | 4 费 | 未来 | `laureate-abhijit-banerjee-skill` | Abhijit Banerjee；效果取自「制度」（§3） |
| 2019 | 埃丝特·迪弗洛 | 效果 | `laureate-esther-duflo` | 4 费 | 未来 | `laureate-esther-duflo-skill` | Esther Duflo；效果取自「增长」（§3） |
| 2019 | 迈克尔·克雷默 | 效果 | `laureate-michael-kremer` | 4 费 | 未来 | `laureate-michael-kremer-skill` | Michael Kremer；效果取自「行为」（§3） |
| 2020 | 保罗·米尔格罗姆 | 效果 | `laureate-paul-r-milgrom` | 4 费 | 未来 | `laureate-paul-r-milgrom-skill` | Paul R. Milgrom；效果取自「激励」（§3） |
| 2020 | 罗伯特·威尔逊 | 效果 | `laureate-robert-b-wilson` | 4 费 | 未来 | `laureate-robert-b-wilson-skill` | Robert B. Wilson；效果取自「均衡」（§3） |
| 2021 | 戴维·卡德 | 效果 | `laureate-david-card` | 4 费 | 未来 | `laureate-david-card-skill` | David Card；效果取自「信息」（§3） |
| 2021 | 吉多·因本斯 | 效果 | `laureate-guido-w-imbens` | 4 费 | 未来 | `laureate-guido-w-imbens-skill` | Guido W. Imbens；效果取自「制度」（§3） |
| 2021 | 乔舒亚·安格里斯特 | 效果 | `laureate-joshua-d-angrist` | 4 费 | 未来 | `laureate-joshua-d-angrist-skill` | Joshua D. Angrist；效果取自「增长」（§3） |
| 2022 | 本·伯南克 | 效果 | `laureate-ben-bernanke` | 4 费 | 未来 | `laureate-ben-bernanke-skill` | Ben Bernanke；效果取自「行为」（§3） |
| 2022 | 道格拉斯·戴蒙德 | 效果 | `laureate-douglas-diamond` | 4 费 | 未来 | `laureate-douglas-diamond-skill` | Douglas Diamond；效果取自「激励」（§3） |
| 2022 | 菲利普·迪布维格 | 效果 | `laureate-philip-dybvig` | 4 费 | 未来 | `laureate-philip-dybvig-skill` | Philip Dybvig；效果取自「均衡」（§3） |
| 2023 | 克劳迪娅·戈尔丁 | 效果 | `laureate-claudia-goldin` | 4 费 | 未来 | `laureate-claudia-goldin-skill` | Claudia Goldin；效果取自「信息」（§3） |
| 2024 | 达龙·阿杰姆奥卢 | 效果 | `laureate-daron-acemoglu` | 4 费 | 未来 | `laureate-daron-acemoglu-skill` | Daron Acemoglu；效果取自「制度」（§3） |
| 2024 | 詹姆斯·罗宾逊 | 效果 | `laureate-james-a-robinson` | 4 费 | 未来 | `laureate-james-a-robinson-skill` | James A. Robinson；效果取自「增长」（§3） |
| 2024 | 西蒙·约翰逊 | 效果 | `laureate-simon-johnson` | 4 费 | 未来 | `laureate-simon-johnson-skill` | Simon Johnson；效果取自「行为」（§3） |
| 2025 | 乔尔·莫基尔 | 效果 | `laureate-joel-mokyr` | 4 费 | 未来 | `laureate-joel-mokyr-skill` | Joel Mokyr；效果取自「激励」（§3） |
| 2025 | 彼得·豪伊特 | 效果 | `laureate-peter-howitt` | 4 费 | 未来 | `laureate-peter-howitt-skill` | Peter Howitt；效果取自「均衡」（§3） |
| 2025 | 菲利普·阿吉翁 | 效果 | `laureate-philippe-aghion` | 4 费 | 未来 | `laureate-philippe-aghion-skill` | Philippe Aghion；效果取自「信息」（§3） |
| 2026 | 迈克尔·伍德福德 | 效果 | `laureate-michael-woodford` | 4 费 | 未来 | `laureate-michael-woodford-skill` | Michael Woodford；效果取自「制度」（§3） |
| 2026 | 罗伯特·霍尔 | 效果 | `laureate-robert-e-hall` | 4 费 | 未来 | `laureate-robert-e-hall-skill` | Robert E. Hall；效果取自「增长」（§3） |

### 6. 统计

| 项 | 数值 |
| --- | --- |
| 得主记录 → 实体 | 1036 → 1028 |
| 随从 / 效果 | 664 / 364 |
| 物理学奖 | 230 |
| 化学奖 | 199 |
| 生理学或医学奖 | 235 |
| 文学奖 | 123 |
| 和平奖 | 140 |
| 经济学奖 | 101 |
| 星级 4★ / 5★ / 6★ / 7★ | 199 / 194 / 303 / 332 |
| 时代 工业 / 现代 / 未来 | 214 / 482 / 332 |
| 复用现有卡 / 设计文档 / 新增 | 8 / 1 / 1019 |

### 7. 落地说明

1. **名册是库存，不是上线清单**：全量 1028 张卡不可能一次性进池，落地时按学科、世纪或奖项分批取用，每批再按卡池规则裁剪。
2. **共享技能是占位**：同一奖项内 6 个技能轮转，意味着大量卡共用同一效果；实施阶段应优先把高辨识度人物替换为专属技能，其余保留共享模板。
3. **与精选设计的关系**：§7.9 的 32 张诺奖效果牌是面向科学概念的精选设计，本名册是面向人物的全量覆盖；同一得主可能同时出现在两边（例如克里克有 DNA 效果牌，又在名册里有随从卡），实施时二选一或同时保留均可。
4. **平衡交由实施阶段**：本批按设计阶段约定不做强度折算、卡池分布、效果牌占比与技能密度校验；`make data-check` 通过只说明文档链接与格式有效，不代表名册可直接进池。
5. **名称与机构**：机构按一张卡处理；同一中文译名指向不同人物时追加奖项后缀（当前仅「乔治·史密斯」一例）。
6. **敏感人物只做库存占位**：和平奖中的当代政治人物、争议人物与战争相关人物按本名册保留卡位，但在上线前必须按本库存「诺奖设计角度」部分 §十的风险清单复核；可替换为其机构、条约或概念卡。

## 第三部分 · 诺奖设计角度

### 一、结论

1. **现有卡池里的诺奖人物只有 8 位**：爱因斯坦、玛丽·居里、卢瑟福、海森堡、费米、弗莱明、摩尔根、薛定谔。全部是 20 世纪物理、化学、医学人物，全部以人科（`hominidae`）随从出场；其中 5 位同时是英雄（爱因斯坦、居里、卢瑟福、弗莱明、摩尔根）。
2. **诺奖相关概念已经零散进池，但学科严重偏科**。效果牌的 `taxon` 分布是：生物学 40、物理学 12、医学 9、化学 1（是毒素题材的「淬毒」，不是化学概念牌）、天文学 1、数学 1、经济学 3、文学 4、心理学 2、地球科学 2。化学、经济学、天文学、数学、地球科学几乎是空白。
3. **诺奖扩充的最高价值不是「再加一批攻 0 科学家」**，而是三条线：
   - 用诺奖概念补学科效果牌，尤其是化学、经济、天文、数学、地球科学；
   - 用少量诺奖人物补 5–7 星人科随从与英雄；
   - 优先选「机制能映射到现有技能模板」的得主；需要新机制的，按可共享的技能模板成批做，避免为单张卡新增独占子系统。
4. **最适合先做的 10 个机制原型**：PCR、CRISPR、MOF、点击化学、转座子、端粒、凋亡、朊病毒、mRNA、创造性破坏。它们分别对应复制、编辑、储存、合并、换位、复生、自毁、传染、预防、升星替换，都是现有卡池没有完整覆盖的机制位。

### 二、现有覆盖

#### 2.1 已有诺奖人物卡

| 人物 | 年份 · 奖项 | 获奖原因 | 当前卡 |
| --- | --- | --- | --- |
| 爱因斯坦 | 1921 · 物理 | 解释光电效应，提出光量子 | 随从 5 星 0/16；英雄 |
| 玛丽·居里 | 1903 · 物理；1911 · 化学 | 放射性研究；发现钋与镭 | 随从 5 星 0/16；英雄 |
| 卢瑟福 | 1908 · 化学 | 元素衰变与放射性物质化学 | 随从 6 星 0/20；英雄 |
| 海森堡 | 1932 · 物理 | 创立量子力学 | 随从 7 星 0/26 |
| 费米 | 1938 · 物理 | 中子辐照产生新放射性元素 | 随从 7 星 0/26 |
| 弗莱明 | 1945 · 医学 | 发现青霉素 | 随从 4 星 5/7；英雄 |
| 摩尔根 | 1933 · 医学 | 确立染色体遗传理论 | 随从 5 星 0/16；英雄 |
| 薛定谔 | 1933 · 物理 | 原子理论的新形式 | 随从 7 星 0/26 |

#### 2.2 已有诺奖相关概念卡

| 概念卡 | 对应得主 | 类型 |
| --- | --- | --- |
| DNA 双螺旋结构 | 克里克、沃森、威尔金斯 1962 医学 | 效果牌 |
| 光电效应 | 爱因斯坦 1921 物理 | 效果牌 |
| 质能等价 | 爱因斯坦（狭义相对论推论） | 效果牌 |
| 广义相对论 / 狭义相对论 | 爱因斯坦 | 效果牌 |
| 引力波 | 魏斯、巴里什、索恩 2017 物理 | 效果牌 |
| X 射线机 | 伦琴 1901 物理 | 随从 |
| 青霉素·提纯 | 弗莱明、钱恩、弗洛里 1945 医学 | 效果牌 |
| 青蒿 | 屠呦呦 2015 医学 | 随从（植物，不是人物卡） |
| 布朗运动 | 佩兰 1926 物理（爱因斯坦 1905 解释） | 效果牌 |
| 光合作用 | 卡尔文 1961 化学 | 效果牌 |

**关键判断**：现有诺奖人物是「人物卡」，诺奖概念是「效果卡」，两条线都只覆盖了 20 世纪上半叶的物理、化学、医学。2000 年以后的诺奖、经济学、文学、和平、地球科学基本没有。

### 三、筛选标准

1. **机制可落地**：优先复用现有 16 个关键词与技能模板；需要新模板的单独标注，不把「需要新子系统」的卡混进首批。
2. **有槽位**：效果牌落在 `science.<学科>` / `humanities.<学科>` / `society.<学科>` 叶子；随从避开「同玩法 + 星级 + 身材」与「同玩法 + 星级 + 效果技能集」撞车。
3. **叙事辨识度**：中文玩家能认出，或概念本身够硬（PCR、CRISPR、mRNA、黑洞、博弈论）。
4. **一人一张**：同一人多次获奖只做一个实体；另一奖写进介绍或技能，不做第二张卡。
5. **风险可控**：避开政治敏感、战争争议、优生学、人体实验题材；和平奖与文学奖优先做效果牌叙事，不做随从。

### 四、最适合做「概念效果牌」的诺奖主题

| 机制原型 | 得主（年份 · 奖项） | 获奖原因 | 卡牌角度 | 建议落位 | 落地成本 |
| --- | --- | --- | --- | --- | --- |
| 量子化伤害 | 普朗克 1918 · 物理 | 因发现能量量子而对物理学发展作出贡献（黑体辐射是其动机之一） | 把一次伤害拆成多次 1 点，专门触发「造成伤害后」 | 物理 / 爬行纲、水螅纲 | 需新增「分段伤害」模板 |
| 互补双形态 | 玻尔 1922 · 物理 | 原子结构与辐射的量子理论 | 一个随从同时获得两种互斥关键词（嘲讽 + 潜行、连击 + 减伤） | 物理 / 机械、鸟纲 | 复用「授予关键词」模板 |
| 反物质 | 狄拉克 1933 · 物理 | 原子理论的新形式，预言反粒子 | 死亡时召唤一个攻血互换的复制体 | 物理 / 机械 | 需新增「属性互换召唤」 |
| 不相容 | 泡利 1945 · 物理 | 不相容原理 | 战斗开始时，消灭与另一个敌方随从身材完全相同的随从 | 物理 / 机械 | 需新增「按身材匹配」 |
| 路径叠加 | 费曼 1965 · 物理 | 量子电动力学 | 本回合下一张效果牌触发两次 | 物理 / 机械 | 需新增「效果复制」 |
| 宇称不守恒 | 杨振宁、李政道 1957 · 物理 | 弱相互作用中宇称不守恒 | 只影响最左侧或最右侧，打破对称 | 物理 / 鸟纲、机械 | 复用「最左侧」目标 |
| 量子纠缠 | 阿斯佩、克劳泽、蔡林格 2022 · 物理 | 纠缠光子实验，验证贝尔不等式 | 两个友方随从共享生命值或成长 | 物理 / 机械、水螅纲 | 需新增「链接」 |
| 神经网络 | 霍普菲尔德、辛顿 2024 · 物理 | 人工神经网络的机器学习基础 | 每回合结束学习并复制攻击力最高随从的攻击力 | 机械 | 需新增「复制最高攻」 |
| 量子隧穿 | 克拉克、德沃雷、马蒂尼斯 2025 · 物理 | 宏观量子隧穿与电路能量量子化 | 无视嘲讽，或攻击时穿过阻挡 | 机械、鸟纲 | 复用「无视嘲讽」 |
| PCR 扩增 | 穆利斯 1993 · 化学 | 发明聚合酶链式反应 | 复制一个友方随从；复制体可继续复制，每回合限一次 | 鱼纲、原生生物界 | 部分复用「复制」 |
| CRISPR 编辑 | 杜德纳、夏彭蒂耶 2020 · 化学 | 开发 CRISPR-Cas9 基因编辑 | 复制或移除一个随从的关键词 / 技能 | 化学 / 机械、真菌界 | 需新增「技能编辑」 |
| MOF 储存 | 亚吉、北川进、罗布森 2025 · 化学 | 金属有机框架 | 战吼储存一个友方随从的属性，亡语释放 | 机械、植物界 | 需新增「储存」 |
| 点击化学 | 贝尔托齐、梅尔达尔、夏普莱斯 2022 · 化学 | 点击化学与生物正交化学 | 合并两个友方随从的身材与关键词 | 化学 / 机械 | 需新增「合并」 |
| 转座子 | 麦克林托克 1983 · 医学 | 发现转座子 | 交换两个友方随从的位置，重排相邻光环 | 昆虫纲、植物界 | 需新增「换位」 |
| 端粒 | 布莱克本、格雷德、绍斯塔克 2009 · 医学 | 端粒与端粒酶保护染色体 | 复生次数 +1，或复生后保留成长 | 水螅纲 | 部分复用「复生」 |
| 凋亡 | 布伦纳、霍维茨、苏尔斯顿 2002 · 医学 | 器官发育与程序性细胞死亡的遗传调控 | 消灭一个友方随从，把属性分给相邻 | 真菌界、昆虫纲 | 复用「消灭友方」+ 成长 |
| 朊病毒 | 普鲁西纳 1997 · 医学 | 发现朊病毒这一新感染原理 | 传染层不随死亡清除，死亡时传播 | 病毒 | 需新增「传染持久化」 |
| mRNA 疫苗 | 卡里科、魏斯曼 2023 · 医学 | 核苷碱基修饰，使 mRNA 疫苗可行 | 战斗开始前给随从护盾，或免疫一次传染 | 病毒、医学 | 复用「护盾」 |
| 免疫耐受 | 布伦考、拉姆斯德尔、坂口志文 2025 · 医学 | 外周免疫耐受 | 你的随从不会被敌方减益选中，或清除自身传染 | 细菌域、爬行纲 | 部分复用「净化」 |
| 创造性破坏 | 莫基尔、阿吉翁、豪伊特 2025 · 经济 | 技术进步与创造性破坏驱动的持续增长 | 消灭一个友方随从，发现一张高 1 星的随从 | 经济 / 机械 | 需新增「升星替换」 |
| 纳什均衡 | 纳什 1994 · 经济 | 非合作博弈的均衡分析 | 双方攻击力最高随从相同时获得加成，或预测敌方目标 | 经济 / 哺乳纲 | 需新增「比较双方」条件 |
| 损失厌恶 | 卡尼曼 2002 · 经济 | 把心理学洞见引入经济学，研究不确定性下的判断 | 失去随从或生命时获得强化 | 经济 / 哺乳纲 | 需新增「失去触发」 |
| 公地治理 | 奥斯特罗姆 2009 · 经济 | 公共资源的经济治理 | 全体友方共享一个成长池，每回合有上限 | 经济 / 植物界 | 需新增「共享池」 |

### 五、最适合做「人科随从 / 英雄」的诺奖人物

| 人物 | 年份 · 奖项 | 获奖原因 | 机制角度 | 建议玩法 | 建议星级 | 备注 |
| --- | --- | --- | --- | --- | --- | --- |
| 巴甫洛夫 | 1904 · 医学 | 消化系统生理学研究；条件反射是其后续工作 | 每当一个友方随从获得成长，本随从获得 +1 攻击力 | 哺乳纲 | 4–5 | 现有 `ally-growth-self-attack` 模板可直接复用 |
| 科赫 | 1905 · 医学 | 发现结核杆菌 | 战斗开始时，消灭一个攻击力不高于 X 的敌方随从 | 细菌域 | 4–5 | 复用「吞噬」类模板 |
| 兰德施泰纳 | 1930 · 医学 | 发现人类血型 | 你的同族随从获得 +1/+1，异族获得 -1/-1 | 哺乳纲 | 4–5 | 需新增「同族 / 异族」条件光环 |
| 班廷 | 1923 · 医学 | 发现胰岛素 | 战吼：使一个友方随从获得生命值 | 哺乳纲 | 3–4 | 复用「+生命」模板 |
| 埃尔利希 | 1908 · 医学 | 免疫学研究；「魔弹」是后续思想 | 对攻击力最低的敌方随从造成伤害 | 细菌域 | 4–5 | 复用「最低攻」目标 |
| 麦克林托克 | 1983 · 医学 | 发现转座子 | 交换两个友方随从的位置 | 昆虫纲、植物界 | 4–5 | 需新增「换位」，与「转座子」效果牌二选一 |
| 布莱克本 | 2009 · 医学 | 端粒与端粒酶 | 复生次数 +1，或复生后保留成长 | 水螅纲 | 5–6 | 复用「复生」 |
| 山中伸弥 | 2012 · 医学 | 成熟细胞可重编程为多能细胞 | 使一个友方随从回手并保留成长 | 水螅纲、原生生物界 | 5 | 需新增「主动回手」 |
| 卡里科 | 2023 · 医学 | 核苷碱基修饰使 mRNA 疫苗可行 | 你的随从获得护盾，或首次传染无效 | 病毒 | 5–6 | 复用「护盾」；可做英雄 |
| 杜德纳 | 2020 · 化学 | CRISPR-Cas9 基因编辑 | 复制或移除一个随从的关键词 | 机械、真菌界 | 6 | 需新增「技能编辑」；可做英雄 |
| 辛顿 | 2024 · 物理 | 人工神经网络的机器学习基础 | 每回合结束复制攻击力最高随从的攻击力 | 机械 | 6 | 需新增「复制最高攻」；与「通用人工智能」联动 |
| 纳什 | 1994 · 经济 | 非合作博弈均衡 | 双方条件比较后获得加成 | 哺乳纲、机械 | 5–6 | 需新增「比较双方」；可做英雄 |
| 鲍林 | 1954 · 化学；1962 · 和平 | 化学键本质；核试验禁试 | 相邻友方随从互相获得 +1/+1 | 哺乳纲 | 5–6 | 双奖人物只做一张；可做英雄 |
| 克里克 / 沃森 / 威尔金斯 | 1962 · 医学 | 核酸分子结构及其信息传递意义 | 复制一个友方随从的技能给另一个 | 哺乳纲、机械 | 5–6 | DNA 效果牌已有，人物可做高星随从 |
| 奥斯特罗姆 | 2009 · 经济 | 公共资源治理 | 全体友方共享一个成长池 | 植物界、哺乳纲 | 5 | 需新增「共享池」；更适合效果牌 |

**人物卡的设计口径**：继续沿用现有学者模板——`taxon: hominidae`、攻击力 0、生命值取同星级上限 ×1.3（4 星 0/13、5 星 0/17、6 星 0/21、7 星 0/26，允许 ±1）。弗莱明式的低攻高功能卡是例外，不作为默认。

### 六、第二梯队：按学科的可选池

以下只给「人物 + 角度」，不展开数值；括号内是设计隐喻。

#### 物理

- 伦琴 1901（X 射线）：透视、使潜行失效；已有 X 射线机，人物可做英雄。
- 密立根 1923（基本电荷、油滴实验）：精确计量、商店费用取整。
- 拉曼 1930（拉曼散射）：把敌方一个随从的属性散射给友方；与「拟态」区分。
- 贝特 1967（恒星核聚变）：相邻同族合并成长。
- 鲁斯卡 / 宾尼希 / 罗雷尔 1986（电子显微镜、扫描隧道显微镜）：查看牌库顶或揭示敌方手牌。
- 阿什金 2018（光镊）：把一个敌方随从移到最左侧；需换位模板。
- 穆鲁 / 斯特里克兰 2018（啁啾脉冲放大）：使一个随从的攻击力翻倍直到回合结束。
- 阿戈斯蒂尼 / 克劳斯 / 吕利耶 2023（阿秒脉冲）：本回合下一张效果牌额外触发一次。
- 彭罗斯 / 根策尔 / 盖兹 2020（黑洞）：吞噬低攻随从并获得其属性；现有「吞噬」机制。
- 皮布尔斯 / 麦耶 / 奎洛兹 2019（宇宙学、系外行星）：发现一张本局玩法随从；现有「发现」。
- 真锅淑郎 / 哈塞尔曼 / 帕里西 2021（气候模型）：每 3 回合触发一次的长期预测。
- 梶田隆章 / 麦克唐纳 2015（中微子振荡）：随从在两种形态间切换。

#### 化学

- 哈伯 1918（合成氨）：把生命或攻击转化为金币 / 成长；题材有战争争议，谨慎。
- 鲍林 1954（化学键）：相邻友方互相加成；与现有相邻光环区分。
- 克里克 / 沃森 / 威尔金斯 1962（DNA）：复制一个随从的技能给另一个。
- 桑格 1958 / 1980（蛋白质与核酸测序）：查看牌库顺序并选择一张。
- 普里戈金 1977（耗散结构）：每次受伤获得 +1/+1，远离平衡的自组织。
- 泽维尔 1999（飞秒化学）：战斗开始时先结算。
- 诺尔斯 / 野依良治 / 夏普莱斯 2001（不对称催化）：只增益最左侧或单侧。
- 海克 / 铃木章 / 根岸英一 2010（钯催化交叉偶联）：两个相邻友方共享攻击力。
- 谢赫特曼 2011（准晶体）：非周期排列，相邻加成不重复但可叠不同种类。
- 贝齐格 / 莫纳 / 黑尔 2014（超分辨荧光显微）：使潜行失效并标记。
- 巴文迪 / 布鲁斯 / 叶基莫夫 2023（量子点）：尺寸决定性质，星级越低成长越快。
- 贝克 / 哈萨比斯 / 江珀 2024（蛋白质设计、AlphaFold）：发现并召唤一张随从。
- 克拉姆 / 莱恩 / 佩德森 1987（主客体化学）：捕获一个敌方随从，本回合无法攻击。
- 克鲁岑 / 莫利纳 / 罗兰 1995（臭氧层化学）：全体减伤 1 的保护层。

#### 生理学或医学

- 埃尔利希 1908（免疫学、魔弹）：定向消灭最低攻随从。
- 缪勒 1946（X 射线诱变）：随机但种子固定地改变一个随从的属性。
- 米尔斯坦 / 克勒 1984（单克隆抗体）：复制一个友方随从。
- 利根川进 1987（抗体多样性）：发现并获得一个技能。
- 哈特韦尔 / 纳斯 / 亨特 2001（细胞周期）：每 3 回合获得一次成长。
- 劳特伯 / 曼斯菲尔德 2003（MRI）：揭示敌方潜行或手牌。
- 马歇尔 / 沃伦 2005（幽门螺杆菌）：减伤 2，耐酸存活。
- 卡佩奇 / 史密斯 / 埃文斯 2007（基因敲除）：移除一个敌方随从的关键词。
- 博伊特勒 / 霍夫曼 / 斯坦曼 2011（先天免疫、树突状细胞）：首次受到伤害时减 2。
- 格登 / 山中伸弥 2012（细胞重编程）：回手并保留成长。
- 谢克曼 / 罗思曼 / 聚德霍夫 2013（囊泡运输）：手牌与战场之间传递。
- 奥基夫 / 莫泽夫妇 2014（位置细胞、网格细胞）：换位与位置加成。
- 大隅良典 2016（自噬）：吞噬一个友方低攻随从获得其属性。
- 霍尔 / 罗斯巴什 / 扬 2017（昼夜节律）：回合开始和结束各触发一次。
- 艾利森 / 本庶佑 2018（免疫检查点）：解除敌方控制，获得潜行或无视嘲讽。
- 凯林 / 拉特克利夫 / 塞门扎 2019（氧感应）：生命低于一半时 +2/+2。
- 朱利叶斯 / 帕塔普蒂安 2021（温度与触觉受体）：受到伤害时对攻击者造成 1 点伤害。
- 帕博 2022（古人类基因组）：复活一个已死亡随从。
- 安布罗斯 / 鲁夫昆 2024（microRNA）：微调，+1/+1 或 -1/-1。
- 法尔 / 梅洛 2006（RNA 干扰）：使一个敌方随从失去技能。
- 爱德华兹 2010（体外受精）：召唤一个 1/1 幼体。
- 布洛贝尔 1999（蛋白质分选信号）：从手牌定向召唤一个随从。
- 弗里施 / 洛伦茨 / 廷伯根 1973（动物行为学）：你的动物随从获得 +1 攻击力。
- 雅各布 / 利沃夫 / 莫诺 1965（操纵子、基因调控）：根据条件获得或失去关键词。
- 尼伦伯格 / 霍拉纳 / 霍利 1968（遗传密码）：把一个随从的属性翻译给另一个。
- 伯内特 / 梅达沃 1960（获得性免疫耐受）：你的随从不会被敌方效果选中。

#### 经济

- 哈耶克 1974（货币与经济波动、制度现象相互依存）：信息不对称、知识分散；查看敌方手牌但自己手牌公开。
- 萨缪尔森 1970（静态与动态经济理论）：双方随从数量相同时获得加成。
- 阿罗 1972（一般均衡与福利理论）：三选一但不能全部满足。
- 弗里德曼 1976（消费分析、货币史与理论）：金币上限 +1，但每回合 -1。
- 西蒙 1978（经济组织内的决策过程）：有限理性，随机但可复现。
- 森 1998（福利经济学）：使一个低星随从获得高星属性。
- 斯蒂格利茨 / 阿克洛夫 / 斯彭斯 2001（信息不对称市场）：隐藏一个随从的真实属性。
- 默顿 / 斯科尔斯 1997（衍生品定价）：延迟支付，下回合获得收益。
- 克鲁格曼 2008（贸易模式与经济活动区位）：位置与相邻加成。
- 塞勒 2017（行为经济学）：助推，强制敌方攻击指定目标。
- 班纳吉 / 迪弗洛 / 克雷默 2019（减贫实验方法）：本局随机效果可复现，重掷一次。
- 迪顿 2015（消费、贫困与福利）：低金币时获得成长。
- 米尔格罗姆 / 威尔逊 2020（拍卖理论）：用金币竞得一个随从。
- 罗斯 / 沙普利 2012（稳定分配与市场设计）：配对两个友方随从，互相加成。
- 哈特 / 霍姆斯特伦 2016（契约理论）：延迟奖励，完成条件后获得。
- 奥曼 / 谢林 2005（冲突与合作）：威慑，敌方攻击最高随从无法攻击。
- 戈尔丁 2023（女性劳动市场）：同工同酬，人科随从获得加成；注意不要标签化。
- 阿西莫格鲁 / 约翰逊 / 罗宾逊 2024（制度如何形成并影响繁荣）：每回合第一个购买的随从 +1/+1。

### 七、文学与和平：只适合效果牌叙事

#### 文学

文学奖不适合做随从机制，但适合做「叙事型效果牌」，延续现有鲁迅四张卡的路子（呐喊、彷徨、狂人日记、阿Q精神）。

| 得主 | 年份 · 奖项 | 获奖原因 | 可用角度 |
| --- | --- | --- | --- |
| 泰戈尔 | 1913 · 文学 | 敏感、清新而优美的诗歌 | 群体小增益、灵感 |
| 罗曼·罗兰 | 1915 · 文学 | 理想主义文学 | 英雄强化、士气 |
| 海明威 | 1954 · 文学 | 叙事艺术，《老人与海》 | 单随从孤军加成 |
| 加缪 | 1957 · 文学 | 以明澈的严肃照亮人的良知问题 | 荒诞：随机但可复现；无法被指定 |
| 马尔克斯 | 1982 · 文学 | 魔幻与现实结合 | 回手、循环、变形 |
| 莫言 | 2012 · 文学 | 幻觉现实主义融合民间故事、历史与当代 | 植物、生命、土地 |
| 石黑一雄 | 2017 · 文学 | 情感力量，揭示与世界联系的错觉 | 回手、失去成长 |
| 韩江 | 2024 · 文学 | 以诗性散文直面历史创伤、暴露人的脆弱 | 植物化、变形、减攻 |
| 克拉斯诺霍尔卡伊 | 2025 · 文学 | 末世恐惧中重申艺术的力量 | 全场伤害、恐惧 |

#### 和平

和平奖的机制隐喻通常是「治疗、减伤、停战、阻止攻击」，适合效果牌，不适合随从；政治人物慎做人物卡。

| 得主 | 年份 · 奖项 | 获奖原因 | 可用角度 |
| --- | --- | --- | --- |
| 杜南 | 1901 · 和平 | 红十字运动与日内瓦公约 | 治疗、减伤 |
| 南丁格尔 | 1907 · 和平 | 现代护理奠基 | 治疗、生命恢复 |
| 马丁·路德·金 | 1964 · 和平 | 非暴力民权运动 | 群体成长、敌方无法攻击 |
| 特蕾莎修女 | 1979 · 和平 | 为穷人服务 | 低血保护、治疗 |
| 曼德拉 | 1993 · 和平 | 和平结束种族隔离 | 解除控制、团结 |
| 无国界医生 | 1999 · 和平 | 人道医疗 | 全体治疗 |
| 尤努斯 | 2006 · 和平 | 小额信贷与扶贫 | 金币、经济 |
| 马拉拉 | 2014 · 和平 | 儿童教育权 | 发现、成长 |
| 玛丽亚·科里纳·马查多 | 2025 · 和平 | 推动委内瑞拉民主权利 | 政治敏感，不建议 |

### 八、玩法 × 诺奖主题落位

| 玩法 | 现有核心 | 适合挂的诺奖主题 |
| --- | --- | --- |
| 哺乳纲 | 相邻同族、少量高质量、成长 | 巴甫洛夫、兰德施泰纳、鲍林、纳什、卡尼曼、文学 / 和平人物 |
| 鸟纲 | 高攻低血、无视嘲讽、突进 | 量子隧穿、宇称不守恒、光镊、网格细胞 |
| 爬行纲 | 低攻高血、减伤、毒 | 免疫耐受、幽门螺杆菌、埃尔利希、臭氧层 |
| 鱼纲 | 群聚、分裂 | PCR、桑格测序、古 DNA |
| 昆虫纲 | 铺场、亡语 | 转座子、凋亡、动物行为学 |
| 头足纲 | 复制、潜行、拟态 | 超分辨显微、神经科学、拉曼散射 |
| 水螅纲 | 复生、复苏 | 端粒、山中伸弥重编程、量子纠缠 |
| 植物界 | 播种、延迟成长 | 麦克林托克、MOF、公地治理、莫言 |
| 真菌界 | 孢子、分解 | 弗莱明、大隅良典自噬、普里戈金耗散结构 |
| 细菌域 | 传染、转化、引爆 | 科赫、马歇尔、免疫耐受、朊病毒 |
| 原生生物界 | 吞噬、分裂 | 凋亡、细胞周期、PCR、自噬 |
| 病毒 | 传染、潜伏、爆发 | 卡里科 mRNA、普鲁西纳朊病毒、zur Hausen HPV |
| 机械 | 装配、零件、无视嘲讽 | 辛顿神经网络、2025 量子隧穿、MOF、点击化学 |

### 九、数值、星级与落地成本速查

- **攻 0 随从生命值**：同星级生命上限 ×1.3，允许 ±1。4 星 0/13、5 星 0/17、6 星 0/21、7 星 0/26。
- **技能数量**：1–2 星最多 1 个；3–4 星最多 2 个；5–7 星最多 3 个，且登记关键词不超过 2 个。
- **效果牌**：`cost` 0–4；4 费只允许 6–7 星；`skills` 必填且非空；不得有攻防。
- **时代**：1901–1945 的得主与概念多用 `modern`；1946–1990 视题材取 `modern` 或 `future`；1991 年以后取 `future`。时代是题材归属，不是严格编年。
- **效果牌星级上限**：任一星级的效果牌数量不得超过该星级全部卡牌的 30%。
- **首批优先复用现有模板**：成长、光环、战吼、亡语、战斗开始伤害、传染、护盾、复生、无视嘲讽、发现、复制、刷新。需要新模板的机制（换位、链接、合并、储存、技能编辑、升星替换、共享池）放到第二批，先做技能模板设计再落卡。

### 十、风险与边界

1. **诺奖是品牌**：卡面可以用「诺贝尔奖」描述史实，但不要使用奖章、官方 logo、官方字体；不要把「获得诺贝尔奖」本身做成机制梗。
2. **同一人多次获奖**：居里（1903 物理、1911 化学）、鲍林（1954 化学、1962 和平）、夏普莱斯（2001、2022 化学）、桑格（1958、1980 化学）、巴丁（1956、1972 物理）。只做一张，另一奖写进介绍或技能。
3. **不要误列**：甘地从未获诺贝尔和平奖；罗莎琳德·富兰克林未获 DNA 诺奖；图灵、冯·诺依曼、香农、高德纳（图灵奖）、萨根、奥本海默、爱迪生、特斯拉、巴斯德、孟德尔、达尔文、林奈、拉马克都不是诺奖得主。现有卡池里这些人已经存在，但不要放进「诺奖人物」表。
4. **阿尔弗雷德·诺贝尔本人不是诺奖得主**：他是炸药发明者，适合做机械 / 炸药题材卡，不适合放进诺奖人物表。
5. **争议题材慎用**：哈伯（合成氨与化学武器）、缪勒（DDT）、莫尼兹（脑白质切断术）、优生学、人体实验；政治人物（昂山素季、达赖、奥巴马、基辛格等）不建议做卡。
6. **文学 / 和平的政治风险**：优先做组织或概念（红十字、无国界医生、非暴力），少做当代政治人物。
7. **攻 0 学者卡已经很多**：人科随从 104 张、攻 0 人科 48 张。再加人物卡容易在「同玩法 + 星级 + 技能集」上撞车；新诺奖人物优先做效果牌或英雄，不要批量追加攻 0 随从。

### 十一、首批 12 张建议（前 10 张优先）

按「机制缺口 + 中文辨识度 + 可拆成技能模板」排序：

1. **PCR**（穆利斯 1993 化学）— 效果牌，鱼纲 / 原生生物界，5 星 3 费：复制一个友方随从；每回合限一次。补「复制链」。
2. **CRISPR**（杜德纳 / 夏彭蒂耶 2020 化学）— 效果牌，化学 / 机械、真菌界，6 星 3 费：使一个友方随从获得另一个友方随从的一个技能，或移除敌方一个关键词。补「技能编辑」。
3. **MOF**（亚吉 / 北川进 / 罗布森 2025 化学）— 随从，机械 / 植物界，5 星 0/17：战吼储存一个友方随从的当前属性，亡语释放。补「储存」。
4. **点击化学**（贝尔托齐 / 梅尔达尔 / 夏普莱斯 2022 化学）— 效果牌，化学 / 机械，5 星 3 费：合并两个友方随从。补「合并」。
5. **转座子**（麦克林托克 1983 医学）— 效果牌，昆虫纲 / 植物界，4 星 1 费：交换两个友方随从的位置。补「换位」。
6. **端粒**（布莱克本 / 格雷德 / 绍斯塔克 2009 医学）— 效果牌，水螅纲，5 星 2 费：使一个友方随从的复生次数 +1，或复生后保留成长。补「复生上限」。
7. **凋亡**（布伦纳 / 霍维茨 / 苏尔斯顿 2002 医学）— 效果牌，真菌界 / 昆虫纲，4 星 2 费：消灭一个友方随从，使相邻随从获得其属性。补「自毁增益」。
8. **朊病毒**（普鲁西纳 1997 医学）— 随从，病毒，6 星：传染层不随死亡清除，死亡时传播给随机敌方随从。补「传染持久化」。
9. **mRNA 疫苗**（卡里科 / 魏斯曼 2023 医学）— 效果牌，病毒 / 医学，4 星 2 费：使你的随从获得护盾，或免疫一次传染。补「预防」。
10. **创造性破坏**（莫基尔 / 阿吉翁 / 豪伊特 2025 经济）— 效果牌，经济 / 机械，5 星 2 费：消灭一个友方随从，发现一张高 1 星的随从。补「升星替换」。
11. （第二批）**量子纠缠**（阿斯佩 / 克劳泽 / 蔡林格 2022 物理）— 效果牌，物理 / 机械，5 星 3 费：使两个友方随从共享生命值。补「链接」。
12. （第二批）**神经网络**（霍普菲尔德 / 辛顿 2024 物理）— 随从，机械，6 星：回合结束时获得攻击力最高随从的攻击力。补「学习」。

### 十二、2026 年新增得主

2026 年名单已进入官方 API；以下只列与机制设计最相关的角度，完整原文见 [nobel-laureates.md](nobel-laureates.md)。

| 得主 | 年份 · 奖项 | 获奖原因 | 设计角度 |
| --- | --- | --- | --- |
| Francis Halzen | 2026 · 物理 | 对 IceCube 中微子天文台的贡献与高能天体物理中微子的发现 | 穿透 / 探测：使潜行失效，或攻击无视嘲讽；中微子几乎不与物质作用，适合「穿过阻挡」 |
| Henri B. Kagan、Kenso Soai | 2026 · 化学 | 不对称有机合成中的非线性效应与自催化 | 自催化：每回合复制一次自身成长；与 PCR 的「复制链」区分，走化学 / 机械线 |
| Karl Deisseroth、Peter Hegemann、Georg Nagel | 2026 · 医学 | 光门控离子通道与光遗传学 | 光控开关：使一个随从本回合无法攻击，或获得潜行；强「开关」机制，适合头足纲 / 机械 |
| Robert E. Hall、Michael Woodford | 2026 · 经济 | 宏观经济稳定的微观基础 | 稳定器：生命或金币低于阈值时触发保护；经济 / 哺乳纲 |
| Anne Carson | 2026 · 文学 | 与古典传统对话、为当代文学创造新形式 | 叙事效果：变形、复制、古典回声 |
| Navi Pillay | 2026 · 和平 | 促进和平与国际法 | 规则 / 停战效果；政治法律人物慎做人物卡 |

其中光遗传学的「开关」机制最值得优先评估：现有卡池没有「主动关闭一个随从」的效果，可补控制轴；自催化与 PCR 机制相近，二者只做一张，避免重复。

### 十三、下一步

- 12 张里只有 mRNA（护盾）、端粒（复生）、PCR（复制）能部分复用现有模板；其余都需要新技能模板。建议先写「换位、链接、合并、储存、技能编辑、升星替换、共享池」这 7 个模板，再落卡，不要先做数值。
- 已在本库存「卡牌设计」部分 §7.9 做了一版只含效果牌的 32 张近似方案：不实现上述 7 个新子系统，用属性互换、复制、沉默、冻结、献祭、群体身材等现有原语表达概念；原机制仍按前一条单独评估。
- 已在本库存「诺奖全量名册」部分做了全量覆盖：1028 位得主/机构一人一卡，科学三奖做随从、文学/和平/经济做效果牌，技能用 36 个奖项效果池按年份轮转，名册阶段不做平衡。
- 如果做「诺奖英雄」：优先鲍林（双奖）、卡里科（mRNA）、杜德纳（CRISPR）、辛顿（神经网络）、纳什（博弈论）。英雄必须与同名随从同 id、同 `categories`，且技能只能主动 / 被动。
- 改卡后必须跑 `python3 scripts/validate_docs.py`，并同步 `card-system.md` 的数据总览与 `cards/README.md` 索引。

## 相关文档

- [诺贝尔奖得主与获奖理由（中文版）](nobel-laureates-zh.md)
- [卡牌规范与数据](../card-system.md)
- [内容参考材料](README.md)
- [内容素材库](../../../../content-backlog.md)
- [版本路线与风险](../../../../version-roadmap.md)
