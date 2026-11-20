# 英雄卡独立技术方案

> 状态：已实施
> 需求归属：v0.1
> 产品规则：[product.md](product.md)（已确认）

## 1. 改动边界

- 包含：卡牌源文档、数据生成与校验、rule-engine 数据模型与查询、room-service 候选分配、sim、客户端 CardDB / 图鉴 / 英雄行 / 英雄详情、文本表、测试与文档。
- 不包含：协议 schema 字段改名、数据库迁移、部署与配置变更。
- 英雄卡与随从卡同名同 id，分属两个集合；协议 `heroCardId` 语义不变，只指向英雄池。

## 2. 数据与生成

### 2.1 源文档

- 新增 `docs/versions/v0.1/product/content/heroes/`，每张英雄卡一份 Markdown，共 32 份；文件名与 id 一致。
- front matter 仅允许：`id`、`card_type`（固定 `hero`）、`name`、`playstyle`、`specialty`、`skills`。
- 英雄卡的 `id`、`name`、`playstyle`、`specialty`、`skills` 与对应随从卡一致；id 与名称不加前缀。
- 正文沿用卡牌文档结构：一级标题、`## 属性`、`## 效果`、`## 设计说明`、`## 出处`；属性表只列 id、名称、类型、玩法、关联分类、技能。
- 原 32 张随从卡删除 `is_hero: true`，属性表删除「英雄资格」行，其余字段与正文不变。

### 2.2 生成

- `scripts/gen_card_data.py`：
  - `FILES` 增加 `heroes.json`；新增 `hero_entry(doc)`，输出 id、card_type、playstyle、specialty、skills。
  - `card_entry` 删除 `is_hero`。
  - `build` 读取 `heroes` 目录并写出 `server/data/v0.1/heroes.json`。
- `scripts/gen_client_data.py`：`FILES` 增加 `heroes.json`，输出 `client/generated/heroes.json`，字段与 `hero_entry` 一致。
- `scripts/gen_text.py`：英雄 id 与随从相同，名称复用 `card.<id>.name`；不新增逐卡文本 key。
- 生成物 `server/data/v0.1/heroes.json` 与 `client/generated/heroes.json` 随源提交。

### 2.3 校验

- `scripts/validate_docs.py`：
  - `card_type` 合法值增加 `hero`；`cards/` 目录不得出现 `card_type: hero`，`heroes/` 目录不得出现 `minion` / `effect`。
  - 英雄卡恰好 32 张；英雄池内 id 唯一；每张英雄卡必须存在同 id 随从卡，且 `playstyle`、`specialty`、`skills` 与随从卡一致。
  - 英雄卡字段白名单校验；不得出现 `attack`、`health`、`level`、`era`、`taxon`、`is_scholar`、`is_hero`、`cost`。
  - 随从卡与效果牌不得再出现 `is_hero`；「英雄资格」属性行从校验目标中删除。
  - 英雄卡属性表按英雄字段生成并交叉核对；每张英雄卡保留非空 `## 出处`。
  - 更新校验输出文案：`英雄卡 32 张` 保留，普通随从与效果牌数量不变。
- `make data-check` 必须无漂移。

## 3. 服务端

### 3.1 rule-engine 数据模型

- `CardType` 增加 `Hero`。
- `Card` 删除 `is_hero` 字段。
- 新增 `HeroCard { id, card_type, playstyle, specialty, skills }`，`deny_unknown_fields`。
- `Catalog` 增加 `heroes: BTreeMap<String, HeroCard>`；`from_json` 增加 `heroes` 参数。
- 加载校验：
  - `cards.json` 只允许 `minion` / `effect`；`heroes.json` 只允许 `hero`。
  - 英雄池内 id 唯一；英雄技能必须存在于技能表；英雄 `playstyle` / `specialty` 合法。
  - 每张英雄卡必须有同 id、同名的随从卡（`card_type == Minion`），且 `playstyle`、`specialty`、`skills` 与随从卡一致；不一致直接拒绝加载。两池可以同名，但共享字段不允许分叉。
  - 英雄池与随从池不要求跨池唯一。
- 新增 `hero_in_pool(&HeroCard, &BTreeSet<Playstyle>)`，规则与 `card_in_pool` 一致；两者共用玩法过滤逻辑。

### 3.2 查询与规则

- `engine.rs` 开局校验英雄：`catalog.heroes.get(id)`，并要求 `card_type == Hero`。
- `recruit.rs` 英雄技能校验与结算：英雄与技能查找改走 `catalog.heroes`；删除购买、卡池、三连、发现中的 `is_hero` 过滤。首次刷新改为无来源 `fire(OnFirstRefresh)`，由执行器解析英雄池。
- `view.rs`：`hero_power_skill_id` 与 `hero_power_requires_target` 都只读 `catalog.heroes`；新增 `Engine::hero_requires_target`，与随从的 `requires_target` 共用选择器判定。
- `effects/aura.rs`：无来源（英雄）光环按 `catalog.heroes` 取技能，棋盘随从仍按 `catalog.cards`。
- `effects/exec.rs`：战斗开始的英雄效果走 `combat_hero_effects`（英雄池）；新增 `owner_effects` / `run_effects`，无来源的 `OnActivate` / `OnTurnStart` / `OnFirstRefresh` 触发按英雄池解析技能，来源为场上卡时仍按随从池解析；同名卡的两个池互不误读。
- `engine.rs`：`fire_turn_start` 先结算英雄的 `OnTurnStart`，再按站位结算随从。
- `ai.rs`、`effects/shop_effects.rs`、`effects/exec.rs`、`triple.rs`：删除 `is_hero` 判断；随从筛选只保留 `card_type == Minion`。
- `sim` 英雄分配、机器人取名与测试辅助：遍历 `catalog.heroes`；`Catalog::from_json` 增加 `include_str!(heroes.json)`。
- `room-service`：
  - 启动常量增加 `HEROES_JSON`；`Catalog::from_json` 传入英雄数据。
  - `ensure_hero_pool` 改查 `catalog.heroes`，校验玩法与关联玩法；不再校验 `taxon`。
  - `assign_candidates` 与 `select_hero` 改查 `catalog.heroes`。
  - 选英雄只接受英雄池中的 id；普通随从 id 即使与英雄同名也不会命中英雄池以外的路径。

### 3.3 协议与兼容

- 协议 schema 不变；`heroCardId`、`heroCandidates` 仍传 id。
- 同名 id 靠集合隔离：任何代码不得用 `catalog.cards` 查英雄，也不得用 `catalog.heroes` 查商店 / 牌池卡。
- 无数据库迁移；旧数据文件整体重新生成。

## 4. 客户端

### 4.1 CardDB

- 新增 `heroes: Dictionary`，读取 `res://generated/heroes.json`；不并入 `cards`，避免同 id 覆盖。
- 新增 `hero(id)`、`heroes_array()`；`display_name` 继续按 id 取文本，名称与随从相同。
- 新增 `hero_text(id)`：按英雄 `skills` 拼效果文本，与 `display_text` 共用技能文本拼接；`display_text(id)` 继续取随从池。
- 英雄美术沿用同 id 的卡面资源；无资源时走现有首字占位。

### 4.2 对局展示

- `game_hero_row.gd`：不再读取英雄卡攻击力；`GameHeroCard.refresh` 改为 `(hero_id, hp, hp_cap, era)`。
- `game_hero_card.gd`：取 `CardDB.hero(hero_id)` 的玩法；移除攻击格，保留时代与生命两格，卡面尺寸不变。
- `game_detail_layer.gd`：英雄详情取 `CardDB.hero(id)`，显示玩法、关联玩法与英雄技能；不显示科学分类与时代。

### 4.3 图鉴

- `CardGallery` 顶层分类增加 `hero`（展示名「英雄」），计数为英雄集合大小。
- 英雄分区使用独立 id 集合（`set_catalog` 的 `hero_ids`；`null` 表示未指定、取 `CardDB.heroes_array()`，空数组表示本局无可用英雄并显示空态）；选中「英雄」时隐藏时代章节栏、清空并忽略时代筛选、二级筛选项为空，网格只列英雄卡。
- `GalleryGrid.set_cards(ids, selected_id, hero)` 把英雄标记传给 `GalleryCardTile`；`GalleryCardFace.build_hero` 渲染「英雄」角标与玩法，不含费用、时代与攻击 / 生命。
- `GalleryDetail.show_hero(id)`：英雄取 `CardDB.hero(id)`，只显示类型、玩法、关联、技能与效果文本；不显示科学分类、时代、费用与初始数值。
- 普通分区仍使用 `CardDB.cards`，432 项与既有筛选、分页、详情行为不变。

### 4.4 文本

- `client/text/zh_CN.csv` 增加 `enum.card_type.hero`（英雄）与 `ui.gallery.category_hero`（英雄）。
- 现有 `ui.game.detail.hero` 继续用于英雄标识与详情标题。

## 5. 测试与验证

- 数据：`make data-check`、`make docs-render`（如涉及索引）、`python3 scripts/validate_docs.py`。
- 服务端：`make lint`、`make test`；新增用例覆盖：
  - `cards.json` 与 `heroes.json` 分池加载，同 id 可共存；
  - 英雄候选只来自英雄池，购买 / 商店 / 三连 / 发现不命中英雄；
  - 英雄技能查询走英雄池，普通随从同名不受影响。
- 客户端：`make client-check`、`make client-smoke`；更新图鉴计数、英雄分区、英雄行布局与英雄详情断言。
- 端到端：`make m1-check`；必要时运行 `cargo test -p room-service --test e2e` 与 `make m1b-e2e`。
- 文档：相对链接校验，核对 `docs/versions/v0.1/product/content/`、`docs/current/` 与索引页。

## 6. 观测与回滚

- 启动时目录校验失败直接拒绝启动，错误信息写明文件与卡牌 id。
- 不新增指标；英雄选择、技能使用沿用现有日志与事件。
- 回滚：同一提交内恢复数据源与生成物、服务端与客户端改动；无外部状态需要迁移。

## 7. 风险

- 同名 id 是本方案的核心约束；遗漏任一 `CardDB.card(hero_id)` 或 `catalog.cards.get(hero_id)` 都会读到随从卡。实现时统一改为按用途查池，并用测试锁定。
- 图鉴英雄分区不能复用随从的玩法 / 类型筛选；计数与章节栏需要单独分支。
- 英雄卡没有攻击、时代、科学分类，所有展示组件必须移除对应取数，避免出现 0 或空行。
- `validate_docs.py` 的固定数量与输出文案、`m1-check` 断言、客户端冒烟计数需要同批更新，不能只改数据。

## 相关文档

- [产品增量](product.md)
- [卡牌系统](../../product/content/card-system.md)
- [核心规则](../../product/rules/core-rules.md)
- [客户端技术](../../../current/technical/client.md)
- [服务端技术](../../../current/technical/room-service/README.md)
