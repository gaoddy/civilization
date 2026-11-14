# 规则引擎与模拟器

> 状态：已实现（v0.1 M0）
> 代码：`server/crates/rule-engine`、`server/crates/sim`、`scripts/`、`Makefile`

本文记录 v0.1 M0 已实现的规则引擎与模拟器的现状：工程结构、引擎边界、数据生成、命令与验证方式；不描述产品规则与后续里程碑规划。

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
      src/state.rs           # GameState / PlayerState / MinionInstance
      src/data.rs            # Catalog 与加载期校验（deny_unknown_fields）
      src/effects/           # DSL 解析（parse）、解释执行（exec）、光环（auras）
      src/intent.rs          # 十种意图与 11 个拒绝码
      src/engine.rs          # Engine::new / apply / start_game / start_turn
      src/recruit.rs         # 商店、买/卖/刷新/冻结/升时代/打出/结束招募
      src/triple.rs          # 三连与发现
      src/combat.rs          # 战斗模拟
      src/settle.rs          # 配对、finish_recruit、伤害、淘汰、排名
      src/rng.rs             # xoshiro256++ 与五个标签流
      src/hash.rs            # FNV-1a 规范编码与状态哈希
      tests/                 # 11 个集成测试文件
    sim/                     # 可执行：CLI 模拟器 + 确定性 AI
      src/lib.rs             # run_games / play_game / replay
      src/ai.rs              # 确定性 AI 策略
      src/report.rs          # 日志与基线报告
      src/main.rs            # play / batch / replay / report
      tests/sim_cli.rs       # CLI 冒烟与字段契约
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

依赖方向固定为 `sim → rule-engine`；`rule-engine` 不依赖 `sim`、tokio、sqlx、协议 crate，不做文件 I/O，不读时钟与环境变量。

## 2. 引擎边界

- `Engine::apply(state, seat, intent)` 是唯一的状态变更入口，返回 `Outcome`（本意图产生的事件）；拒绝时返回 `Reject { code, message }` 且不改变状态。
- 拒绝码与核心规则 §3 一致：`NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY`、`NO_SPACE`、`INVALID_STAGE`、`NO_SUCH_MINION`、`ALREADY_MAX_ERA`、`CARD_NOT_IN_HAND`、`HERO_POWER_LIMIT`、`NO_VALID_TARGET`、`TARGET_NOT_ALLOWED`、`INVALID_CHOICE`。
- 九种招募意图：`BuyMinion`、`SellMinion`、`RerollShop`、`UpgradeEra`、`PlayCard`、`RearrangeBoard`、`FreezeShop`、`UseHeroPower`、`EndRecruit`；发现选择 `ChooseDiscover` 是引擎内部的第十种意图，不属于协议意图。
- 阶段不匹配时，`recruit::apply` 对全部招募意图（含 `UpgradeEra`、`RerollShop` 等）统一返回 `INVALID_STAGE`；`EndRecruit` 另有「本座位已结束招募」时仍处于 `Recruit`、返回成功且无副作用。
- `BuyMinion`、`PlayCard`、`UseHeroPower` 带 `target: Option<InstanceId>`，M0 由 AI 自动选择目标。
- `EndRecruit` 只结束本座位；全员（未出局）结束后才进入 `Combat` 并发出 `PhaseChanged`，重复 `EndRecruit` 成功且无副作用。
- `settle::finish_recruit` 幂等：`Recruit` 阶段（超时路径）生成配对并切 `Combat`；`Combat` 阶段只补齐一次配对，不重复发事件、不重复配对。
- 阶段推进由驱动方按固定顺序调用 `finish_recruit` / `resolve_combat` / `resolve_settle` / `start_turn`，引擎不自行推进时间。
- `rule-engine` 顶部 `#![forbid(unsafe_code)]`；结算只用整数，容器用 `BTreeMap` 与有序 `Vec`。
- M0 不产出 `PlayerView`；视图与协议映射在 M1 建立。

## 3. 卡牌数据生成

- 唯一数据源：`docs/versions/v0.1/product/content/` 下的卡牌与技能实体文档；`scripts/gen_card_data.py` 复用 `validate_docs.py` 的 front matter 解析。
- 经济与对局配置由生成器从产品数值文档口径抄录到 `config.json`（初始生命、棋盘/手牌/商店槽位、回合上限、伤害上下限、升时代费用、星级牌份数、商店概率、金币曲线）。
- 输出确定性 JSON（UTF-8、两空格缩进、键排序、末尾换行）；生成物随源提交。
- 引擎用 `include_str!` 嵌入 `server/data/v0.1/*.json`，加载期校验 `deny_unknown_fields`、重复 id、悬空技能引用等，未知字段或未知 DSL 组合直接失败。
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
| `make m0-report` | 从批量日志生成基线报告（字段口径见第 6 节） |
| `python3 scripts/validate_docs.py` | 文档与实体数据校验 |

模拟器直接调用：`sim play --seed N`、`sim batch --games N --seed N --out PATH`、`sim replay --log PATH`、`sim report --log PATH`；参数错误或模拟失败以非零码退出。

## 5. 验证方式

验证矩阵：`make data-check && python3 scripts/validate_docs.py && make test && make lint && make m0-determinism && make m0-batch && make m0-report`。

每个 M0 标准类别由下列命令与字段支撑；表中「报告字段」指 `make m0-report` 输出的 JSON 字段。

| M0 标准 | 命令 | 证据 |
| --- | --- | --- |
| 卡池完整 | `make data-check`、`python3 scripts/validate_docs.py`、`make test` | 命令退出码 0；`catalog_load::loads_all_committed_data` 断言 220 卡 / 118 技能 / 3 Token / 13 英雄卡全部加载，`every_card_skill_exists`、`unknown_field_fails`、`effects_parse::every_skill_in_catalog_parses`；`audit_effects.py` 输出 `审计通过` |
| 数据一致 | `make data-check` | 重新生成后输出 `生成数据无漂移`，退出码 0 |
| 规则覆盖 | `make test` | `server/crates/rule-engine/tests/` 与 `server/crates/sim/tests/` 的用例全部 `test result: ok`（用例清单见第 7 节） |
| 确定性 | `make m0-determinism` | `replay_same_log_100_times_has_same_hash` 通过：100 次重放每次退出码 0，`final_hash` 全部等于原局 |
| 稳定性 | `make m0-batch` | 退出码 0，无错误输出；产物 `output/m0-batch.json` 的 `games` 数为 1000（每局一条日志） |
| 可判定 | `make m0-batch`、`make m0-report` | 每局日志 `ranking` 含 8 个座位、名次 1–8 连续（`sim_cli::single_game_produces_ranking_and_hash`、`batch_of_ten_all_ranked` 固化该契约）；报告 `games` = 1000 |
| 数值基线 | `make m0-report` | 退出码 0；`wins_by_tribe`、`win_rate_permille_by_tribe`、`avg_turns`、`era_upgrade_turns_by_seat`、`avg_era_upgrade_turn`、`triples_per_game`、`games_reaching_turn_20`、`survivors_at_turn_20`、`avg_damage_per_loss` 均有值；越界与口径问题登记 [待确认事项](../../versions/v0.1/open-questions.md) |

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
| `survivors_at_turn_20` | 上述对局在第 20 回合招募开始前的平均存活人数，向下取整；无此类局为 `null` |
| `games_ended_on_turn_20` | 结束回合为 20 的局数 |
| `avg_survivors_at_end` | 各局结束时的存活人数之和 / 局数，向下取整；当前规则下第 20 回合收束（或决出唯一幸存者）时已无存活玩家，实测为 0 |
| `avg_damage_per_loss` | 每次战斗失败的伤害之和 / 败者次数，向下取整 |
| `final_hashes` | 每局最终状态哈希（16 位小写十六进制，按局序） |

`sim play` 的单局日志 `stats` 含 `era_upgrade_turns`（8 个座位各自的首次升时代回合，未升为 `null`）、`survivors_at_turn_20`、`survivors_at_end`、`damage_per_loss`、`winner_seat`、`winner_tribe`。

## 7. 规则用例清单

`make test` 运行的测试文件即规则用例清单：

| 规则章节 | 用例文件 |
| --- | --- |
| 核心规则 §2 阶段状态机、§7 淘汰排名、§8 边界 | `rule-engine/tests/settle.rs`、`recruit.rs` |
| 核心规则 §3 九种意图与 11 个失败码 | `rule-engine/tests/recruit.rs` |
| 核心规则 §4 招募细则 | `rule-engine/tests/recruit.rs`、`choices.rs` |
| 核心规则 §5 战斗与伤害公式 | `rule-engine/tests/combat.rs` |
| 核心规则 §6 三连与发现 | `rule-engine/tests/triple.rs`、`choices.rs` |
| 关键词机制 §1–§3 | `rule-engine/tests/turn_effects.rs`、`death_effects.rs`、`effects_exec.rs`、`auras.rs`、`combat.rs` |
| 关键词机制 §9、§10 | `rule-engine/tests/effects_parse.rs`、`effects_exec.rs` |
| 时代与数值 | `rule-engine/tests/recruit.rs`、`settle.rs`、`turn_effects.rs` |
| 确定性 | `rule-engine/tests/determinism.rs`、`sim/tests/sim_determinism.rs` |
| 卡池加载 | `rule-engine/tests/catalog_load.rs` |
