# Skill.effects 结构化设计

> 状态：待用户确认
> 任务分支：`feature/structured-skill-effects`

## 1. 目标

把技能定义里 `effects` 从「一段 DSL 字符串」改为「effect 数组」：引擎加载后直接得到 `Vec<Effect>`，运行期不再解析字符串。

`Skill` 仍是技能定义的核心结构；本次只改变 `effects` 的表示，不改变任何游戏规则。

## 2. 已确认的取舍

讨论过三种数组元素形态：

| 方案 | 做法 | 结论 |
| --- | --- | --- |
| A. 完全结构化对象 | 每个元素是 `{trigger, op, selector, params}`，`Vec<Effect>` 由 serde 反序列化 | **采用** |
| B. DSL 字符串数组 | 每个元素仍是一段 DSL，只是去掉 `;` 分隔 | 未采用：仍要运行期解析，没有类型安全 |
| C. 半结构化 | `trigger`/`op` 结构化，`selector`/`params` 保留字符串 | 未采用：两套语法并存 |

结构化落点也确认过：

| 方案 | 做法 | 结论 |
| --- | --- | --- |
| A1. 仅生成物与引擎结构化 | 源文档继续写 DSL；生成脚本把它编译成结构化 JSON | **采用** |
| A2. 源文档也结构化 | 改 131 个技能文档，并扩展自研 front-matter 解析器 | 未采用：改动面与风险最大 |
| A3. 源文档也结构化 + PyYAML | 同上，引入第三方依赖 | 未采用：项目当前无第三方 Python 依赖 |

因此：**DSL 是作者格式，结构化 JSON 是编译产物。** `docs/current/product/content/skills/*.md` 不变；`validate_docs.py` 的 DSL 校验保留，但参数与语义规则随引擎对齐（`DealDamage.canKill`、移除未实现的 `source`）。

## 3. 数据模型

### 3.1 目标 JSON

`server/data/v0.1/skills.json` 的一条技能：

```json
{
  "id": "division-of-labor",
  "name": "分工",
  "keyword": null,
  "text": "使一个友方人类随从获得一次成长；若你控制至少 3 个人类，改为两次",
  "effects": [
    {
      "trigger": "on_activate",
      "op": "growth",
      "selector": {
        "kind": "side",
        "side": "friendly",
        "pick": "single",
        "filters": [{ "key": "tribe", "value": "人类" }]
      },
      "params": {
        "times": 1,
        "times_if": { "tribe": "人类", "times": 2 },
        "condition": {
          "side": "friendly",
          "filter": { "key": "tribe", "value": "人类" },
          "compare": "ge",
          "count": 3
        }
      }
    }
  ],
  "cost": 1,
  "uses_per_turn": 1,
  "match_text": "同左"
}
```

没有效果的技能写 `"effects": []`。

### 3.2 枚举取值

所有枚举在 JSON 中用 `snake_case`：

| Rust 类型 | JSON 取值 |
| --- | --- |
| `Trigger` | `on_play`、`on_play_effect`、`on_use`、`on_activate`、`on_turn_start`、`on_first_refresh`、`on_combat_start`、`on_deal_damage`、`on_death`、`on_friendly_death`、`on_enemy_death`、`on_ally_summon`、`on_ally_growth`、`on_ally_evolve`、`on_combat_end`、`aura`、`conditional_aura` |
| `Op` | `growth`、`modify_stats`、`grant_keyword`、`remove_growth`、`cannot_attack`、`deal_damage`、`summon`、`copy`、`discover`、`refresh_shop`、`add_shop_slot`、`gain_minion` |
| `Side` | `friendly`、`enemy`、`both` |
| `Pick` | `single`、`all`、`random`、`leftmost`、`leftmost_adjacent`、`top_attack` |
| `Duration` | `instant`（缺省）、`permanent`、`combat` |
| `Zone` | `hand`、`board`、`shop` |
| `Compare` | `ge`、`le`、`eq`、`gt`、`lt` |
| `CardType` | 沿用中文取值 `随从`、`效果` |

### 3.3 子结构

```rust
pub struct Effect {
    pub trigger: Trigger,
    pub op: Op,
    pub selector: Selector,
    pub params: Params,
}

#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Selector {
    #[serde(rename = "self")]
    SelfCard,
    Target,
    Side { side: Side, pick: Pick, #[serde(default)] filters: Vec<Filter> },
}

pub struct Filter { pub key: String, pub value: String }

pub struct Condition {
    pub side: Side,
    pub filter: Filter,
    pub compare: Compare,
    pub count: i32,
}

pub struct TimesIf { pub tribe: Tribe, pub times: i32 }

pub struct KeywordSpec { pub keyword: String, pub value: Option<i32> }
```

`Params` 保持扁平，字段名与现有语义一一对应：

```json
{
  "times": 1,
  "count": 1,
  "amount": 3,
  "attack": 2,
  "health": 2,
  "value": 1,
  "extra": 1,
  "duration": "permanent",
  "can_kill": false,
  "health_only": true,
  "times_if": { "tribe": "人类", "times": 2 },
  "condition": { "...": "见上" },
  "when": { "key": "tribe", "value": "人类" },
  "keyword": { "keyword": "繁衍", "value": 2 },
  "token": "synthetic-cell-token",
  "card": "baiyi",
  "to": "hand",
  "from": "shop",
  "card_type": "随从",
  "pool": "tribe",
  "choose": 1,
  "free": true,
  "era": "current",
  "level": "current"
}
```

规则：

- 所有结构 `deny_unknown_fields`，未知字段在加载期报错。
- 缺省字段不写；`Params`、`Selector::Side.filters`、`KeywordSpec.value` 均带 `#[serde(default)]`。
- `duration` 缺省为 `instant`；生成器只在显式出现 `duration` 时输出。
- `KeywordSpec` 由元组枚举改为具名字段，执行器从 `KeywordSpec::Keyword(name, layer)` 改为 `spec.keyword` / `spec.value`。
- `times_if` 由 `(String, i32)` 改为具名结构，`tribe` 使用 `Tribe` 枚举。

## 4. 校验

分两层，与现有 `Catalog::from_json` 的初始化路径一致：

1. **结构校验（serde）**：未知枚举、未知字段、类型不符、重复字段由 `serde_json` 拒绝，错误归入 `CatalogError::File("skills.json", err)`。
2. **语义校验（新增 `effects/validate.rs`）**：对每个 `Effect` 执行现有 DSL 解析器里的跨字段规则，错误归入 `CatalogError::Rule`：
   - `Aura` / `ConditionalAura` 只能用 `modify_stats` / `grant_keyword`。
   - 每个 `op` 的必填参数与可选参数集合（沿用现有 `op_spec`）。
   - `times_if` 只能与 `growth` 的 `times` 同时出现。
   - `when` 只能用于事件类触发，集合与 `scripts/audit_effects.py` 的 `WHEN_TRIGGERS` 对齐：`on_enemy_death`、`on_friendly_death`、`on_ally_summon`、`on_ally_growth`、`on_ally_evolve`、`on_deal_damage`、`on_combat_end`、`on_use`、`on_death`。
   - `summon` 必须且只能有 `token` / `card` 之一。
   - `discover` 必须有 `pool` 与 `choose`，且 `choose == 1`。
   - `gain_minion` 必须有 `level` 与 `to`，`from` 只接受 `shop`。
   - 筛选键只能是 `tribe` / `subtype` / `name` / `type` / `other`；`tribe`、`type`、`other` 的取值域校验。
   - `grant_keyword` 的关键词必须在关键词表内；`繁衍` / `装配` / `传染` 必须带层数，其余关键词禁止层数。

`effects/parse.rs` 保留为 DSL 作者格式的工具（测试与人工对照用），引擎加载与执行不再调用它。它和 `validate.rs` 共享参数约束表，避免两套规则漂移。

## 5. 生成链路

`scripts/gen_card_data.py` 新增 `structured_effects(value)`：

- 复用现有 `parse_effects` 做语法与语义校验；
- 复用 `audit_effects.check_effect` 校验跨字段规则，并补上只在 Rust 侧的 `GainMinion.from` 只接受 `shop`；生成期与 Rust `validate.rs` 必须同规则，避免 `make data` 通过但服务端加载失败；
- 把 `trigger` / `op` 映射为 `snake_case`；
- 把 `selector` 解析为 `self` / `target` / `side` 对象；
- 把 `params` 的值解析成整数、布尔、枚举、`times_if`、`condition`、`when`、`keyword` 等结构；
- 解析失败时抛异常，生成命令退出非 0。

`scripts/gen_card_data.py` 的 `skill_entry` 调用 `structured_effects`。`scripts/gen_client_data.py` 复用 `skill_entry`，因此 `client/generated/skills.json` 同步变成结构化数组；客户端当前不读取 `effects`，无需改 `.gd`。

`scripts/audit_effects.py` 仍审计源文档 DSL，不改。

## 6. 兼容性

- `skills.json` 是编译期 `include_str!` 的生成物，不是持久化用户数据，也不是对外协议；**不保留旧字符串格式的兼容分支**。
- 协议（`protocol/`）不携带 `effects`，不变。
- 客户端只读 `text` / `cost` 等展示字段，`effects` 形态变化不影响渲染。
- 数据结构版本仍为 `v0.1`，不新建数据目录。

## 7. 改动文件

服务端：

- `server/crates/rule-engine/src/data.rs`
- `server/crates/rule-engine/src/effects/mod.rs`
- `server/crates/rule-engine/src/effects/validate.rs`（新增）
- `server/crates/rule-engine/src/effects/parse.rs`（适配新类型）
- `server/crates/rule-engine/src/effects/exec.rs`
- `server/crates/rule-engine/src/effects/aura.rs`
- `server/crates/sim/src/ai.rs`
- `server/crates/rule-engine/tests/*`、`server/crates/room-service/tests/game.rs`

脚本与生成物：

- `scripts/validate_docs.py`（OP_SPEC 与布尔取值对齐）
- `scripts/audit_effects.py`（关键词内联层数规则）
- `scripts/gen_card_data.py`
- `server/data/v0.1/skills.json`
- `client/generated/skills.json`
- `tests/test_gen_card_data.py`、`tests/test_structured_effects.py`

文档：

- `docs/current/technical/rule-engine-and-simulator.md`
- `docs/current/technical/client.md`（如涉及 `skills.json` 描述）
- 本设计文档

不改：

- `docs/current/product/content/skills/*.md`
- `protocol/`、`room-service` 业务逻辑、客户端 `.gd`

## 8. 测试与验收

新增/调整：

- Rust：结构化 JSON 反序列化成功；未知枚举/未知字段/类型错误被拒；语义规则错误被拒；真实 `skills.json` 全部通过。
- Python：`structured_effects` 对单段、多段、空效果、各类参数与错误输入的输出；生成结果中 `effects` 为列表且字段类型正确。
- 行为不变：现有引擎、招募、战斗、光环、发现/复制/进化测试全部保持通过；`sim_determinism::batch_final_hashes_match_golden_baseline` 以改造前 20 局最终哈希为固定基线。

验证命令：

```bash
make data          # 重新生成数据
make data-check    # 生成物无漂移
make test          # Python unittest + cargo test --workspace
make lint          # fmt + clippy -D warnings
make audit
make m0-determinism
make client-check  # 客户端生成物与 Godot 导入
```

判定标准：以上命令退出码均为 0；`m0-determinism` 复现哈希与改动前一致，证明数据表示变化没有改变游戏行为。

## 9. 不做的事

- 不改任何技能文本、数值或触发规则。
- 不把源文档改成嵌套 YAML。
- 不引入新依赖。
- 不新增 `v0.2` 数据目录或迁移脚本。
- 不重构与 `effects` 无关的引擎代码。
