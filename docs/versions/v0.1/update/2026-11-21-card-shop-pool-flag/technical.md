# 卡牌商店池标记与 Token 合入技术方案

> 状态：方案未实施
> 需求归属：v0.1
> 产品规则：[product.md](product.md)（已确认）

## 1. 改动边界

- 包含：卡牌源文档与索引、数据生成与校验、rule-engine 数据模型与查询、room-service / sim 加载、客户端 CardDB / 卡池 / 图鉴、文本表、测试与当前技术文档。
- 不包含：协议 schema 字段、数据库迁移、部署配置、英雄卡结构、对局规则与数值。
- 合并后卡牌目录是唯一实体集合；`Token` 不再有独立结构体、独立文件与独立 id 命名空间。

## 2. 数据与生成

### 2.1 卡牌源文档

- 新增 23 份 `docs/versions/v0.1/product/content/cards/<id>.md`，id 与名称沿用 `tokens/README.md` 注册表；删除 `content/tokens/`。
- front matter 增加可缺省布尔 `in_shop_pool`，缺省为 `true`；只有非池卡写 `false`。现有 408 张池内卡不批量补字段。
- `triple-reward` 增加 `in_shop_pool: false`，技能从 `trigger-discover-tribe-minion` 改为新的通用技能 `gain-next-level-minion`；其余字段不变。
- 新增通用模板 `on-play-grant-minion` 与技能 `gain-next-level-minion`：`GrantMinion{level=next,to=hand}`，打出后直接随机获得一张下一等级的池内随从，不登记待选、不扣池。旧 `on-play-discover-playstyle` / `trigger-discover-tribe-minion` 保留给其它卡牌，行为不变。
- Token 文档字段：
  - `card_type: minion`、`era: prehistoric`、`is_scholar: false`、`skills` 省略。
  - `level` 取满足该星级攻击上限与生命上限的最小星级；不检查星级下限（1/1 衍生随从允许 1 星）。
  - `subtype` 按玩法映射现有枚举：植物 `crop`、细菌 `bacteria`、哺乳 `mammal`、鱼 `fish`、真菌 `fungus`、原生生物 `protozoa`、水螅 `ocean`、昆虫 `insect`、鸟 `bird`、头足 `mollusk`、中立 `cell`。
  - `categories` 由原玩法列转为 `play:<玩法>`；基础细胞（原 `neutral`）不写 `categories`。
  - 正文含 `## 属性`、`## 效果`（无技能写「无」）、`## 设计说明`、`## 出处`；属性表在 `in_shop_pool: false` 时增加「进商店卡池：否」行。
- `cards/README.md` 增加「非池卡」分区列出 24 张（23 张衍生物 + 三连奖励），更新总数与索引说明；`content/README.md` 删除 Token 注册表入口。
- 文档统计按 432 张重算：332 随从 + 100 效果；时代、星级、玩法、关键词与技能引用统计同步。

### 2.2 数据生成

- `scripts/gen_card_data.py`：
  - `FILES` 删除 `tokens.json`；删除 `parse_tokens` / `parse_token_names` / `_token_rows`。
  - `card_entry` 输出 `in_shop_pool`（解析后的布尔，所有卡都带值）。
  - 生成物删除 `server/data/v0.1/tokens.json`。
- `scripts/gen_client_data.py`：删除 `_display_token` 与 `tokens.json`；卡牌展示条目带 `in_shop_pool`。
- `scripts/gen_text.py`：删除 `token.<id>.name`；Token 名称改由 `card.<id>.name` 承载。
- `scripts/skill_templates.py`：
  - `self.cards` 读取 `in_shop_pool`；`self.tokens` 改为 `cards` 中 `in_shop_pool == false` 的子集。
  - 参数 `token` 必须命中非池卡；参数 `card` 必须命中池内卡。
  - `_render_value` 统一从 `self.cards` 取名称与身材。
- `scripts/gen_card_data.py` 的 `OP_JSON` 与 `scripts/validate_docs.py` 的效果白名单增加 `GrantMinion`；`balance_report.py` 的机制词表不需要新增词条。
- `scripts/balance_report.py`：统计与违规检查只取 `in_shop_pool` 为真的卡，避免 Token 改变玩法身材与密度基线。

### 2.3 校验

- `scripts/validate_docs.py`：
  - `in_shop_pool` 为可选布尔；`false` 时属性表必须有「进商店卡池：否」，`true`/缺省时不得出现该行。
  - 名称唯一性只比较 `in_shop_pool: true` 的卡。
  - 「无技能随从恰好 1 张」只统计池内随从。
  - `play:` 至少 1 条只约束池内卡；非池卡允许 0 条，按中立处理。
  - 非池卡星级数值只检查上限；池内卡维持现有上下限与超模规则。
  - 卡牌总数与时代/星级分布改为 432 / 332 随从口径；关键常量 `EXPECT_ERA`、`EXPECT_LEVEL` 同步。
  - 每张非池卡仍校验 `taxon` 叶子、`出处` 非空与属性表一致性。
- `make data-check` 必须无漂移。

## 3. 服务端

### 3.1 rule-engine 数据模型

- `Card` 增加 `in_shop_pool: bool`，`#[serde(default = "default_true")]`，旧数据缺字段按进池处理。
- 删除 `Token` 结构体、`Token::playstyle()`、`Catalog.tokens` 与 `from_json` 的 `tokens` 参数。
- `Catalog` 只保留 `cards` / `heroes` / `skills` / `config` / `taxonomy`；所有卡牌与 Token 校验统一走 `cards`。
- `validate_taxonomy` 只遍历 `cards`。
- 加载期新增：非池卡可以没有 `play:`；池内卡仍必须有至少 1 条合法 `play:`。

### 3.2 查询与规则

- `state.rs::base_stats`、`combat.rs::base_keyword_occurrences`：只查 `catalog.cards`，删除 Token 回退。
- `effects/exec.rs::card_facts`：只查 `catalog.cards`；删除 `token_playstyle` 与 Token 回退，玩法统一读 `categories`。
- `engine.rs::build_pool`：只对 `card_type == Minion && in_shop_pool` 的卡填共享池。
- `recruit.rs::is_eligible`：先要求 `card.in_shop_pool`；删除 `REWARD_CARD_ID` 特判，`triple-reward` 由字段排除。
- `triple.rs`：
  - `find_triple` 的随从判定增加 `in_shop_pool`，非池随从（衍生物）不参与三连。
  - `is_discover_candidate` 增加 `in_shop_pool`，非池卡不作为发现候选。
- `effects/exec.rs`：
  - `recruit_summon_card` / `combat_summon_instance`：`token` 参数要求命中非池卡且不扣池；`card` 参数要求命中池内卡且扣池。
  - `pool=playstyle` 候选过滤改为 `card.in_shop_pool`，删除 Token 与奖励卡 id 特判；载体玩法只读 `categories`。
- 新增效果 `GrantMinion{level,to}`：
  - `level=next` 取 `EffectCtx.level_override`（奖励牌携带的来源星级 +1，6 星封顶）；缺省回退玩家当前时代。也接受 `current` 与 1–7。
  - `to` 只接受 `hand`。
  - 从 `catalog.cards` 中筛选 `card_type == Minion && in_shop_pool && card.level == star && card_in_pool(card, enabled_playstyles)`，按 id 升序枚举后用 `Stream::Effects` 随机取一张。
  - 不检查、不扣共享池；手牌已满或无候选时不获得。
  - 获得后推入 `CardDiscovered` 事件并执行 `check_triple`，与现有发现入手牌口径一致。
- `EffectCtx.discover_level` 改名为 `level_override`，供 `Discover` 与 `GrantMinion` 共用；`HandCard.reward_level` 与奖励牌发放流程不变。
- `triple-reward` 的 `reward_level` 仍为来源星级 +1（6 星封顶）；打出奖励牌时把该值写入 `level_override`，由 `GrantMinion` 消费。
- `combat.rs` 播种幼苗判定改为查 `cards` 中 `seedling` 且 `in_shop_pool == false`。
- `triple.rs::REWARD_CARD_ID` 保留用于发放奖励牌，不再作为排除条件。
- `room-service`、`sim`：启动常量删除 `TOKENS_JSON`，`Catalog::from_json` 改为 5 个数据文件。

### 3.3 协议与兼容

- 协议 schema 不变；视图仍只传卡牌 id、身材、金色等既有字段。
- 数据文件整体重新生成，不做运行时迁移。
- 非池卡进入手牌或战场时，视图按普通卡读取；`card_type` 仍为 `minion` / `effect`。

## 4. 客户端

### 4.1 CardDB

- 删除 `tokens` 字典与 `generated/tokens.json` 加载；`cards` 直接包含全部 432 张。
- `display_name` 统一取 `card.<id>.name`；`display_text` 不变。
- `play_categories` / `primary_playstyle` 删除 Token 单值回退；基础细胞无 `play:` 时显示「未定/中立」。
- `CardPool.card_ids` 跳过 `in_shop_pool == false` 的卡，口径与服务端 `card_in_pool` 一致。

### 4.2 图鉴

- 大厅图鉴仍为 432 项；Token 现在有 `era` / `level` / `subtype`，按普通随从参与时代、玩法与随从筛选。
- 图鉴详情类型行统一显示「随从」；删除 `ui.gallery.type_token` 分支与文本键。
- 时代、玩法、随从/效果计数随 432 张数据重算；`ui_smoke_gallery` 断言同步。
- 对局覆盖层仍只列本局池内卡；`ammonite-token` 继续不在池。

### 4.3 文本

- `client/text/zh_CN.csv`：删除 23 个 `token.<id>.name`，新增对应 `card.<id>.name`；删除 `ui.gallery.type_token`。
- 其它文本 key 不变；`card.<id>.match_text` 只属于效果牌，Token 不新增。

## 5. 测试与验证

- 数据：`make data-check`、`make docs-render`（属性表与效果正文）、`python3 scripts/validate_docs.py`。
- 服务端：`make lint`、`make test`；覆盖：
  - 432 张卡加载，无 `tokens.json`；
  - `in_shop_pool` 缺省为真，`false` 卡不进商店、共享池、三连、发现；
  - 召唤 `token` 不扣池、`card` 扣池；
  - 非法 `token` / `card` 参数被模板校验拒绝；
  - `GrantMinion` 不登记待选、不扣共享池，随机结果落在来源星级 +1 的池内随从；
  - 手牌满、无候选、7 星来源封顶 6 星三种边界；
  - `GrantMinion` 获得的随从照常触发三连判定。
- 客户端：`make client-check`、`make client-smoke`；更新图鉴计数、类型文字与文本 key 断言。
- 端到端：`make m1-check`；必要时运行 `cargo test -p room-service --test e2e` 与 `make m1b-e2e`。
- 模拟器：三连奖励从“待选发现”改为“直接发牌”会改变固定种子对局，按仓库约定重定 golden 基线并记录原因。
- 文档：相对链接校验，核对 `docs/versions/v0.1/product/`、`docs/current/` 与索引页。

## 6. 观测与回滚

- 启动加载失败时错误信息写明 `cards.json` 与卡牌 id。
- 不新增指标；商店、召唤与三连沿用现有日志与事件。
- 回滚：同一提交内恢复源文档、生成物与服务端/客户端改动；无外部状态迁移。

## 7. 风险

- 任何仍按 `catalog.tokens` 或 `CardDB.tokens` 判断的路径都会漏掉合并后的非池卡；实现时统一改为 `in_shop_pool`。
- 三连判定若忘记加 `in_shop_pool`，衍生物会被误合；测试必须覆盖三个同名衍生物不上金。
- 发现与 `pool=playstyle` 候选若忘记过滤，Token 会进入候选。
- 图鉴计数、`EXPECT_ERA` / `EXPECT_LEVEL`、`balance_report.py` 与冒烟断言必须同批更新，不能只改数据。
- 基础细胞没有 `play:` 归属；客户端与服务端都必须按「中立/不命中」处理，不能回退到默认玩法。

## 相关文档

- [产品增量](product.md)
- [卡牌规范与数据](../../product/content/card-system.md)
- [核心规则](../../product/rules/core-rules.md)
- [客户端技术](../../../current/technical/client.md)
- [规则引擎技术](../../../current/technical/rule-engine-and-simulator.md)
