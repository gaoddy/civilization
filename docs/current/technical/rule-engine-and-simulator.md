# 规则引擎与模拟器

> 状态：已实现（v0.1 M0；M1a 的增量见[房间服务与联网](room-service.md)）
> 代码：`server/crates/rule-engine`、`server/crates/sim`、`scripts/`、`Makefile`

本文记录 v0.1 M0 已实现的规则引擎与模拟器的现状：工程结构、引擎边界、随从属性分层、数据生成、命令与验证方式；不描述产品规则与后续里程碑规划。

## 1. 工程结构

```
server/
  Cargo.toml                 # workspace：crates/rule-engine、crates/sim
  rust-toolchain.toml        # 工具链 1.94.1，edition 2024
  data/v0.1/
    cards.json               # 生成物：220 张卡
    skills.json              # 生成物：118 个技能
    tokens.json              # 生成物：3 个 Token
    config.json              # 生成物：经济与对局配置
  crates/
    rule-engine/             # 纯库
      src/state.rs           # GameState / PlayerState / MinionInstance / MinionStats
      src/data.rs            # Catalog 与加载期校验（deny_unknown_fields）
      src/effects/           # 结构化效果校验（validate）、解释执行（exec）、光环（auras）；parse 仅保留为 DSL 作者工具
      src/intent.rs          # 协议十一种意图（含 ChooseDiscover、ChooseTarget）与 11 个规则拒绝码（各自映射到对应的协议错误码）
      src/engine.rs          # Engine::new / apply / start_game / start_turn
      src/recruit.rs         # 商店、买/卖/刷新/冻结/升时代/打出/结束招募
      src/triple.rs          # 三连与发现
      src/combat.rs          # 战斗模拟
      src/settle.rs          # 配对、finish_recruit、伤害、淘汰、排名
      src/rng.rs             # xoshiro256++ 与五个标签流；seed_from_u64（M1a 由 sim 迁入）
      src/hash.rs            # FNV-1a 规范编码与状态哈希
      src/view.rs            # 逐座位视图 SeatView（M1a 新增，供 room-service 映射协议视图）
      tests/                 # 17 个集成测试文件
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
  gen_card_data.py           # 实体文档 → server/data/v0.1/*.json
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
- 拒绝码与核心规则 §3 一致：`NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY`、`NO_SPACE`、`INVALID_STAGE`、`NO_SUCH_MINION`、`ALREADY_MAX_ERA`、`CARD_NOT_IN_HAND`、`HERO_POWER_LIMIT`、`NO_VALID_TARGET`、`TARGET_NOT_ALLOWED`、`INVALID_CHOICE`。
- 协议十一种意图：`BuyMinion`、`SellMinion`、`RerollShop`、`UpgradeEra`、`PlayCard`、`RearrangeBoard`、`FreezeShop`、`UseHeroPower`、`EndRecruit`、`ChooseDiscover`、`ChooseTarget`。`ChooseDiscover` 与 `ChooseTarget` 都用于消费引擎产生的 `PendingChoice`（发现/进化、目标）；`PendingChoice` 本身是引擎内部状态，不直接出现在协议里。
- `PlayCard { hand_index, position, target }`：`position` 只对随从牌生效，是入场后的插入下标，`None` 表示追加到最右；超出当前随从数返回 `INVALID_CHOICE`。效果牌不带落点：目标缺失或不在合法池内时整条意图返回 `NO_VALID_TARGET`，合法池为空时效果落空、意图仍生效。随从牌入场后判定战吼：未带 `target` 且合法池非空时推入 `PendingChoice::Target`，由 `ChooseTarget { minion }` 指定目标；合法池为空（含被过滤器收窄为空）时战吼直接落空、不产生待选。
- 阶段不匹配时，`recruit::apply` 对全部招募意图（含 `UpgradeEra`、`RerollShop` 等）统一返回 `INVALID_STAGE`；`EndRecruit` 另有「本座位已结束招募」时仍处于 `Recruit`、返回成功且无副作用。
- `BuyMinion { slot }` 不带目标：买入一律进入手牌，不触战吼、不分配实例编号；`PlayCard` 与 `UseHeroPower` 带 `target: Option<InstanceId>`，M0 由 AI 自动选择目标。
- `PendingChoice::Target { seat, source, card_id, golden }`：目标待选期间该座位除 `ChooseTarget` 外一切招募意图（含 `EndRecruit`）返回 `INVALID_STAGE`。目标待选在 `finish_recruit` 进入战斗前作废（战吼落空）；发现与进化待选允许留到下回合。`ChooseTarget` 要求队首是目标待选、目标是我方棋盘上的随从，且通过该牌 `OnPlay` 效果的合法池校验，否则返回 `INVALID_CHOICE` / `NO_VALID_TARGET`；消费后立即按 `OnPlay` 结算该牌效果。
- `HandCardView.requires_target` 由效果选择器决定：`Selector::Target` 或 `friendly:single`（`Side` + `Pick::Single`）即为需要目标，随从牌恒为 `false`。
- `EndRecruit` 只结束本座位；全员（未出局）结束后才进入 `Combat` 并发出 `PhaseChanged`，重复 `EndRecruit` 成功且无副作用。
- `settle::finish_recruit` 幂等：`Recruit` 阶段（超时路径）生成配对并切 `Combat`；`Combat` 阶段只补齐一次配对，不重复发事件、不重复配对。
- 阶段推进由驱动方按固定顺序调用 `finish_recruit` / `resolve_combat` / `resolve_settle` / `start_turn`，引擎不自行推进时间。
- `rule-engine` 顶部 `#![forbid(unsafe_code)]`；结算只用整数，容器用 `BTreeMap` 与有序 `Vec`。
- M1a 起引擎提供只读逐座位视图 `rule_engine::view::view(&Engine, &GameState, Seat) -> Option<SeatView>`；`SeatView` 是引擎自有类型，不依赖协议 crate，由 `room-service::view::to_protocol` 映射为协议 `PlayerView`。

## 3. 卡牌数据生成

- 唯一数据源：`docs/versions/v0.1/product/content/` 下的卡牌与技能实体文档；`scripts/gen_card_data.py` 复用 `validate_docs.py` 的 front matter 解析。
- 经济与对局配置由生成器从产品数值文档口径抄录到 `config.json`（初始生命、棋盘/手牌/商店槽位、回合上限、伤害上下限、升时代费用、星级牌份数、商店概率、金币曲线）。
- 输出确定性 JSON（UTF-8、两空格缩进、键排序、末尾换行）；生成物随源提交。
- 引擎用 `include_str!` 嵌入 `server/data/v0.1/*.json`，加载期由 serde 校验结构化字段与枚举，再由 `validate` 校验跨字段组合；未知字段或非法组合直接失败。源文档 `effects` 仍写 DSL，由生成器编译为结构化数组，生成期用与 `validate` 同规则的 `check_structured` 预检，避免 `make data` 通过而引擎加载失败。
- `make data-check` 重新生成到临时文件并比对，同时运行 DSL 审计。

## 4. 命令

| 命令 | 作用 |
| --- | --- |
| `make test` | Python unittest 与 `cargo test --workspace` |
| `make lint` | `cargo fmt --check` 与 `cargo clippy --workspace --all-targets -- -D warnings` |
| `make audit` | effects DSL 审计 |
| `make data` / `make data-check` | 生成卡牌数据 / 比对漂移并审计 |
| `make protocol` / `make protocol-check` | 生成协议 GDScript 常量与字段表 / 漂移检测 |
| `make m0-determinism` | 运行 `sim_determinism::replay_same_log_100_times_has_same_hash`：同一局用 `sim replay` 重放 100 次，哈希全部一致 |
| `make m0-batch` | 1000 局 8 AI 批量对局（seed 1），写入 `output/m0-batch.json` |
| `make m0-check` | `make data-check && make test && make lint && make m0-determinism`（计划中的 M0 汇总门禁） |
| `make m1-check` | M1a 汇合门禁：`protocol-rs-check`、`protocol-check`、`data-check`、`test`、`lint`、`m0-determinism`；e2e 由 `make test` 覆盖，详见[房间服务与联网](room-service.md) |
| `make m0-report` | 先校验批量日志（`sim check --expect-games 1000`：局数、每局 8 名排名），再生成基线报告（字段口径见第 6 节）；输出同时写入 `output/m0-report.log`。可用 `M0_BATCH_LOG`、`M0_GAMES` 指向其他批量日志与局数 |
| `make dev-bots` | 本地联调：起服务端与 7 个机器人，打印房间码后等开发者作为第 8 人入座。`DEV_PORT` 默认 8080，端口被占用时直接失败 |
| `python3 scripts/validate_docs.py` | 文档与实体数据校验 |

模拟器直接调用：`sim play --seed N`、`sim batch --games N --seed N --out PATH`、`sim replay --log PATH`、`sim report --log PATH`、`sim check --log PATH [--expect-games N]`、`sim bot --connect ws://HOST:PORT/ws --name NAME --hero CARD_ID [--room CODE] [--create] [--intent-seed N]`、`sim dev --connect ws://HOST:PORT/ws [--intent-seed N]`；参数错误、模拟失败或日志校验不通过以非零码退出。`sim bot` 的细节见[房间服务与联网](room-service.md)第 11 节。

`sim dev` 起 7 个机器人：第 0 个建房并把房间码打印为 `ROOM_CODE=<code>`，其余 6 个在取到码后并发加入，给开发者留出恰好一个座位。英雄卡按 id 升序取自内嵌卡表，第 `index` 个机器人的意图种子为 `intent_seed + index`。取房间码超时 10 秒，取到码后等待开局与整局结束的总超时 900 秒；超时或任一机器人失败时取消其余机器人并以非零码退出。

`sim bot` 只在视图变化时决策；服务端拒绝意图不会带来新视图，因此收到 `Error` 后每回合最多补发一次 `EndRecruit`（`brain::recover_after_error`），避免整局停在招募阶段等超时。该兜底由 `make m1b-e2e` 的稳定运行暴露并覆盖。

`sim bot` 的意图表同步扩到十一种：`pendingChoices` 头部为 `target` 时发 `ChooseTarget { minion }`，取己方棋盘最左侧随从；`PlayCard` 一律带 `position: None`（追加到最右），并读手牌视图 `requiresTarget`：为真且棋盘非空时预填己方第一个随从为目标，棋盘为空时不填（效果可落空）。`sim/src/ai.rs`（M0 基线）同样按该规则消费目标待选。

## 5. 验证方式

验证矩阵：`make data-check && python3 scripts/validate_docs.py && make m0-check && make m0-batch && make m0-report`（其中 `make m0-check` = `make data-check && make test && make lint && make m0-determinism`）。

每个 M0 标准类别由下列命令与字段支撑；表中「报告字段」指 `make m0-report` 输出的 JSON 字段。

| M0 标准 | 命令 | 证据 |
| --- | --- | --- |
| 卡池完整 | `make data-check`、`python3 scripts/validate_docs.py`、`make test` | 命令退出码 0；`catalog_load::loads_all_committed_data` 断言 220 卡 / 118 技能 / 3 Token / 13 英雄卡全部加载，`every_card_skill_exists`、`unknown_field_fails`、`effects_schema::committed_data_is_structured`；`audit_effects.py` 输出 `审计通过` |
| 数据一致 | `make data-check` | 重新生成后输出 `生成数据无漂移`，退出码 0 |
| 规则覆盖 | `make test` | `server/crates/rule-engine/tests/` 与 `server/crates/sim/tests/` 的用例全部 `test result: ok`（用例清单见第 7 节）；商店池耗尽留空槽由 `recruit::reroll_with_exhausted_pool_leaves_empty_slots_and_buy_rejects` 锁定 |
| 确定性 | `make m0-determinism` | `replay_same_log_100_times_has_same_hash` 通过：100 次重放每次退出码 0，`final_hash` 全部等于原局 |
| 稳定性 | `make m0-batch`、`make m0-report` | `make m0-batch` 退出码 0，无错误输出；产物 `output/m0-batch.json` 的 `games` 数为 1000（每局一条日志）。`make m0-report` 以 `sim check --expect-games 1000` 断言批量日志 `games` = 1000（批量退出码 0、异常/超时 0 的「无缺局」口径），局数不符即非零退出 |
| 可判定 | `make m0-batch`、`make m0-report` | `sim check`（由 `make m0-report` 调用）逐局断言 `ranking.len() == 8`、座位恰为 0–7、名次 1–8 连续，任一局不满足即非零退出并给出异常局数；`sim report` 也内置同一校验，stdout 仅输出 JSON。契约另由 `sim_cli::single_game_produces_ranking_and_hash`、`batch_of_ten_all_ranked`、`check_fails_on_incomplete_ranking` 固化 |
| 数值基线 | `make m0-report`（`output/m0-report.log`） | 退出码 0；`wins_by_tribe`、`win_rate_permille_by_tribe`、`avg_turns`、`era_upgrade_turns_by_seat`、`avg_era_upgrade_turn`、`triples_per_game`、`games_reaching_turn_20`、`survivors_at_turn_20`、`avg_damage_per_loss` 均有值；越界与口径问题登记 [待确认事项](../../versions/v0.1/open-questions.md) |

## 6. 报告字段口径

`make m0-report` 的 JSON：

| 字段 | 口径 |
| --- | --- |
| `games` / `seed` | 局数；批量起始种子（第 i 局种子为 `seed + i`） |
| `wins_by_tribe` | 各流派夺冠局数，只列出夺冠过的流派 |
| `win_rate_permille_by_tribe` | 各流派夺冠率整数千分比 = 夺冠局数 × 1000 / `games`，向下取整；7 个流派全部列出 |
| `avg_turns` | 各局结束回合之和 / 局数，向下取整 |
| `era_upgrade_turns_by_seat` | 8 个座位各自「首次升时代回合」的累加值（未升级的座位计 0） |
| `avg_era_upgrade_turn_by_seat` | 上项 / `games`，向下取整；未升级按 0 计入 |
| `avg_era_upgrade_turn` | 已记录首次升时代回合之和 / 记录数（`era_upgrade_count`），向下取整；未升级的座位不计入；无记录为 `null` |
| `era_upgrade_count` | 有首次升时代记录的（座位, 对局）数 |
| `triples_per_game` | 三连总数 / 局数，向下取整 |
| `games_reaching_turn_20` | `survivors_at_turn_20` 非空的局数 |
| `survivors_at_turn_20` | 上述对局在第 20 回合 `resolve_settle` 完成后的平均存活人数，向下取整；无此类局为 `null`。采样时点语义登记 [待确认事项](../../versions/v0.1/open-questions.md) |
| `games_ended_on_turn_20` | 结束回合为 20 的局数 |
| `avg_survivors_at_end` | 各局结束时的存活人数之和 / 局数，向下取整。「结束时存活人数」在 `GameOver` 时刻采样（驱动循环退出后取一次）。第 20 回合收束时 `resolve_settle` 只做排序与收束，不淘汰剩余玩家（淘汰只发生在生命归零时），未被淘汰的玩家都会被计入，因此该值不保证为 0；seed 1 的 1000 局实测分布为 1 人 62 局、2 人 333 局、3 人 468 局、4 人 124 局、5 人 13 局，平均 2.693（向下取整报告为 2） |
| `avg_damage_per_loss` | 每次战斗失败的伤害之和 / 败者次数，向下取整 |
| `final_hashes` | 每局最终状态哈希（16 位小写十六进制，按局序） |

`sim play` 的单局日志 `stats` 含 `era_upgrade_turns`（8 个座位各自的首次升时代回合，未升为 `null`）、`survivors_at_turn_20`（第 20 回合 `resolve_settle` 完成后）、`survivors_at_end`（`GameOver` 时刻）、`damage_per_loss`、`winner_seat`、`winner_tribe`。

## 7. 规则用例清单

`make test` 运行的测试文件即规则用例清单：

| 规则章节 | 用例文件 |
| --- | --- |
| 核心规则 §2 阶段状态机、§7 淘汰排名、§8 边界 | `rule-engine/tests/settle.rs`、`recruit.rs` |
| 核心规则 §3 十一种意图与 11 个失败码 | `rule-engine/tests/recruit.rs`、`choices.rs` |
| 核心规则 §4 招募细则 | `rule-engine/tests/recruit.rs`、`choices.rs`；商店重试后仍空即空槽，此时 `BuyMinion` 返回 `SHOP_SLOT_EMPTY`（`reroll_with_empty_star_retries_then_leaves_slot_empty`、`reroll_with_exhausted_pool_leaves_empty_slots_and_buy_rejects`） |
| 核心规则 §5 战斗与伤害公式 | `rule-engine/tests/combat.rs` |
| 核心规则 §6 三连与发现 | `rule-engine/tests/triple.rs`、`choices.rs` |
| 关键词机制 §1–§3 | `rule-engine/tests/turn_effects.rs`、`death_effects.rs`、`effects_exec.rs`、`auras.rs`、`combat.rs` |
| 关键词机制 §9、§10 | `rule-engine/tests/effects_parse.rs`、`effects_schema.rs`、`effects_exec.rs` |
| 时代与数值 | `rule-engine/tests/recruit.rs`、`settle.rs`、`turn_effects.rs` |
| 确定性 | `rule-engine/tests/determinism.rs`、`sim/tests/sim_determinism.rs` |
| 卡池加载 | `rule-engine/tests/catalog_load.rs` |
| 随从攻防分层 | `rule-engine/tests/stats.rs`、`stats_model.rs`、`stats_view.rs`、`combat_damage.rs` |

## 8. 随从属性分层

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
- 初始值在 `MinionInstance::new` 时解析：token 取 token 注册表，普通卡取卡牌攻防，金色 ×2；`is_scholar` 同时确定并存储。
- 读取口 `attack()` / `health()` 返回 `max(0, current)`；学者 `attack()` 恒为 0，但各层照常累积。战斗内部死亡判定直接用原始 `stats.health.current <= 0`，不经过非负夹紧。
- 视图 `view::MinionStatsView` / `StatBreakdownView` 把攻防映射为 `initial`、`permanent`（= `growth + permanent`）、`turn`、`aura`、`damage`、`current` 六项；在可达的下发状态下满足 `initial + permanent + turn + aura - damage == current`（复生造成的负 `damage` 只存在于战斗内部，不下发）。学者攻击六层全为 0。`health.current` 在视图中夹到 0，招募期负血量由 `damage` 吸收。`BoardMinion` 协议的 `stats` 与顶层 `attack` / `health` 复用同一口径。

## 9. 战斗伤害、复生与快照

- 行动轮次：`SideBoard` 每侧一个指针，双方逐拍交替出手，同一侧按站位从左到右、跳过阵亡与不能攻击的随从。指针越过该侧末尾后，该侧下一次轮到回到最左，并恢复全部存活随从的本轮行动次数（普通 1 次、连击 2 次），循环至一方全灭或 200 次行动上限；两侧轮次各自独立。连击在同一拍连续结算两次，两次独立选目标，第一次击杀目标后第二次重新选目标；攻击者第一次攻击后阵亡则第二次不进行；学者被授予连击时同样连击两次，攻击力仍为 0。复生保留本轮剩余行动次数，召唤与复生按插入位置决定本轮是否轮到。
- `BattleField::apply_damage(id, amount, can_kill)`（`server/crates/rule-engine/src/combat.rs`）是唯一的战斗掉血入口，只扣 `stats.health.current`；`deal_damage` 是同义包装。攻击主伤害与反击伤害都走它。`can_kill = false` 时最多把当前生命降到 1，`can_kill = true` 时允许降到 0 或更低，由死亡判定按 `current <= 0` 处理。
- 战斗棋盘与招募棋盘同容量：`BattleField::has_room`（`alive_count(side) < catalog.config.board_slots`，v0.1 为 7）是所有进场路径的唯一闸门——召唤效果（含左邻插入与追加）、`Copy` 到棋盘、死亡结算的播种召唤、复生，容量不足时一律作废：不扣牌池、不生成实例编号、不触发 `OnAllySummon`、不增加 `summon_count`。阵亡随从留在向量里但不计数。
- 同一时点的进场顺序固定为**死亡 → 召唤 → 复生**：死亡先把 `alive` 置 false，存活数立即减一；亡语与播种召唤占用空出的位置；复生最后检查容量，被召唤占满时保持阵亡（`reborn_used` 照常置 true，本场不再尝试）。
- `append` / `insert_at` 返回 `Option<InstanceId>`，容量不足返回 `None`，调用方据此跳过召唤计数与联动触发。写入 `CombatAction::Summon.position` 的是 `alive_rank`——该随从在**存活随从序列**中的下标（已阵亡条目不计数），值域由 `board_slots` 决定；向量下标仍用于维持阵亡锚点的顺序。
- 复生直接调用 `MinionStats::revive_to(1)`，把 `current` 置为 1，允许大于层和；这是 `current` 正向偏离层和的唯一入口，不复用 `apply_damage`。
- `SideBoard` 不再持有独立生命数组，战斗内所有当前生命读写都落在 `stats.health.current`。
- `CombatLog` 除动作序列外记录与动作同序同长的 `states`：每条动作执行后受影响随从的快照 `MinionSnapshot { instance_id, attack, health, alive }`——`attack` 只记录主动攻击这一条，带攻方与受方两个快照，反击伤害已并入快照（不再单独记录反击动作）；`death`/`reborn`/`contagion`/`summon` 各带一个。召唤动作 `CombatAction::Summon { minion, card_id, golden, seat, position }` 记录插入位置，`position` 是该随从在存活随从序列中的下标（阵亡条目不计数，上界由 `board_slots` 决定），召唤物不在战前快照中，位置只从该字段读取；它的那条快照带召唤瞬间的实际攻防（含光环折叠与战斗内加成），房间服务用它填展示数值。
- 战后 `write_back_persistent`（`server/crates/rule-engine/src/settle.rs`）只回写 `contagion` 与 `stats.*.growth`；持久棋盘上的随从按层和重算当前值（满血进入下一回合），死亡不改变持久棋盘，战斗伤害、回合效果与光环都不回写。
- 进入结算时 `clear_combat_duration` 调用 `clear_turn` 清除攻防的 `turn` 层，同时保留 `granted` 的时长清理与禁攻规则。
- 幽灵战（`settle.rs::resolve_ghost`）的幽灵来自 `state.ranking`（全部出局者，最近出局在前）：每回合用 `Stream::Matchmaking` 随机抽一名，可能连续抽到同一人；没有出局者时幽灵为空阵容，落单座位仍产生配对与战报。幽灵取其最终棋盘（死亡时的真实数值，保留成长与永久增益），并按幽灵自己的光环来源（`player_auras`）折叠一次；不结算英雄被动、不产生手牌、不写回牌池与持久棋盘。若幽灵与己方棋盘实例编号重号，`renumber_ghost` 只在本次模拟的克隆上改号。双方同回合清场判平局；幽灵获胜时落单座位按伤害公式扣血，落单座位获胜时计入 `last_won`/`last_winner`。
- 光环只经 `fold_auras` 折叠进克隆：把 `state.auras` 的派生值写入克隆的 `aura` 层并保留「光环失效不致死」的夹紧，持久实例的 `aura` 层始终为 0。
- `hash.rs` 编码 `stats` 各分量（含 `current`）与 `is_scholar`；编码变化会改变状态哈希，同代码重放仍保持确定。
