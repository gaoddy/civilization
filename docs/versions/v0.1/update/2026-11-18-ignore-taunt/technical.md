# 移除最优攻击并新增无视嘲讽 · 技术方案

> 状态：方案未实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 规则引擎 | `pick_target` 删除 `optimal_attack` 的击杀优先与绕嘲讽分支；新增 `ignore_taunt` 分支；遗留 `optimal_attack` 仍可被解析但不产生选目标效果 |
| 数据与校验 | 引擎与脚本的关键词词表同时接受遗留 `optimal_attack` 与新 `ignore_taunt`；生成文本保留 `optimal_attack` 的展示名 |
| 协议 | 无字段与生成物变化；关键词继续以字符串下发 |
| 房间服务 | 无代码改动 |
| 客户端 | 无代码改动；`ignore_taunt` 的展示文本进入 `zh_CN.csv`，旧卡仍显示「最优攻击」 |
| 卡池 | 16 张旧卡与 5 个相关技能的数据、卡面文档本轮不改 |

不涉及：卡牌攻防与星级调整、客户端界面改版、历史战斗回放重算、潜行与嘲讽的既有判定、伤害与死亡结算。

## 2. 规则引擎

`server/crates/rule-engine/src/combat.rs`：

- 删除 `OPTIMAL_ATTACK` 常量与 `pick_target` 中的击杀优先分支。
- 新增 `IGNORE_TAUNT` 常量，值为 `ignore_taunt`。
- `pick_target` 的候选池按以下顺序构造：
  1. 仍排除潜行随从；全部潜行时沿用现有最左侧兜底。
  2. 攻击者关键词包含 `ignore_taunt` 时，候选池为全部可选中随从。
  3. 否则筛出可选中嘲讽；嘲讽非空时只用嘲讽，为空时用全部可选中随从。
- 随机数仍只走 `rng.below(Stream::Combat, pool.len())`，池子组成变化会改变同种子的战斗结果，必须重定固定种子基线。
- `optimal_attack` 不再被任何分支读取；携带该遗留 id 的随从在选目标时与没有关键词等价。

`resolve_attack`、伤害、反击、护盾、传染、剧毒、击杀归属、死亡结算与行动轮次均不改。

## 3. 数据与校验

### 3.1 引擎词表

`server/crates/rule-engine/src/effects/validate.rs`：

- `KEYWORD_BASES` 增加 `ignore_taunt`。
- 保留 `optimal_attack`，加注释「卡池迁移前的遗留 id，卡池迁移后删除」。

### 3.2 文档校验与生成脚本

- `scripts/i18n_terms.py`：`KEYWORD_BASES` 增加 `ignore_taunt`；保留 `optimal_attack`；新增 `LEGACY_KEYWORD_LABELS = {"optimal_attack": "最优攻击"}`。
- `scripts/validate_docs.py`：
  - `_KEYWORD_NAMES` 与 `KEYWORD_BASES` 增加 `ignore_taunt`，保留 `optimal_attack`。
  - `keyword_labels()` 在解析判定表后合并 `LEGACY_KEYWORD_LABELS`，保证旧卡在生成文档与文本表中仍显示「最优攻击」。
  - `keyword_cost` 给 `ignore_taunt` 按 2 点折算；保留 `optimal_attack` 的 2 点口径，直到卡池迁移。
  - `KW_UNLOCK` 保留 `optimal_attack: 3`，保证现有卡池的最小星级校验继续通过；本轮不登记 `ignore_taunt`，否则在没有任何卡牌使用时 `check_keyword_unlock` 会失败。卡池迁移时把该项替换为 `ignore_taunt: 3`。
- `scripts/gen_text.py`：无需改逻辑；`keyword_labels()` 合并遗留标签后会自动生成 `enum.keyword.ignore_taunt` 与 `enum.keyword.optimal_attack`。

### 3.3 生成物

- `make data` 重新生成 `server/data/v0.1/`、`client/generated/` 与 `client/text/zh_CN.csv`。
- 卡牌与技能实体文档本轮不改，`cards.json` / `skills.json` 应无内容变化；`zh_CN.csv` 新增 `enum.keyword.ignore_taunt`，并保留 `enum.keyword.optimal_attack`。
- `make data-check` 必须通过，确认生成物与实体文档一致。

## 4. 协议与客户端

- `protocol/schema/v1/messages.schema.json`、`protocol/generated/protocol.rs` 与 GDScript 生成物无变化。
- 房间服务按字符串透传关键词，无需改动。
- 客户端按 `enum.keyword.<id>` 取展示名；新关键词自动可显示。没有卡牌使用 `ignore_taunt`，本轮不会出现在实际对局卡面上。

## 5. 测试调整

| 文件 | 调整 |
| --- | --- |
| `server/crates/rule-engine/tests/combat.rs` | 删除 `optimal_attack_prefers_killable`；新增「带 `ignore_taunt` 可以选中非嘲讽目标」与「遗留 `optimal_attack` 仍被嘲讽强制」两条用例；保留 `taunt_forces_target` 与潜行兜底用例 |
| `server/crates/rule-engine/tests/effects_exec.rs` | 拟态用例里的光环关键词从 `optimal_attack` 改为 `ignore_taunt`，避免继续用已废弃词做关键词复制断言 |
| `server/crates/sim/tests/sim_determinism.rs` | 重新生成 `golden/seed-1-games-20.json` 的 20 局哈希，并在基线注释中写明本次变化原因 |

`ignore_taunt` 绕过嘲讽的用例不依赖单次随机结果：用固定的多组种子模拟，断言至少有一局首次攻击选中非嘲讽目标；带遗留 `optimal_attack` 的用例则断言首次攻击只能选中嘲讽，直接钉住旧行为已失效。

验证命令：

```bash
make data
make data-check
make test
make lint
make m0-determinism
```

## 6. 文档回写

- 已完成的产品基线：`keywords-and-mechanics.md` §1、§7 与相关示例；`core-rules.md` §5.2；`design-overview.md`、`game-intro.md`、`economy-and-numbers.md`、`eras-and-scenarios.md`、`card-system.md` 的关键词列表。
- 实现后更新 `docs/current/technical/rule-engine-and-simulator.md`：关键词 id 示例改为 `ignore_taunt`，并在战斗章节补充选目标顺序。
- 卡面文档、技能文档与卡牌索引留到卡池迁移时同步；本轮在 `product.md` 的过渡状态中保留。
- 平衡影响已登记 `docs/versions/v0.1/open-questions.md`。

## 7. 风险

- **卡面与效果不一致**：16 张旧卡仍显示「最优攻击」，但战斗中无效果。这是本轮明确接受的过渡状态，卡池迁移后消除。
- **基线重定范围**：只有战斗结果与随机流受影响，重放确定性与 100 次重放校验仍必须通过。
- **校验空窗**：`ignore_taunt` 的时代解锁与最小星级在卡池接入前不进入 `KW_UNLOCK`，接入时必须补上并让校验通过。
