# 卡牌关联分类技术方案

> 状态：方案未实施
> 需求归属：v0.1
> 产品规则：[product.md](product.md)（已确认）

## 1. 改动边界

- 包含：441 份卡牌与英雄源文档、Python 生成与校验、已提交生成物、`rule-engine` 数据模型与筛选、`room-service` 英雄校验、`sim` 的 AI 与统计、客户端 `CardDB` 与图鉴 / 详情 / 选英雄页、文本表、测试与文档。
- 不包含：协议 schema 字段、数据库迁移、部署配置、13 个玩法的核心循环、卡牌数值与技能效果、Token 注册表。
- Token 不是卡牌，继续使用单值 `playstyle`（可为 `neutral`）；`Playstyle::Neutral` 仅保留给 Token 与引擎内部哨兵，不再出现在卡牌数据中。

## 2. 数据模型与迁移

### 2.1 源文档

- 卡牌与英雄 front matter 删除 `playstyle` 与 `specialty`，新增：

```yaml
categories: [play:mammal, play:bird, prey:ungulate, host:bacteria]
```

- 条目规则：
  - `play:` 至少 1 条，取值只能是 13 个玩法 id，不含 `neutral`。
  - `prey:` / `host:` 可 0 条，只允许生物卡；节点必须在分类树中。
  - 同一张卡内不允许重复条目。
  - 顺序固定：先按 `taxonomy.md` §2 的 13 个玩法顺序排列 `play:`，再排列 `prey:`，最后排列 `host:`；同类内按分类树声明顺序。
- 迁移脚本从现有 `playstyle` + `specialty` 推导：
  - 非 `neutral` 卡：`categories = [play:<playstyle>] + specialty 中的 play/prey/host`。
  - `neutral` 卡：`categories = specialty 中的 play/prey/host`。
  - 61 张无 play 的效果牌使用 `product.md` §3 的指派表写入 `play:`。
  - 迁移脚本放 `output/` 临时目录，迁移完成后删除，不提交。
- `## 属性` 表：
  - `玩法` 行显示全部 `play:` 的展示名，按玩法顺序用「、」分隔。
  - `关联分类` 行只显示 `prey:` / `host:`，格式保持「捕食：…；寄生：…」；没有时省略该行。
  - 学者不再出现「擅长：」段，工具卡不再出现「关联：」段，效果牌不再出现 `play:` 关联段。
- `cards/README.md` 与 `heroes/README.md` 索引表按卡牌 `categories` 更新「玩法」列；英雄索引删除「关联」列，改为「英雄 | 玩法 | 技能」。

### 2.2 生成物

- `scripts/gen_card_data.py` 输出的 `cards.json`、`heroes.json` 用 `categories` 数组，元素为 `{"kind": "play"|"prey"|"host", "value": "..."}`，不再输出 `playstyle` / `specialty`。
- `tokens.json` 不变，仍输出单值 `playstyle`。
- `client/generated/cards.json`、`client/generated/heroes.json` 同步生成并随源提交。

## 3. 服务端

### 3.1 数据模型

`server/crates/rule-engine/src/data.rs`：

- `SpecialtyKind` 改名 `CategoryKind`，`Specialty` 改名 `Category`；`Card` 与 `HeroCard` 的字段改为 `categories: Vec<Category>`。
- 删除 `Card.playstyle`、`Card.specialty`、`HeroCard.playstyle`、`HeroCard.specialty`。
- 新增辅助函数：
  - `fn play_categories(categories: &[Category]) -> impl Iterator<Item = &str>`。
  - `fn has_play(categories: &[Category], playstyle: &str) -> bool`。
  - `fn in_pool(categories: &[Category], enabled: &BTreeSet<Playstyle>) -> bool`：任一条 `play:` 命中启用玩法即入池；没有 `play:` 返回 `false`（加载期已拒绝）。
- `Playstyle::Neutral` 保留，但不参与卡牌数据与 `Playstyle::ALL`。

加载期校验：

- 每张卡与英雄至少 1 条 `play:`；玩法 id 合法且不含 `neutral`。
- `prey:` / `host:` 节点在分类树中，且卡牌 `taxon` 根节点属于生物界。
- 同卡不允许重复 `categories` 条目。
- 英雄与同 id 随从的 `categories` 与 `skills` 完全一致。
- 移除「造物卡机械玩法必须 taxon=automation」的限制：28 张工具卡改为 `play:machine` 后仍保留各自造物分类；机械玩法可落在造物六类工具分类。

### 3.2 效果筛选与 AI

- `CardFacts` 改为持有 `categories`；Token 仍用 `token.playstyle()` 合成一条玩法。
- `playstyle` 与 `play` 两个筛选键都改为「任一条 `play:` 命中」；`prey` / `host` 从 `categories` 读取。这样带 `play:machine` 的学者与工具卡会被「你的机械随从」等筛选选中，符合已确认的产品规则。
- `validate_filter_value` 继续接受 `neutral`，用于 Token 与内部哨兵。
- `sim/src/ai.rs`：
  - `dominant_playstyle`：每张卡按每条 `play:` 各计 1 次；没有 `play:` 的卡跳过；无任何计数时返回 `Neutral`。
  - `effect_matches_playstyle`：效果牌没有主导玩法时始终可买；有主导玩法时，任一条 `play:` 等于主导玩法才可买；没有 `play:` 的效果牌按可买处理（防御性兜底）。
- `effects/evolve.rs`：可进化判定改为「非学者且没有 `play:machine`」，替代原 `playstyle != Neutral && != Machine`。
- `sim/src/lib.rs` 的 `winner_playstyle` 继续复用 `dominant_playstyle`，口径随上。

### 3.3 room-service

- `server/crates/room-service/src/lib.rs` 的英雄加载校验改为 `categories`：玩法合法、节点合法、至少 1 条 `play:`。
- 英雄候选过滤继续调用 `hero_in_pool`，实现改为多玩法任命中。

## 4. Python 脚本与校验

- `scripts/validate_docs.py`：
  - 字段白名单删除 `playstyle` / `specialty`，加入 `categories`。
  - `specialty_errors` 改为 `category_errors`：语法、前缀、重复、至少 1 条 play、play 取值、prey/host 生物限制。
  - 删除「学者 playstyle 必须 neutral」「中立非学者只能造物工具」「效果牌只允许 1 条 play」等旧规则。
  - `effective_playstyle` 改为 `effective_playstyles(doc)` 返回列表；关键词约束改为「任一条 play 命中允许集合」。`assemble` 允许 `play:machine`，`decompose` / `sow` / `poison` / `armor` / `stealth` 同理。
  - `specialty_display` 改为只渲染 prey/host；`expected_card_attrs` 与 `expected_hero_attrs` 的「玩法」行取全部 play。
  - `check_hero_cards` 比较 `categories` 与 `skills`。
  - 数据总览的玩法计数改为多归属计数，卡牌总数仍按唯一 id 计。
  - 旧字段残留检查加入 `playstyle` / `specialty`。
- `scripts/gen_card_data.py`：`parse_specialty` 改为 `parse_categories`；`card_entry` / `hero_entry` 输出 `categories`。
- `scripts/gen_client_data.py`：透传生成物，字段随 `gen_card_data` 变化。
- `scripts/gen_text.py`：读取卡的 `categories` 字段做枚举校验；`enum.playstyle.*` 仍由 `taxonomy.md` 生成，`enum.playstyle.neutral` 保留供 Token 使用。
- `scripts/render_docs.py`：属性表由更新后的 `expected_*_attrs` 重写，迁移后执行一次。
- `scripts/balance_report.py`：`TARGETS` 删除 `neutral`；按每条 play 统计玩法数值与定向效果牌数量；`vocabulary_errors` 的「中立/学者跳过」改为按 `categories` 判断。
- `scripts/i18n_terms.py`：生成前缀不变；新增触发标签使用人工 `ui.` 前缀，不需要扩展生成前缀。

## 5. 客户端

### 5.1 CardDB 与文本

`client/autoload/card_db.gd`：

- 新增 `play_categories(entry) -> Array[String]`：读 `categories` 的 `play` 条目；Token 回退 `playstyle`。
- 新增 `primary_playstyle(entry) -> String`：取第一条 `play:`；Token 回退 `playstyle`；空时返回 `"neutral"`。
- 新增 `category_values(entry, kind) -> Array[String]`：读取 `play` / `prey` / `host`。
- 新增 `skill_info(skill_id) -> Dictionary`：返回 `name`、`text`、`trigger`、`trigger_label`、`cost`、`uses`、`is_active`。
- 新增 `trigger_name(trigger) -> String`：读 `ui.skill.trigger.<trigger>`；空触发且技能有关键词时返回「关键词」。
- `playstyle_name` 不变。

`client/scenes/card_text.gd`：

- `playstyle_text(entry)` 改为拼接全部玩法名，用「、」分隔。
- 新增 `association_text(entry)`：只拼接 prey/host；供详情复用。

`client/text/zh_CN.csv`：

- 新增 `ui.skill.trigger.*` 标签、`ui.skill.uses`、`ui.skill.cost`、`ui.skill.empty`。
- `ui.game.detail.association` 文案改为「关联分类」。
- 删除或停用不再使用的 `ui.game.detail.specialty`（「擅长玩法」）与 `ui.game.detail.hero_power`（英雄技能标题按新列表处理）。

### 5.2 卡池与图鉴

- `client/scenes/card_pool.gd`：任一条 `play:` 命中启用玩法即入池。
- `client/scenes/card_gallery.gd`：
  - 分类项为 `CardDB.playstyles` 的 13 个玩法，删除 `CATEGORY_NEUTRAL`。
  - 随从分类归属取全部 `play:`；英雄分类同样取全部 `play:`，删除 `_hero_taxon` 对单值 `playstyle` 的读取。
  - 分类计数按多归属累加；「全部」按唯一卡牌数计。
  - 切换分类后的分页与选中行为不变。
- `client/scenes/gallery_detail.gd`：
  - 玩法行显示全部玩法名。
  - 关联分类行只显示 prey/host。
  - 英雄详情删除「技能」名称行与拼接效果块，改为 `UI.skill_list` 逐条展示。
  - 预览封面颜色取 `primary_playstyle`。
- `client/scenes/game_detail_layer.gd`：卡牌与英雄详情同样改为多玩法与 prey/host 关联；英雄 hover 详情继续显示玩法与技能文本，不新增技能列表。
- `client/scenes/game_hero_card.gd`、`client/ui/ui.gd`、`client/scenes/game_shop_view.gd`：封面 / 边框颜色取 `primary_playstyle`。
- `client/scenes/room.gd`：技能浮层在英雄名下方调用 `UI.skill_list`；浮层宽度 320，文本自动换行，英雄技能最多 2 条，不需要滚动。

### 5.3 技能列表

- 在 `client/ui/ui.gd` 新增 `static func skill_list(parent: VBoxContainer, skill_ids: Array) -> void`：
  - 每条技能一个块：技能名（有名称时）与触发标签同一行，文本在下一行；主动技能追加「费用 N · 每回合 N 次」。
  - 触发标签使用 `UI.pill_label`，无技能时显示 `ui.skill.empty`。
  - `gallery_detail.gd` 与 `room.gd` 共用该函数。
- 触发标签固定映射：

| trigger | 标签 |
| --- | --- |
| `on_activate` | 主动 |
| `on_play` | 战吼 |
| `on_play_effect` | 效果牌 |
| `on_death` | 亡语 |
| `on_combat_start` | 战斗开始时 |
| `on_turn_start` | 回合开始时 |
| `on_first_refresh` | 每回合首次刷新时 |
| `aura` | 光环 |
| `conditional_aura` | 条件光环 |
| `on_kill` | 击杀时 |
| `on_friendly_death` | 友方死亡时 |
| `on_enemy_death` | 敌方死亡时 |
| `on_ally_kill` | 友方击杀时 |
| `on_ally_summon` | 友方召唤时 |
| `on_ally_growth` | 友方成长时 |
| `on_ally_evolve` | 友方进化时 |
| `on_use` | 使用效果牌时 |
| `on_deal_damage` | 造成伤害时 |
| `on_combat_end` | 战斗结束时 |
| 空触发 | 关键词（技能没有效果段时）/ 常驻 |

## 6. 文档

- 内容规范：`card-system.md` §1、§1.1、§2.1、§3；`taxonomy.md` §2、§3；`skill-system.md` §2.2 第 10–11 条；`cards/README.md`；`heroes/README.md`。
- 平衡：`balance-analysis.md` 的玩法计数口径；`balance-questions.md` 记录效果牌分布偏机械/哺乳纲。
- UI：`docs/current/product/ui/pages/gallery.md`、`pages/game.md`、`components/card-gallery.md`、`components/gallery-toc.md`、`components/card-panel.md`、`interactions.md`、选英雄页文档。
- 本需求目录保留 `product.md` 与 `technical.md`，实现完成后按 runbook 删除已实现部分。

## 7. 迁移顺序

1. 写临时迁移脚本，改写 441 份源文档 front matter。
2. 更新 `validate_docs.py` 与 `gen_card_data.py`。
3. 运行 `python3 scripts/render_docs.py` 重写属性表与索引。
4. 运行 `python3 scripts/gen_card_data.py`、`python3 scripts/gen_client_data.py`、`python3 scripts/gen_text.py` 生成数据与文本。
5. 更新 Rust 数据模型与筛选、room-service、sim。
6. 更新客户端 `CardDB`、卡池、图鉴、详情、选英雄页。
7. 更新测试与文档。
8. 按验证矩阵执行全量检查。

## 8. 测试与验证

- `python3 -m unittest discover -s tests -v`。
- `cargo test --workspace`。
- `make m1-check`（协议生成物、文档数据、测试、静态检查、确定性）。
- `make client-check`、`make client-smoke`。
- 端到端：`cargo test -p room-service --test e2e`、`cargo test -p room-service --test reconnect`、`make m1b-e2e`。
- 确定性：`make m0-determinism`、`make m0-batch`、`make m0-report`。
- 重点断言：
  - 409 张卡与 32 张英雄都有至少 1 条 `play:`，无 `neutral`。
  - 61 张指派效果牌只在指定玩法中出现。
  - 图鉴分类栏无「中立」，多玩法卡在多个分类中命中，「全部」不重复。
  - 英雄候选只含命中启用玩法的英雄。
  - 英雄详情与选英雄浮层的技能条数与 `skills` 一致，主动技能显示费用与次数。

## 9. 风险

- 效果牌玩法池由「通用」变为按玩法分配，机械 28 / 哺乳纲 25 明显多于其他玩法；平衡报告与 `balance-questions.md` 会记录。
- 多玩法归属参与对局筛选后，学者与工具卡会进入对应玩法的光环 / 战吼目标，数值表现会变化；这是已确认规则。
- 图鉴分类计数会出现重复计入，跨分类合计大于卡牌总数；文档与测试按此口径更新。
- 客户端多处直接用 `playstyle` 取色或取文本，必须统一走 `primary_playstyle` / `playstyle_text`，避免多玩法时取到空值。
- `validate_docs.py` 与 `render_docs.py` 的字段名耦合较强，先改校验再渲染，避免属性表漂移。
- `sim` 的主导玩法统计改为多归属计数后，AI 行为与夺冠率报告会变化；通过 `make m0-determinism` 与 `make m0-report` 观察。

## 10. 兼容与可观测性

- v0.1 尚未上线，不为 `playstyle` / `specialty` 保留读取兼容；源文档、生成物与服务端同批切换。Token 的单值 `playstyle` 不受影响。
- 协议 `enabledFactions` / `enabledPlaystyles` 等字段不变，客户端与服务端仍按玩法 id 通信。
- 不新增日志、指标或追踪；玩法分布与效果牌池变化通过 `scripts/balance_report.py` 与模拟器报告观察。
- 回滚边界：本改动不涉及数据库与部署，回滚即恢复源文档与生成物；已生成的对局日志若包含旧卡牌字段，按 v0.1 未上线处理，不提供跨版本回放兼容。
