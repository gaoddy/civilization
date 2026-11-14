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
      src/intent.rs          # 九种意图与 11 个拒绝码
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
      tests/sim_cli.rs
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
| `make m0-determinism` | 同种子同意图序列重放用例（`sim_cli::replay_same_log_twice_has_same_hash`） |
| `make m0-batch` | 1000 局 8 AI 批量对局（seed 1），写入 `output/m0-batch.json` |
| `make m0-report` | 从批量日志生成基线报告 |
| `python3 scripts/validate_docs.py` | 文档与实体数据校验 |

模拟器直接调用：`sim play --seed N`、`sim batch --games N --seed N --out PATH`、`sim replay --log PATH`、`sim report --log PATH`；参数错误或模拟失败以非零码退出。

## 5. 验证方式

验证矩阵：`make data-check && python3 scripts/validate_docs.py && make test && make lint && make m0-determinism && make m0-batch && make m0-report`。

| M0 验证标准 | 判定依据 |
| --- | --- |
| 卡池完整 | `validate_docs.py` 退出码 0；`catalog_load::loads_all_committed_data` 断言 220 卡 / 118 技能 / 13 英雄卡全部加载 |
| 数据一致 | `make data-check` 重新生成无差异 |
| 规则覆盖 | `server/crates/rule-engine/tests/` 与 `server/crates/sim/tests/` 的规则用例清单 |
| 确定性 | `make m0-determinism`：重放 100 次哈希一致 |
| 稳定性 | `make m0-batch` 1000 局退出码 0，无异常、无超时 |
| 可判定 | 1000 局全部输出 1–8 名排名 |
| 数值基线 | `make m0-report` 输出基线报告；越界项登记 [待确认事项](../../versions/v0.1/open-questions.md) |

数值基线的实测越界项登记在 v0.1 的待确认事项中。
