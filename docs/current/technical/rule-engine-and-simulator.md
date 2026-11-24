# 规则引擎与模拟器

> 状态：已实现（v0.1 M0；M1a 的增量见[房间服务与联网](room-service/README.md)）
> 代码：`server/crates/rule-engine`、`server/crates/sim`、`scripts/`、`Makefile`

本文记录 v0.1 M0 已实现的规则引擎与模拟器的现状：工程结构、引擎边界、随从属性分层、数据生成、命令与验证方式；不描述产品规则与后续里程碑规划。

## 1. 工程结构

```
server/
  Cargo.toml                 # workspace：crates/rule-engine、crates/sim
  rust-toolchain.toml        # 工具链 1.94.1，edition 2024
  data/v0.1/
    cards.json               # 生成物：随从与效果牌
    heroes.json              # 生成物：英雄卡（只含身份与技能）
    skills.json              # 生成物：259 个技能
    config.json              # 生成物：经济与对局配置
    taxonomy.json            # 生成物：科学分类树与玩法 id 清单
  crates/
    rule-engine/             # 纯库
      src/state.rs           # GameState / PlayerState / MinionInstance / MinionStats
      src/data.rs            # Catalog（cards / heroes / skills）、Playstyle / Category 类型与加载期校验（deny_unknown_fields）
      src/taxonomy.rs        # 科学分类树：祖先匹配 is_within / 路径 path / 叶子判定
      src/playstyles.rs      # 开局玩法抽取 draw 与 PLAYSTYLES_PER_GAME
      src/effects/           # 结构化效果校验（validate）、解释执行（exec）、光环（auras）；parse 仅保留为 DSL 作者工具
      src/intent.rs          # 协议十三种意图（含 ChooseDiscover、ChooseTarget、AbandonTarget）与 11 个规则拒绝码（各自映射到对应的协议错误码）
      src/engine.rs          # Engine::new（含玩法参数）/ apply / start_game / start_turn
      src/achievements.rs    # 局内成就标识、成长记账与各阶段奖励结算
      src/recruit.rs         # 商店、买/卖/刷新/冻结/升时代/打出/结束招募
      src/triple.rs          # 三连与发现
      src/combat.rs          # 战斗模拟
      src/settle.rs          # 配对、finish_recruit、伤害、淘汰、排名
      src/rng.rs             # xoshiro256++ 与五个标签流；seed_from_u64（M1a 由 sim 迁入）
      src/hash.rs            # FNV-1a 规范编码与状态哈希
      src/view.rs            # 逐座位视图 SeatView（M1a 新增，供 room-service 映射协议视图）
      tests/                 # 集成测试文件（共享 helper 在 tests/common/；含 playstyles.rs、taxonomy.rs）
    sim/                     # 可执行：CLI 模拟器 + 确定性 AI
      src/lib.rs             # run_games / play_game / replay；重导出 rule_engine::rng::seed_from_u64
      src/ai.rs              # M0 确定性 AI 策略（按视图消费目标待选）
      src/bot/               # M1a：view 驱动的机器人（brain.rs）与真实 WebSocket 客户端（client.rs）
      src/report.rs          # 日志与基线报告
      src/main.rs            # play / batch / replay / report / check / bot
      tests/sim_cli.rs       # CLI 冒烟与字段契约
      tests/sim_bot.rs       # 机器人决策优先级
      tests/sim_determinism.rs # 同一局重放 100 次
scripts/
  gen_card_data.py           # 实体文档与 taxonomy.md → server/data/v0.1/*.json（含 taxonomy.json）
  gen_text.py                # 实体文档与人工 key → client/text/zh_CN.csv
  i18n_terms.py              # 枚举、关键词与文本 id 的命名规则
  audit_effects.py           # effects DSL 审计
  validate_docs.py           # 文档与实体数据校验
  gen_protocol_gdscript.py   # 协议 GDScript 常量（M1 使用）
  gen_protocol_docs.py       # 协议字段表
  validate_protocol.py       # 协议样例校验
tests/                       # Python 标准库 unittest
```

依赖方向固定为 `sim → protocol`、`sim → rule-engine`；`rule-engine` 不依赖 `sim`、tokio、sqlx、协议 crate，不做文件 I/O，不读时钟与环境变量。

## 2. 引擎边界

- `Engine::apply(state, seat, intent)` 是唯一的状态变更入口，返回 `Outcome`（本意图产生的事件）；拒绝时返回 `Reject { code, message }` 且不改变状态。
- 拒绝码与核心规则 §3 一致：`NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY`、`NO_SPACE`、`INVALID_STAGE`、`NO_SUCH_MINION`、`ALREADY_MAX_ERA`、`CARD_NOT_IN_HAND`、`HERO_POWER_LIMIT`、`NOT_ACTIVE_SKILL`、`NO_VALID_TARGET`、`TARGET_NOT_ALLOWED`、`INVALID_CHOICE`、`SEVEN_VOYAGES_ACTIVE`。
- 协议十三种意图：`BuyMinion`、`SellMinion`、`RerollShop`、`UpgradeEra`、`PlayCard`、`RearrangeBoard`、`FreezeShop`、`UnfreezeShop`、`UseHeroPower`、`EndRecruit`、`ChooseDiscover`、`ChooseTarget`、`AbandonTarget`。`UnfreezeShop` 与 `FreezeShop` 对称：指定槽位解冻该槽位，`None` 解冻全部非空槽位，不重抽、不改实例编号、不消耗随机流。`UnfreezeShop` 要求本回合冻结过商店（`PlayerState::froze_this_turn`，回合开始清零），否则返回 `INVALID_STAGE`；冻结标记本身跨回合保留，只由解冻或重抽（手动刷新、升时代）清除。`ChooseDiscover`、`ChooseTarget` 与 `AbandonTarget` 都用于消费引擎产生的 `PendingChoice`（发现/进化、目标）；`PendingChoice` 本身是引擎内部状态，不直接出现在协议里。`AbandonTarget` 只消费本座位队首的目标待选，撤销本次出牌并把原牌退回原手牌位置；没有待选或队首不是目标待选时返回 `INVALID_CHOICE`。
- `PlayCard { hand_index, position, target }`：`position` 只对随从牌生效，是入场后的插入下标，`None` 表示追加到最右；超出当前随从数返回 `INVALID_CHOICE`。效果牌不带落点：目标缺失或不在合法池内时整条意图返回 `NO_VALID_TARGET`，合法池为空时效果落空、意图仍生效。效果牌与主动技能的 `target` 是 `TargetRef { zone, id }`，`zone` 取 `hand` / `shop` / `board`；提交时先校验实例是否属于本人在该区域，再进入合法池校验，不符返回 `NO_VALID_TARGET`。随从牌入场后判定战吼：未带 `target` 且服务端算出候选非空时只推入 `PendingChoice::Target`，由 `ChooseTarget { target }` 指定目标，本次意图内不发金色「三连奖励」、不执行 `check_triple`、不触发入场观察者；候选为空（含被过滤器收窄为空）时战吼直接落空、不产生待选，这些入场副作用照基线顺序立即执行。
- 阶段不匹配时，`recruit::apply` 对全部招募意图（含 `UpgradeEra`、`RerollShop` 等）统一返回 `INVALID_STAGE`；`EndRecruit` 另有「本座位已结束招募」时仍处于 `Recruit`、返回成功且无副作用。
- 实例编号 `InstanceId` 由 `state.next_instance_id` 单调分配：商店每次刷新的每个槽位（含效果牌槽）各有一个，冻结保留原编号与冻结标记直到解冻或重抽；买入、入手牌、打出到棋盘都沿用该编号。三连合成时新建一个金色编号，`TripleFormed` 事件携带被合并的三张原始副本编号（实例本身不保存来源记录）。复制品按复制方式分配新编号。
- `BuyMinion { slot }` 不带目标：买入一律进入手牌，不触战吼；手牌保留随从实例（含成长等状态），不再退回卡面基础值。`PlayCard` 与 `UseHeroPower` 带 `target: Option<TargetRef>`，M0 由 AI 按区域从本人棋盘、手牌、商店依次选择目标。
- 三连判定见核心规则 §6：合成后金色进入手牌（手牌满时留在棋盘最早副本的位置且不发奖励）；金色成功打出入场时发放一张 `triple-reward` 效果牌，牌上记录**发放时玩家当前时代 +1**（封顶 6），此后升时代不追溯。打出该牌由 `GrantMinion { level: next, to: hand }` 直接随机发放一张命中本局启用玩法、`in_shop_pool: true`、星级等于该记录的池内随从到手牌；不登记发现待选、不扣共享牌池，手牌满或无候选时奖励牌照常消耗。奖励效果牌自身 `in_shop_pool: false`，不进商店、不进任何候选、不扣共享牌池；普通效果牌按商店概率进入商店槽位，但同样不扣共享牌池。
- 金色效果按「单次结算 + 数值产出 ×2」执行：`Effect::doubled_outputs()` 只翻倍 `Growth.times` / `times_if.times`、`ModifyStats` 的攻防与 `per_*`、`GrantKeyword` 层数、`DealDamage.amount`、`Summon` / `AddShopSlot` / `Salvage` 数量、`Revive` 攻防、`Detonate.amount`、`Convert.count`、`Latent.amount` / `count`；`condition`、`threshold`、`max_count`、`when`、`Devour.value`、`Discover.choose`、`SevenVoyages.turns_max` 保持原值。光环由 `accumulate(..., golden)` 显式 ×2，战斗传染每次叠 2 层，`Decompose` 在 `apply_combat_decompose` 翻倍获得量。
- `PendingChoice::Target { seat, source, hand_card, hand_index, fire_ally_evolve }`：`hand_card` 是入场前的原手牌（含成长、护盾与金色标记），`hand_index` 是原手牌位置，放弃时按 `min(hand_index, len)` 插回；`fire_ally_evolve` 表示入场观察者是否推迟到目标结算后触发。目标待选期间该座位除 `ChooseTarget` 与 `AbandonTarget` 外一切招募意图（含 `EndRecruit`）返回 `INVALID_STAGE`。`ChooseTarget` 要求队首是目标待选、目标属于本人在对应区域的实体，且通过该牌 `OnPlay` 效果的合法池校验，否则返回 `INVALID_CHOICE` / `NO_VALID_TARGET`；消费后立即按 `OnPlay` 结算该牌效果，再按固定顺序补做被推迟的入场副作用：金色三连奖励 → `check_triple` → `fire_ally_evolve` 为真时触发入场观察者。`AbandonTarget` 要求来源随从仍在本人棋盘（否则 `NO_SUCH_MINION`），移除来源、把原牌插回原手牌位置、移除待选，不发战吼、不补奖励、不执行三连与入场观察者。发现与进化待选允许留到下回合；目标待选若在 `finish_recruit` 进入战斗前仍未选择，按同样规则退牌（来源仍在场时），随后照常推进阶段。
- `HandCardView.requires_target` 由效果选择器决定：`Selector::Target` 或 `friendly:single`（`Side` + `Pick::Single`）即为需要目标，随从牌恒为 `false`。视图同时下发 `targetCandidates`（引擎 `target_candidates` 的结果，只含本人棋盘/手牌/商店）、`instanceId`、`golden`，以及随从牌的 `attack` / `health`（当前值，含成长等全部层数）；效果牌这两项为 `null`，`golden` 恒为 `false`，奖励牌另带 `rewardLevel`（锁定的发放星级）。`requires_target` 为真且候选为空时 `playable=false`、`disabledReason=NO_VALID_TARGET`。
- `SelfView.hero_powers` 按英雄卡 `skills` 顺序列出全部主动技能，每项带 `skill_id`、`uses_left`、`requires_target`、`target_candidates`、`can_use`、`disabled_reason`，目标口径与手牌一致；需要目标且候选为空时该技能 `can_use=false`、`disabled_reason=NO_VALID_TARGET`，不影响同一英雄的其他技能。`PendingChoiceView::Target` 同时下发 `candidates`。
- `UseHeroPower { skill_id, target }` 只校验并结算 `skill_id` 自身的 `OnActivate` 效果；同一英雄的其它主动技能不随本次意图结算，也不扣金币与次数。含 `SevenVoyages` 的技能在费用与次数校验后先检查是否已有进行中的航程（有则返回 `SEVEN_VOYAGES_ACTIVE`），通过后先扣金币与次数，再用 `Stream::Effects` 掷出航程回合数。
- `EndRecruit` 只结束本座位；全员（未出局）结束后才进入 `Combat` 并发出 `PhaseChanged`，重复 `EndRecruit` 成功且无副作用。
- `settle::finish_recruit` 幂等：`Recruit` 阶段（超时路径）生成配对并切 `Combat`；`Combat` 阶段只补齐一次配对，不重复发事件、不重复配对。
- 阶段推进由驱动方按固定顺序调用 `finish_recruit` / `resolve_combat` / `resolve_settle` / `start_turn`，引擎不自行推进时间。
- `rule-engine` 顶部 `#![forbid(unsafe_code)]`；结算只用整数，容器用 `BTreeMap` 与有序 `Vec`。
- M1a 起引擎提供只读逐座位视图 `rule_engine::view::view(&Engine, &GameState, Seat) -> Option<SeatView>`；`SeatView` 是引擎自有类型，不依赖协议 crate，由 `room-service::view::to_protocol` 映射为协议 `PlayerView`。

## 3. 卡牌分类、玩法与卡池

### 3.1 数据类型

`server/crates/rule-engine/src/data.rs`：

```rust
pub enum Playstyle {
    Mammal, Bird, Reptile, Fish, Insect, Cephalopod, Cnidarian,
    Plant, Fungus, Bacteria, Protozoa, Virus, Machine, Neutral,
}

pub enum CategoryKind { Play, Prey, Host }

pub struct Category { pub kind: CategoryKind, pub value: String }

pub struct Card {
    // 既有字段
    pub taxon: String,
    pub categories: Vec<Category>,
    // 缺省 true；false 表示不进商店卡池、共享牌池、三连与发现
    pub in_shop_pool: bool,
}
```

- 数据类型已无部落枚举与卡牌部落字段；卡牌的物种由 `taxon`、玩法归属与捕食/寄生关系由 `categories` 表达，卡牌不再有单值 `playstyle` 字段。
- `Playstyle::ALL` 固定 13 项且不含 `Neutral`；`id()` / `from_id()` 是数据文件与协议中的玩法 id；`ordinal()` 是状态哈希中的序号（0..13，`Neutral` 为 13）。`Neutral` 只服务引擎内部哨兵，不进入卡牌 `categories`，也不参与 `Playstyle::ALL`。
- `CategoryKind` 对应卡牌 `categories` 的三种前缀：`Play`（`play:`）、`Prey`（`prey:`）、`Host`（`host:`）；每条为 `{ kind, value }`。
- 卡牌不再区分独立 Token 实体：衍生物、生成卡（三连奖励）与普通卡共用 `Card` 结构，靠 `in_shop_pool` 区分。`in_shop_pool: false` 的卡不要求 `play:`，其 `taxon`、`era`、`subtype`、`level` 等字段照常参与筛选与展示。
- 成就映射数据在 `server/data/v0.2/achievements.json`，由 `scripts/gen_achievement_data.py` 从 v0.2 成就实体文档生成；规则引擎只使用其中的 `id` 与 `steam_api_name`，条件文本不参与解析。

### 3.2 分类树

`server/crates/rule-engine/src/taxonomy.rs`：

```rust
pub struct Taxonomy {
    nodes: BTreeMap<String, TaxonNode>,
    ids: HashMap<String, u16>,
    parents: Vec<Option<u16>>,
    names: Vec<String>,
}
pub struct TaxonNode { pub parent: Option<String>, pub rank: Rank }

pub enum Rank { Kingdom, Phylum, Class, Order, Family, Technique, Discipline }

impl Taxonomy {
    pub fn contains(&self, id: &str) -> bool;
    pub fn is_leaf(&self, id: &str) -> bool;
    pub fn node_id(&self, id: &str) -> Option<u16>;
    pub fn ancestor_ids(&self, node: u16) -> Vec<u16>;
    pub fn is_within(&self, leaf: &str, ancestor: &str) -> bool;
    pub fn is_within_id(&self, leaf: u16, ancestor: u16) -> bool;
    pub fn path(&self, leaf: &str) -> Vec<&str>;
}
```

- `taxonomy.json` 结构为 `{ "nodes": [{id, parent, rank}], "playstyles": [玩法 id] }`；节点按 id 去重，重复 id 解析失败。
- `Rank` 对应 `taxonomy.md` 的「界 / 门 / 纲 / 目 / 科 / 技术分类 / 学科」，只表达结构，不参与匹配。
- `is_within` 先把两侧 id 解析为整数下标，再沿 `parents` 的整数父链上溯，最多 5 层；公开签名不变，任一节点不存在返回 `false`。`names` 是下标到 id 的反向映射，与 `parents` 同序，仅用于构建期生成 `ids` / `parents`；当前运行期不读取（字段带 `#[allow(dead_code)]`，为后续 `path` 整数化保留）。
- `path` 从叶子返回至根的 id 列表（含自身），未知节点返回空列表。
- `Catalog` 持有 `cards`、`heroes`、`skills: Vec<Skill>`、`skill_ids: HashMap<String, SkillId>` 与 `taxonomy: Taxonomy`；技能按 id 升序编号为 `SkillId`，卡牌与英雄的 `skills` 保存整数标识，触发与效果收集按下标取技能，协议、视图与 `hero_uses` 仍以字符串技能 id 为标识。英雄卡按 `id` 独立存储，与同名随从卡分属两个池。`Catalog` 另持私有侧表 `card_keywords: BTreeMap<String, KeywordSet>`：加载期按每张卡 `skills` 的 `keyword` 预计算卡面自带关键词；`base_keywords(card_id)` 命中侧表时直接克隆预计算值，侧表缺失（测试直接向 `cards` 插卡）时按当前数据即时计算回退，两者口径一致。加载期校验所有卡牌的 `taxon` 节点存在且为叶子，`prey:` / `host:` 节点存在且只用于生物卡；`in_shop_pool: true` 的卡至少 1 条 `play:` 且取值是 13 个可启用玩法之一（不含 `neutral`），非池卡允许没有 `play:`；`CardType` 只有 `minion` / `effect`，`cards.json` 写入 `hero` 在反序列化阶段即被拒绝；英雄卡不含 `card_type`，与随从卡靠独立文件、独立结构体与独立集合区分；每张英雄卡必须有同 id 的随从卡且 `categories` 完全一致，`skills` 各自维护、不要求相同；校验失败即拒绝加载。
- `Catalog` 的分类侧表 `card_filter_facts` 在加载期把每张卡的 `taxon`、`play:` 归属与捕食/寄生分类节点压成整数下标与位掩码（`CardFilterFacts`）；筛选按整数比较，测试直接插卡时回退即时计算。
- 卡牌标识在状态、事件与协议中保持字符串：`MinionInstance.card_id`、`ShopSlot.card_id`、`HandCard` 与 `GameEvent` 的卡牌字段都是稳定 id，整数化只发生在目录内部（技能表与分类侧表）。
- 按用途选池：英雄技能查询（视图、光环、战斗开始、首次刷新、回合开始与主动技能）只读 `catalog.heroes`；商店、卡池、三连、发现与棋盘随从只读 `catalog.cards`。两池同名同 id，任何路径不得跨池回落。

### 3.3 筛选与匹配

`Filter` 的键与匹配口径：

| 键 | 匹配 |
| --- | --- |
| `taxon` | 卡牌 `taxon` 等于该节点或位于其下（`Taxonomy::is_within`） |
| `playstyle` | 卡牌任一条 `play:` 精确相等（按 id 比较） |
| `play` | `categories` 中存在 `play:<值>`，任一条命中即可 |
| `prey` | `categories` 中存在 `prey:<节点>`，且该节点等于筛选节点或位于其下 |
| `host` | 同 `prey`，匹配 `host:` |
| `subtype` / `id` / `type` | 保持既有语义 |
| `keyword` / `other` | 由调用方单独处理 |

- 筛选键集合如上表，不含部落类键；`Filter.key` / `TimesIf.key` 是 `FilterKey` 枚举，未知键在解析期即报错。
- 筛选事实由 `CardFacts` 提供：只查 `catalog.cards`，`playstyle` / `prey` / `host` / `taxon` 读 `CardFilterFacts` 的位掩码与整数下标；查不到时不匹配。
- `timesIf` 结构为 `{ key: FilterKey, value: String, times }`，写作 `<键>:<值>:<次数>`（如 `timesIf=playstyle:mammal:2`、`timesIf=taxon:hominidae:2`），取值必填。
- `condition` 与 `when` 复用同一套筛选键与匹配口径；选择器多个筛选并列表示同时满足。

### 3.4 卡池与候选

`data.rs::card_in_pool(card, enabled)` 是统一入口：

- 卡牌 `categories` 中任一条 `play:` 命中启用玩法即入池；卡牌没有 `play:` 时在加载期被拒绝，不会走到这里。工具卡与效果牌同一口径——效果牌只在本局启用其任一条 `play:` 归属玩法时进商店。
- 商店、发现候选、随机召唤、商店类效果（刷新、加槽、获得随从）与英雄候选过滤都走该入口。
- 复制已有实例、显式 `Summon {card}`、复活是技能明确指定的结果，不走卡池判定。
- 英雄卡不进商店；英雄候选由房间服务按同一入口过滤（见[房间服务与联网](room-service/README.md)）。

**共享池占用口径**：`GameState.pool` 是「当前未被任何商店占用的份数」。`roll_player` 先归还未保留的旧槽位，再由 `draw_shop_card` 在选中后扣 1；`AddShopSlot` 同样即抽即扣。商店槽位只在购买/效果取走/刷新/出局时释放，因此被商店占用的份天然不会进入其他玩家的商店、发现候选、`GainMinion`、`Summon {card}` 与战斗 `Convert`/`Summon`。`MinionInstance.pool_copies: u8` 记录该实例实际从池中扣过的份数（普通 0/1、金色 0–3），出售时按它归还；`sell`、三连合成与 `state_hash` 都读写该字段。

### 3.5 开局玩法

- `GameState.enabled_playstyles: BTreeSet<Playstyle>`，长度固定 5，永不包含 `Neutral`。
- `playstyles::draw(seed) -> [Playstyle; 5]`：独立 splitmix64 随机源，做 Fisher–Yates 无放回抽 5 个，按枚举顺序排序；不读取也不消费引擎的 `RngStreams`。常量 `PLAYSTYLES_PER_GAME = 5`。
- `Engine::new(catalog, seed, playstyles)` 接收调用方抽取好的 5 个玩法；`start_game` 不再抽取。传入含 `Neutral` 或重复玩法时 panic，由调用方保证合法。
- 状态哈希按 `enabled_playstyles` 的 `ordinal()` 写入 5 个序号；同 seed 的抽取结果与顺序固定，重放一致。

## 4. 卡牌数据生成

- 唯一数据源：`docs/current/product/content/` 下的卡牌、技能实体文档与 `taxonomy.md`；`scripts/gen_card_data.py` 复用 `validate_docs.py` 的 front matter 与分类树解析。
- 分类树与玩法清单由 `taxonomy.md` §1 / §2 解析为 `taxonomy.json`（`{nodes, playstyles}`）；客户端生成物 `client/generated/taxonomy.json` 同源。展示名不进 JSON，由文本表 `enum.taxon.<id>` 与 `enum.playstyle.<id>` 提供。
- 生成数据只含规则与标识，不含展示文本：`cards.json`、`skills.json` 没有 `name`、`text` 字段。卡牌 `card_type` / `categories` / `taxon` / `era` / `subtype`、`in_shop_pool`，技能与效果中的 `keyword`、DSL 筛选与枚举（`playstyle`、`taxon`、`play`、`prey`、`host`、`type`、`keyword`、进化选项等）一律为英文 id（如 `minion`、`mammal`、`felidae`、`industrial`、`ignore_taunt`）。展示文本由 `scripts/gen_text.py` 汇总到 `client/text/zh_CN.csv`，按 `card.<id>.name`、`skill.<id>.text` 等文本 id 取用；非池卡同样使用 `card.<id>.name`。
- 经济与对局配置由生成器从产品数值文档口径抄录到 `config.json`（初始生命、棋盘/手牌/商店槽位、回合上限、伤害上下限、升时代费用、星级牌份数、商店概率、金币曲线）。
- 输出确定性 JSON（UTF-8、两空格缩进、键排序、末尾换行）；生成物随源提交。
- 引擎用 `include_str!` 嵌入 `server/data/v0.1/*.json`，加载期由 serde 校验结构化字段与枚举，再由 `validate` 校验跨字段组合；未知字段或非法组合直接失败。源文档 `effects` 仍写 DSL，由生成器编译为结构化数组，生成期用与 `validate` 同规则的 `check_structured` 预检，避免 `make data` 通过而引擎加载失败。
- 效果参数按 `op` 类型化：`skills.json` 的 `params` 仍是扁平对象，自定义 `Effect` 反序列化按 `op` 把对象解码为 `Params` 的对应变体（如 `Params::Decompose(DecomposeParams)`），不属于该原语的字段与缺失的必填字段都在加载期被拒绝；跨字段规则（光环可用原语、`token`/`card` 二选一、`level` 取值等）仍由 `validate` 校验。执行层按变体解构，不再共享可空参数包。
- `make data-check` 重新生成到临时文件并比对（含文本表漂移检测），同时运行 DSL 审计。

## 5. 命令

| 命令 | 作用 |
| --- | --- |
| `make test` | Python unittest 与 `cargo test --workspace` |
| `make lint` | `cargo fmt --check` 与 `cargo clippy --workspace --all-targets -- -D warnings` |
| `make audit` | effects DSL 审计 |
| `make data` / `make data-check` | 生成卡牌数据 / 比对漂移并审计；四条生成或七项检查由 `scripts/run_parallel.py` 并行执行（`JOBS` 控制并行度，生成物确定性不变） |
| `make protocol` / `make protocol-check` | 生成协议 GDScript 常量与字段表 / 漂移检测 |
| `make sim-determinism` | 运行 `sim_determinism::replay_same_log_100_times_has_same_hash`：同一局用 `sim replay` 重放 100 次，终局状态哈希与事件序列哈希全部一致 |
| `make sim-batch` | 1000 局 8 AI 批量对局（seed 1），写入 `output/sim-batch.json` |
| `make check-core` | `data-check`、`test`、`lint`、`sim-determinism` 四项并行执行（基础验收门禁），任一失败整体非 0 |
| `make check` | 全量验收门禁：`protocol-rs-check`、`protocol-check`、`data-check`、`test`、`lint`、`sim-determinism` 六项并行执行；e2e 由 `make test` 覆盖，详见[房间服务与联网](room-service/README.md) |
| `make sim-report` | 先校验批量日志（`sim check --expect-games 1000`：局数、每局 8 名排名），再生成基线报告（字段口径见第 7 节）；输出同时写入 `output/sim-report.log`。可用 `SIM_BATCH_LOG`、`SIM_GAMES` 指向其他批量日志与局数 |
| `make load-build` | release 构建 `room-service` 与 `sim`，供压测使用（debug 结果与线上不可比） |
| `make loadtest` | 并发容量稳态压测：本机起 `room-service`，维持 `LOAD_ROOMS`（默认 10）个活跃房间并补位 `LOAD_DURATION_MS`（默认 30000），报告写 `output/loadtest-steady.json` |
| `make loadtest-ramp` | 爬坡找上限：从 `LOAD_ROOMS` 到 `LOAD_ROOMS_MAX`（默认 100）按 `LOAD_STEP`（默认 10）逐档压测，每档 `LOAD_STEP_MS`（默认 20000），报告写 `output/loadtest-ramp.json` |
| `make loadtest-smoke` | 压测冒烟：2 局稳态 5 秒，报告写 `output/loadtest-smoke.json`，任一健康档存在即退出码 0 |
| `make dev-bots` | 本地联调：起服务端与 7 个机器人，打印房间码后等开发者作为第 8 人入座；一局结束后自动建下一局（房间码每局递增，机器人意图种子每局 +1），`Ctrl-C` 退出，`DEV_ROUNDS=<正整数>` 限定局数，`DEV_INTENT_SEED` 指定起始意图种子。`DEV_PORT` 默认 8080，端口被占用时直接失败 |
| `python3 scripts/validate_docs.py` | 文档与实体数据校验 |

模拟器直接调用：`sim play --seed N`、`sim batch --games N --seed N --out PATH`、`sim replay --log PATH`、`sim report --log PATH`、`sim check --log PATH [--expect-games N]`、`sim bot --connect ws://HOST:PORT/ws --name NAME --hero CARD_ID [--room CODE] [--create] [--intent-seed N]`、`sim dev --connect ws://HOST:PORT/ws [--intent-seed N]`、`sim load --serve-room-service PATH | --connect ws://HOST:PORT/ws --scenario steady|ramp --rooms N [其余压测参数]`；参数错误、模拟失败或日志校验不通过以非零码退出。`sim bot` 的细节见[房间服务与联网](room-service/README.md)第 12 节。

`sim load` 复用 `sim bot` 的机器人协议：steady 维持指定房间数并在一局结束后补位，ramp 逐档递增并在首个不健康档停止。每档结束对全部机器人发停止信号，机器人发送 `LeaveRoom`；对局中按下弃权结算，存活数降到 1 时服务端进入 `GameOver` 并立即销毁房间，不依赖断线宽限。默认 `--serve-room-service` 自动起停本机服务端并每秒采样 CPU/RSS；`--connect` 压远端地址，可用 `--ssh-host` + `--ssh-pid` 读 `/proc` 采样。健康判据为启动失败率（默认 ≤10‰）、开局时延 p95（默认 ≤10s）与 server+generator CPU 是否接近机器核数；同机压测在报告中标记 `coHosted` 并记录生成器开销。远端 `wss://` 尚未支持，压远端需先经端口转发转成 `ws://`。

单局日志除 `final_hash` 外还记录 `event_hash`：`play_game` 把 `start_game` 与每次引擎调用返回的事件按顺序送入 `rule-engine::hash::EventHasher`；`replay` 按同一顺序累计后逐项比对事件序列哈希、终局状态哈希与排名，任一不一致即返回错误。日志缺少 `event_hash` 时 `report::game_from_json` 拒绝解析。

`sim dev` 起 7 个机器人：第 0 个建房并把房间码打印为 `ROOM_CODE=<code>`，其余 6 个在取到码后并发加入，给开发者留出恰好一个座位。英雄卡按 id 升序取自内嵌卡表，第 `index` 个机器人的意图种子为 `intent_seed + index`。取房间码超时 10 秒；取到码后等待开局与整局结束，不设总时限——本地联调时真人每回合可长达 120 秒，固定总时限会中断对局。任一机器人失败时取消其余机器人并以非零码退出。

`sim bot` 只在视图变化时决策；服务端拒绝意图不会带来新视图，因此收到非目标类 `Error` 后每回合最多补发一次 `EndRecruit`（`brain::recover_after_error`），避免整局停在招募阶段等超时。`NO_VALID_TARGET` 走另一条路径：记入本回合阻塞集合后按同一视图立即重决策。

`sim bot` 的意图表同步扩到十三种，不主动发送 `AbandonTarget` 与 `UnfreezeShop`：`pendingChoices` 头部为 `target` 时发 `ChooseTarget { target }`，取 `candidates` 中第一个未被拒绝的 `TargetRef`；`PlayCard` 一律带 `position: None`（追加到最右）；需要目标的随从牌按两段式先不带 `target`，等目标待选后再发 `ChooseTarget`；效果牌读手牌视图 `requiresTarget`，为真时从 `targetCandidates` 预填第一个未被拒绝的目标，候选为空时不填（效果可落空）。被拒目标按 `zone:id` 记入阻塞集合，沿候选列表换下一个。`sim/src/ai.rs`（M0 基线）按 `Selector::Target { zone }` 依次尝试本人三个区域的实例编号。

M0 确定性 AI（`sim/src/ai.rs`）针对效果牌补充三条规则：

- **购买**：`best_affordable` 跳过「效果牌且其全部 `play:` 归属都不等于当前阵容主导玩法」的槽位。主导玩法取棋盘随从全部 `play:` 归属的众数，多条归属各计一次，同票取枚举序靠前者；没有主导玩法时所有效果牌都可买。
- **打出**：`choose_play_effect` 先尝试不带目标地打出效果牌（合法池为空时效果落空、意图仍生效）；该意图被引擎以 `NO_VALID_TARGET` 拒绝后，同一回合改为按本人棋盘、手牌、商店依次带目标重试。
- **重规划预算**：`sim/src/lib.rs` 的驱动循环按座位、按回合统计连续被拒意图数，上限 `MAX_REPLANS_PER_SEAT_TURN = 64`；任一次成功执行都会清空 `blocked` 并把计数归零，预算只用于识别「状态不变的连续被拒」死循环，不跨成功动作累计。

## 6. 验证方式

验证矩阵：`make data-check && python3 scripts/validate_docs.py && make check-core && make sim-batch && make sim-report`（其中 `make check-core` 并行执行 `data-check`、`test`、`lint` 与 `sim-determinism`）。

每个 M0 标准类别由下列命令与字段支撑；表中「报告字段」指 `make sim-report` 输出的 JSON 字段。

| M0 标准 | 命令 | 证据 |
| --- | --- | --- |
| 卡池完整 | `make data-check`、`python3 scripts/validate_docs.py`、`make test` | 命令退出码 0；`catalog::loads_all_committed_data` 断言 432 卡（其中 24 张非池卡）/ 259 技能 / 32 英雄卡全部加载，`every_card_skill_exists`、`unknown_field_fails`、`catalog::committed_data_is_structured`；`audit_effects.py` 输出 `审计通过` |
| 数据一致 | `make data-check` | 重新生成后输出 `生成数据无漂移`，退出码 0 |
| 规则覆盖 | `make test` | `server/crates/rule-engine/tests/` 与 `server/crates/sim/tests/` 的用例全部 `test result: ok`（用例清单见第 8 节）；商店池耗尽留空槽由 `recruit::reroll_with_exhausted_pool_leaves_empty_slots_and_buy_rejects` 锁定 |
| 确定性 | `make sim-determinism` | `replay_same_log_100_times_has_same_hash` 通过：100 次重放每次退出码 0，`final_hash` 全部等于原局，事件序列哈希同样一致；重放由日志种子重新推导同一组 5 个玩法 |
| 分类与玩法 | `make data-check`、`python3 scripts/validate_docs.py`、`make test` | 分类树节点、父链、叶子层级与卡牌 `taxon` 全覆盖；`playstyles::draw` 同 seed 固定 5 个玩法；`Engine::new` 拒绝 `Neutral` 与重复玩法；`card_in_pool` 与筛选键由引擎测试覆盖 |
| 稳定性 | `make sim-batch`、`make sim-report` | `make sim-batch` 退出码 0，无错误输出；产物 `output/sim-batch.json` 的 `games` 数为 1000（每局一条日志）。`make sim-report` 以 `sim check --expect-games 1000` 断言批量日志 `games` = 1000（批量退出码 0、异常/超时 0 的「无缺局」口径），局数不符即非零退出 |
| 可判定 | `make sim-batch`、`make sim-report` | `sim check`（由 `make sim-report` 调用）逐局断言 `ranking.len() == 8`、座位恰为 0–7、名次 1–8 连续，任一局不满足即非零退出并给出异常局数；`sim report` 也内置同一校验，stdout 仅输出 JSON。契约另由 `sim_cli::single_game_produces_ranking_and_hash`、`batch_of_ten_all_ranked`、`check_fails_on_incomplete_ranking` 固化 |
| 数值基线 | `make sim-report`（`output/sim-report.log`） | 退出码 0；`wins_by_playstyle`、`win_rate_permille_by_playstyle`、`avg_turns`、`era_upgrade_turns_by_seat`、`avg_era_upgrade_turn`、`triples_per_game`、`games_reaching_turn_20`、`survivors_at_turn_20`、`avg_damage_per_loss` 均有值；越界与口径问题登记[平衡问题](../product/content/balance-questions.md) |

## 7. 报告字段口径

`make sim-report` 的 JSON：

| 字段 | 口径 |
| --- | --- |
| `games` / `seed` | 局数；批量起始种子（第 i 局种子为 `seed + i`） |
| `wins_by_playstyle` | 各玩法夺冠局数，按玩法 id 升序，只列出夺冠过的玩法；冠军玩法按冠军最终阵容随从的全部 `play:` 条目统计，多条归属各计一次，没有可统计归属时为 `neutral` |
| `win_rate_permille_by_playstyle` | 各玩法夺冠率整数千分比 = 夺冠局数 × 1000 / `games`，向下取整；13 个玩法与中立全部列出 |
| `avg_turns` | 各局结束回合之和 / 局数，向下取整 |
| `era_upgrade_turns_by_seat` | 8 个座位各自「首次升时代回合」的累加值（未升级的座位计 0） |
| `avg_era_upgrade_turn_by_seat` | 上项 / `games`，向下取整；未升级按 0 计入 |
| `avg_era_upgrade_turn` | 已记录首次升时代回合之和 / 记录数（`era_upgrade_count`），向下取整；未升级的座位不计入；无记录为 `null` |
| `era_upgrade_count` | 有首次升时代记录的（座位, 对局）数 |
| `triples_per_game` | 三连总数 / 局数，向下取整 |
| `games_reaching_turn_20` | `survivors_at_turn_20` 非空的局数 |
| `survivors_at_turn_20` | 上述对局在第 20 回合招募结束、`finish_recruit` 与战斗结算之前的平均存活人数，向下取整；无此类局为 `null` |
| `games_ended_on_turn_20` | 结束回合为 20 的局数 |
| `avg_survivors_at_end` | 各局结束时的存活人数之和 / 局数，向下取整。「结束时存活人数」在 `GameOver` 时刻采样（驱动循环退出后取一次）。第 20 回合收束时 `resolve_settle` 只做排序与收束，不淘汰剩余玩家（淘汰只发生在生命归零时），未被淘汰的玩家都会被计入，因此该值不保证为 0；seed 1 的 1000 局实测分布为 1 人 62 局、2 人 333 局、3 人 468 局、4 人 124 局、5 人 13 局，平均 2.693（向下取整报告为 2） |
| `avg_damage_per_loss` | 每次战斗失败的伤害之和 / 败者次数，向下取整 |
| `final_hashes` | 每局最终状态哈希（16 位小写十六进制，按局序） |

`sim play` 的单局日志 `stats` 含 `era_upgrade_turns`（8 个座位各自的首次升时代回合，未升为 `null`）、`survivors_at_turn_20`（第 20 回合招募结束、`finish_recruit` 与战斗结算之前）、`survivors_at_end`（`GameOver` 时刻）、`damage_per_loss`、`winner_seat`、`winner_playstyle`。`winner_playstyle` 取玩法 id（`neutral` 为合法值），缺失或非法时 `sim check` 拒绝该日志。

模拟器与确定性：

- `sim::assign_heroes` 只保留 `hero_in_pool` 通过的英雄卡（遍历 `Catalog.heroes`），按 id 升序取 8 张依次分配给座位 0–7；不足 8 张时从头循环取，没有任何可用英雄时报错。
- `play_game` 与 `replay` 都由日志种子调用 `playstyles::draw(seed)` 得到本局 5 个玩法，再传给 `Engine::new`；因此重放与批量运行使用同一组玩法。
- 报表字段为 `wins_by_playstyle` / `win_rate_permille_by_playstyle`，单局日志字段为 `winner_playstyle`。

## 8. 规则用例清单

`make test` 运行的测试文件即规则用例清单：

| 规则章节 | 用例文件 |
| --- | --- |
| 核心规则 §2 阶段状态机、§7 淘汰排名、§8 边界 | `rule-engine/tests/settle.rs`、`recruit.rs` |
| 核心规则 §3 十三种意图与十三个失败码 | `rule-engine/tests/recruit.rs`、`choices.rs` |
| 核心规则 §4 招募细则 | `rule-engine/tests/recruit.rs`、`choices.rs`；商店重试后仍空即空槽，此时 `BuyMinion` 返回 `SHOP_SLOT_EMPTY`（`reroll_with_empty_star_retries_then_leaves_slot_empty`、`reroll_with_exhausted_pool_leaves_empty_slots_and_buy_rejects`） |
| 核心规则 §5 战斗与伤害公式 | `rule-engine/tests/combat.rs` |
| 核心规则 §5.6 七下西洋 | `rule-engine/src/settle.rs`、`src/recruit.rs`（内联用例）、`tests/catalog.rs`、`effects_exec.rs`、`combat.rs` |
| 核心规则 §6 三连与发现 | `rule-engine/tests/triple.rs`、`choices.rs` |
| 关键词机制 §1–§3 | `rule-engine/tests/turn_effects.rs`、`death_effects.rs`、`effects_exec.rs`、`auras.rs`、`combat.rs` |
| 关键词机制 §9、§10 | `rule-engine/tests/effects_parse.rs`、`catalog.rs`、`effects_exec.rs` |
| 时代与数值 | `rule-engine/tests/recruit.rs`、`settle.rs`、`turn_effects.rs` |
| 确定性 | `rule-engine/tests/determinism.rs`、`sim/tests/sim_determinism.rs` |
| 卡池加载 | `rule-engine/tests/catalog.rs` |
| 玩法抽取与开局玩法 | `rule-engine/tests/playstyles.rs` |
| 卡池与筛选键 | `rule-engine/tests/pool.rs`、`effects_exec.rs`、`catalog.rs` |
| 击杀归属与分解 | `rule-engine/tests/kills.rs`、`decompose.rs`、`combat.rs` |
| 随从攻防分层 | `rule-engine/tests/stats.rs`、`stats_view.rs`、`combat_damage.rs` |

## 9. 随从属性分层

棋盘随从实例 `MinionInstance`（`server/crates/rule-engine/src/state.rs`）的攻防各用一个 `StatComponent` 记账，合为 `MinionStats { attack, health }`。每一项数值分六层：

| 字段 | 含义 |
| --- | --- |
| `initial` | 初始值：卡面攻防，金色翻倍；生产路径下仅在三连升金时变化 |
| `growth` | 成长来源的永久加成；`RemoveGrowth` 只清这一层 |
| `permanent` | 其他永久加成（`duration=permanent` 的修正）；传染的永久减血也记在这里 |
| `turn` | 本回合/本场临时效果（`duration=combat` 的修正）；进入 Settle 时清除 |
| `aura` | 光环派生值；只存在于战斗与视图的折叠副本，不写回持久实例 |
| `current` | 当前量；战斗判定与视图读取的当前数值 |

- **`current` 是唯一真相，各层只是构成。**「层和等于 `current`」不是不变量：受击与复生会让二者偏离，`damage()` 返回 `层和 - current`。
- 变更一律走方法，分量与 `current` 同步：`add_growth` / `add_permanent` / `add_turn` / `set_aura` / `clear_turn` / `remove_growth` / `set_initial`。`apply_damage` 只扣 `current`，`revive_to` 只写 `current`；除这三者外没有直接写 `current` 的公开入口。
- `StatComponent` 的字段保持 `pub`：集成测试与 `room-service` 需要直接读取各层，视图也要逐项复制。因此「不直接写 `current`」靠约定与评审保证，不是类型约束；新增代码只能经上面列出的方法改动分量。
- 初始值在 `MinionInstance::new` 时解析：按卡牌 id 取 `catalog.cards` 的攻防（池内卡与非池卡同一口径），金色 ×2。
- 读取口 `attack()` / `health()` 返回 `max(0, current)`；各层照常累积。战斗内部死亡判定直接用原始 `stats.health.current <= 0`，不经过非负夹紧。
- 视图 `view::MinionStatsView` / `StatBreakdownView` 把攻防映射为 `initial`、`permanent`（= `growth + permanent`）、`turn`、`aura`、`damage`、`current` 六项；在可达的下发状态下满足 `initial + permanent + turn + aura - damage == current`（复生造成的负 `damage` 只存在于战斗内部，不下发）。`health.current` 在视图中夹到 0，招募期负血量由 `damage` 吸收。`BoardMinion` 协议的 `stats` 与顶层 `attack` / `health` 复用同一口径。

**来源账本与下发：**

- `MinionInstance` 额外维护 `stat_sources: Vec<StatSource>` 展示账本，条目为 `{ layer, origin, attack, health }`。`StatSourceLayer` 为 `Growth` / `Permanent` / `Turn` / `Aura`；`StatSourceOrigin` 为卡牌 `Card { card_id, skill_id }` 或系统 `System(StatSystemSource)`，后者取 `Evolve` / `Breed` / `Contagion` / `Revive` / `Recall` / `CombatGrowth` / `AuraFloor` / `Unknown`。
- 所有写数值的入口在写入分量后调用 `record_stat_source`：按 `(layer, origin)` 合并、攻防分别累加，攻防双 0 时不记录。`remove_growth_sources`、`clear_turn_sources`、`replace_aura_sources` 与 `remove_growth`、`clear_turn`、`set_aura(0)` 成对调用，保证账本跟随分量清理。
- 账本只服务展示，聚合层已完整表达数值语义，`hash.rs` 不编码来源，`state_hash` 不受来源顺序影响；来源在视图层显式排序，重放与确定性不受影响。
- 视图 `MinionStatsView.sources` 由 `view::stat_source_views` 生成：`Growth` 与 `Permanent` 合并为 `Permanent` 层，同 `(kind, card_id, skill_id)` 再求和；攻防双 0 的来源不输出，某层聚合值为 0 时整层来源都不输出；排序为层序 `Aura → Permanent → Turn`，层内按 `|attack| + |health|` 降序，再按 `card_id`、`skill_id`、`kind` 升序。`kind` 升序取 `StatSourceKindView` 的枚举声明序（`Card` → `Evolve` → `Breed` → `Contagion` → `Revive` → `Recall` → `CombatGrowth` → `AuraFloor` → `Unknown`），不是字符串字典序。`stats_breakdown` 用 `debug_assert_eq!` 断言每层来源和等于对应聚合层。
- 协议 `MinionStats.sources` 为必填数组，元素 `MinionStatSource` 字段为 `layer` / `kind` / `cardId` / `skillId` / `attack` / `health`；`layer` 枚举 `aura` / `permanent` / `turn`，`kind` 枚举 `card` / `evolve` / `breed` / `contagion` / `revive` / `recall` / `combat_growth` / `aura_floor` / `unknown`，`cardId` / `skillId` 为 `string | null`，攻防可为负。`room-service::view::to_protocol` 逐项映射，战斗报告的 `board_minion_from` 复用同一映射。
- 光环来源由 `GameState.aura_sources: BTreeMap<InstanceId, Vec<AuraSource>>` 提供，与 `state.auras` 由 `recompute_auras` / `recompute_auras_for` 成对重算；逐来源按来源卡 id 合并，英雄光环使用英雄卡 id，卡表缺失时为空串。`fold_auras` 把 Aura 层来源整体替换进折叠克隆的账本，持久实例的 `aura` 层与 Aura 来源始终为空；触发「光环失效不致死」夹紧时，差额并入生命增量最负的来源（同值按卡 id 升序），没有负向来源时追加一条 `System(AuraFloor)`，保证来源之和等于聚合层。
- 回手与战斗成长：战斗回手时清除临时层与 Aura 来源，保留成长数值但把成长来源整体替换为一条 `System(Recall)`，不再保留原始成长来源卡；`write_back_persistent` 保留持久棋盘原有的成长来源，只把 `updated.growth - stored.growth` 的差值补记为一条 `System(CombatGrowth)`，战斗克隆里的技能级来源只留在快照、不回写持久棋盘。

## 10. 战斗伤害、复生与快照

- 行动轮次：`SideBoard` 每侧一个指针，双方逐拍交替出手，同一侧按站位从左到右、跳过阵亡与不能攻击的随从。指针越过该侧末尾后，该侧下一次轮到回到最左，并恢复全部存活随从的本轮行动次数（普通 1 次、连击 2 次），循环至一方全灭或 200 次行动上限；两侧轮次各自独立。连击在同一拍连续结算两次，两次独立选目标，第一次击杀目标后第二次重新选目标；攻击者第一次攻击后阵亡则第二次不进行。复生保留本轮剩余行动次数，召唤与复生按插入位置决定本轮是否轮到。
- 无伤害僵局：每拍开始先检查双方是否都没有能造成伤害的随从——存活、`starting_tokens > 0`、`effective_attack > 0`。用 `starting_tokens` 而不是 `cannot_attack` 字段：战斗开始时施加的临时禁攻不改变入场时算出的行动次数，后续轮次会按该次数恢复出手。两侧都没有时置 `CombatLog.stalemate` 并结束模拟，已产生的动作保留；结算层据此判平，不走行动上限后的生命 → 累计伤害 → 座位判据。效果触发都依赖受伤、死亡、召唤、主动攻击或入场，僵局下这些事件不会发生，因此不遍历卡牌效果。
- 选目标：先排除仍在潜行的随从，全部潜行时固定选择最左侧存活随从；攻击者带 `ignore_taunt` 时在全部可选中随从中随机，否则有可选中嘲讽时只在嘲讽中随机，否则在全部可选中随从中随机。
- `BattleField::apply_damage(id, amount, can_kill)`（`server/crates/rule-engine/src/combat.rs`）是唯一的战斗掉血入口，只扣 `stats.health.current`；`deal_damage` 是同义包装。攻击主伤害与反击伤害都走它。`can_kill = false` 时最多把当前生命降到 1，`can_kill = true` 时允许降到 0 或更低，由死亡判定按 `current <= 0` 处理。
- 战斗棋盘与招募棋盘同容量：`BattleField::has_room`（`alive_count(side) < catalog.config.board_slots`，v0.1 为 7）是所有进场路径的唯一闸门——召唤效果（含左邻插入与追加）、`Copy` 到棋盘、死亡结算的播种召唤、复生，容量不足时一律作废：不扣牌池、不生成实例编号、不触发 `OnAllySummon`、不增加 `summon_count`。阵亡随从留在向量里但不计数。
- 同一时点的进场顺序固定为**死亡 → 召唤 → 复生**：死亡先把 `alive` 置 false，存活数立即减一；亡语与播种召唤占用空出的位置；复生最后检查容量，被召唤占满时保持阵亡（`reborn_used` 照常置 true，本场不再尝试）。
- `append` / `insert_at` 返回 `Option<InstanceId>`，容量不足返回 `None`，调用方据此跳过召唤计数与联动触发。写入 `CombatAction::Summon.position` 的是 `alive_rank`——该随从在**存活随从序列**中的下标（已阵亡条目不计数），值域由 `board_slots` 决定；向量下标仍用于维持阵亡锚点的顺序。
- 复生直接调用 `MinionStats::revive_to(1)`，把 `current` 置为 1，允许大于层和；这是 `current` 正向偏离层和的唯一入口，不复用 `apply_damage`。
- `SideBoard` 不再持有独立生命数组，战斗内所有当前生命读写都落在 `stats.health.current`。
- `CombatLog` 除动作序列外记录与动作同序同长的 `states`：每条动作执行后受影响随从的快照 `MinionSnapshot { instance_id, attack, health, alive }`——`attack` 只记录主动攻击这一条，带攻方与受方两个快照，反击伤害已并入快照（不再单独记录反击动作）；`death`/`reborn`/`contagion`/`summon` 各带一个。`CombatLog.stalemate` 标记无伤害僵局；为真时 `winner` 为空，结算层判平。召唤动作 `CombatAction::Summon { minion, card_id, golden, seat, position }` 记录插入位置，`position` 是该随从在存活随从序列中的下标（阵亡条目不计数，上界由 `board_slots` 决定），召唤物不在战前快照中，位置只从该字段读取；它的那条快照带召唤瞬间的实际攻防（含光环折叠与战斗内加成），房间服务用它填展示数值。
- 战后 `write_back_persistent`（`server/crates/rule-engine/src/settle.rs`）只回写 `contagion` 与 `stats.*.growth`；持久棋盘上的随从按层和重算当前值（满血进入下一回合），死亡不改变持久棋盘，战斗伤害、回合效果与光环都不回写。
- 进入结算时 `clear_combat_duration` 调用 `clear_turn` 清除攻防的 `turn` 层，同时保留 `granted` 的时长清理与禁攻规则。
- 幽灵战（`settle.rs::resolve_ghost`）的幽灵来自 `state.ranking`（全部出局者，最近出局在前）：每回合用 `Stream::Matchmaking` 随机抽一名，可能连续抽到同一人；没有出局者时幽灵为空阵容，落单座位仍产生配对与战报。幽灵取其最终棋盘（死亡时的真实数值，保留成长与永久增益），并按幽灵自己的光环来源（`player_auras`）折叠一次；不结算英雄被动、不产生手牌、不写回牌池与持久棋盘。若幽灵与己方棋盘实例编号重号，`renumber_ghost` 只在本次模拟的克隆上改号。双方同回合清场或双方都无伤害来源时判平局；幽灵获胜时落单座位按伤害公式扣血，落单座位获胜时计入 `last_won`/`last_winner`。
- 光环只经 `fold_auras` 折叠进克隆：把 `state.auras` 的派生值写入克隆的 `aura` 层并保留「光环失效不致死」的夹紧，持久实例的 `aura` 层始终为 0。
- `state.auras` 由引擎在固定时机重算：`recompute_auras` 全量扫描全部未出局座位并整体替换，用于开局、回合开始、招募结束（`Engine::finish_recruit`）与结算（`Engine::resolve_settle`）；`recompute_auras_for(seat)` 只重算单个座位，供 `Engine::apply` 在每条意图成功后调用——移除并重建该座位棋盘的条目，同时清理已不在任何未出局玩家棋盘上的陈旧条目，结果与全量重算一致。
- `hash.rs` 编码 `stats` 各分量（含 `current`）；编码变化会改变状态哈希，同代码重放仍保持确定。
- 关键词编码：光环关键词存 `KeywordSet` 位图，`hash.rs` 经 `names()` 输出排序后的原文逐条写入；随从的 `granted` 仍按 `(关键词原文, 时长)` 逐条写入。战斗内部的位图判定与层数累加不改变哈希口径，因此关键词位图化不改变既有日志与黄金基线的哈希。

### 10.1 伤害来源与击杀归属

`combat.rs`：

```rust
pub enum DamageSource {
    Minion(InstanceId),
    None,
}

pub struct KillRecord {
    pub killer: InstanceId,
    pub victim: InstanceId,
    pub killer_side: usize,
    /// 攻击者的主伤害击杀目标、目标的这次反击击杀攻击者；为真时跳过击杀类钩子。
    pub mutual: bool,
}
```

- `BattleField::apply_damage(id, amount, can_kill, source)` 是唯一掉血入口；`deal_damage(id, amount, can_kill)` 是同义包装，固定 `DamageSource::None`。
- 只有 `source = Minion(killer)`、目标本次仍存活且伤害把生命降到 ≤0 时，向 `pending_kills` 追加一条 `KillRecord`；同一段伤害最多记录一次。
- 攻击主伤害归因攻击者，反击伤害归因反击者；效果伤害（`DealDamage`）归因施放该效果的随从；传染、回合开始掉血没有来源，不产生击杀。
- 同一次交换中攻击者的主伤害击杀目标、目标的这次反击击杀攻击者时，本次交换产生的两条记录（攻击者击杀目标、目标反击击杀攻击者）都标 `mutual = true`。互杀判定在反击结算后、`OnDealDamage` 之前完成；`OnDealDamage` 等效果造成的死亡不参与该判定。
- 每次主动攻击都按目标当前攻击力结算反击，目标被主伤害击杀不影响反击伤害；`counter > 0` 才检查并消耗攻击者护盾，目标攻击力为 0 时不造成伤害、不消耗护盾。
- `OnDealDamage` 在本次交换的死亡判定前触发：攻击者在主伤害 >0 时触发；目标在反击伤害 >0 且未被主伤害击杀时触发。`OnKill` / `OnAllyKill` 在死亡队列处理完后触发。
- 死亡队列处理完后，按记录顺序触发：
  1. `OnKill`：只对仍存活的击杀者触发，事件目标为被击杀者；
  2. `OnAllyKill`：被击杀者属于击杀者敌方时，对击杀方全部存活随从（含击杀者）触发，事件目标为被击杀者。
- `mutual = true`、击杀者已离场或生命 ≤0 的记录两条钩子都不触发；互杀后击杀者因复生回到场上也不补发。击杀友方随从只触发 `OnKill`，不触发 `OnAllyKill`。
- `Trigger` 枚举与 `TRIGGER_JSON` 已登记 `OnKill` / `OnAllyKill`，技能侧只声明触发名。

### 10.2 分解（Decompose）

`effects/mod.rs` 与 `effects/exec.rs`：

```rust
pub enum Op { /* 既有 */ Decompose }

pub enum Basis { Original, CombatStart, PreKill }

pub struct DecomposeParams {
    /// 分解取生命值的口径：original / combat_start / pre_kill。
    pub basis: Basis,
    /// 必填：permanent / combat；instant 由校验拒绝。
    pub duration: Duration,
    pub condition: Option<Condition>,
    pub when: Option<Filter>,
}

pub enum Params {
    // 其他原语的变体省略
    Decompose(DecomposeParams),
}
```

- `selector` 指定获得生命值的随从（数据固定写 `self`）；`event_target` 是被分解的随从。
- 口径读取 `BattleField::decompose_health(catalog, victim, basis)`：

| `basis` | 来源 | 实现 |
| --- | --- | --- |
| `original` | 卡面基础生命值（金色 ×2） | `base_stats(catalog, victim.card_id, victim.golden).1` |
| `combat_start` | 进入战斗快照时的当前生命值 | `combat_start_health[victim]` |
| `pre_kill` | 承受致命伤害前的当前生命值 | `pre_kill_health[victim]` |

- `BattleField` 持有 `combat_start_health` / `pre_kill_health` 两张 `BTreeMap<InstanceId, i32>`：构造时写入双方全部随从的当前生命值，召唤物在召唤时写入 `combat_start_health`；`apply_damage` 在本段伤害足以致死时、扣血前写入 `pre_kill_health`。
- `duration` 必填，数据校验只接受 `permanent` / `combat`：`permanent` 走 `health.add_permanent`，`combat` 走 `health.add_turn`；`instant` 由校验拒绝、省略由反序列化拒绝；执行期保守跳过。
- 获得量为 ≤0、目标已离场、`event_target` 缺失或快照缺失时静默跳过，不报错。
- 金色来源的分解获得量 ×2、单次结算：`apply_combat_decompose` 在 `ctx.golden` 时把 `amount` 乘 2；条件与阈值参数不受影响。
- 触发时点由技能数据选择（`OnKill` / `OnFriendlyDeath` / `OnAllyKill`），引擎不限制 `Decompose` 只能配这些触发。
- 复生不撤销本次死亡事件：已经按死亡结算过的分解仍使用死前快照。

### 10.3 七下西洋（SevenVoyages）

- 数据：`skill-templates/activate-seven-voyages.md` 编译为 `on_activate | SevenVoyages | self | { turns_max }`；`turns_max` 取值 1–7、默认 7，校验要求 `OnActivate` 与 `Selector::SelfCard`。
- 激活：`recruit::check_hero_power` 在费用与次数校验后检查「技能含 `Op::SevenVoyages` 且 `player.seven_voyages.is_some()`」，命中返回 `SEVEN_VOYAGES_ACTIVE`；通过后先扣金币与次数，再由 `apply_seven_voyages` 以 `state.rng.below(Stream::Effects, turns_max) + 1` 得到 n，写入 `SevenVoyagesState { remaining: n, pending: 0, total: n }`。激活当回合计为第 1 个航行回合，当回合战斗结算即累计第一笔。
- 结算：`resolve_real` 与 `resolve_ghost` 在战斗改变棋盘前截取本方快照（`board_a` / `board_b` / `own_board`），`board_slots` 取 `catalog.config.board_slots`（v0.1 为 7）；占位数 = 快照长度，空位数 = `board_slots - 占位数`。`voyage_result` 以 `seat_winner` 判定：`None`（双方同回合清场或无伤害僵局）为平，胜者为本座位为胜，否则为负；触行动上限的平局仍按 `seat_winner` 的胜/负口径。胜利或平局取 `max(占位数, 空位数)`，失败取 `min`，累计进 `pending`（`u16` 饱和加）并把 `remaining` 减 1；`remaining == 0` 后不再累计。
- 到账：`Engine::start_turn` 给未出局座位分配当回合正常金币后检查 `seven_voyages`：`remaining == 0` 时把 `pending` 夹到 `u8` 上限并饱和加到 `gold`，随后清除状态，使该技能可再次使用；`remaining > 0` 时原样保留。到账不设上限，也不跨回合保留（未到账前 `pending` 属于航程状态，不计入金币）；航行途中出局或对局结束不会再走 `start_turn`，未到账的累计作废。
- 状态与视图：`PlayerState.seven_voyages: Option<SevenVoyagesState>` 只在激活后存在；`view::HeroPowerView` 仅对含 `Op::SevenVoyages` 的技能下发 `voyage_remaining` 与 `voyage_pending`，协议字段为 `voyageRemaining`（`null` 或 0–7）与 `voyagePending`（≥0）。
- 哈希：`hash_player` 先写存在标记（1/0），存在时再写 `remaining`、`pending`、`total`；航程状态变化会改变状态哈希。
