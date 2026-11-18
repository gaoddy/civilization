# 玩法扩充收尾：9 条待办落实报告

- 日期：2026-11-18
- 基线：`develop` @ `8037405`（玩法扩充合并点）
- 分支：`feature/playstyle-followups`
- 范围：`docs/superpowers/plans/2026-11-18-playstyle-card-expansion.md` 完成后遗留的 9 条待办

## 1. 待办逐项结论

### 1. 客户端潜行标识

- 结论：已实现。`GameCardFace.face()` 新增 `stealth` 参数，角标行右侧渲染「潜」状态胶囊（`StealthBadge`），与金色 `★` 分列两端；招募棋盘与战斗回放共用同一卡面（`read_only_minion_card` 与 `_minion_card` 都传入 `minion.stealth`）。
- 数据来源：`PlayerView.board[].stealth`（`MinionView`）与战斗回放 `ownBoard`/`opponentBoard`（`board_minion_from` 由折叠后的随从重新计算 `has_stealth`）。
- 文案：新增人工 key `ui.game.card.stealth_glyph=潜`。
- 测试：`client/tests/ui_smoke.gd::_check_board_stealth_badge`（有潜行显示、无潜行不显示）。
- 遗留：战斗快照 `CombatMinionState` 不含 `stealth`，战斗中打破潜行后徽记要等下一次棋盘视图刷新才消失。

### 2. `Salvaged` 日志丢 `card_id`

- 结论：已实现。`messages.schema.json` 的 `CombatResultActionsItem` 增加可选 `cardId`（无卡牌载荷的动作省略，向后兼容）；`filter.rs` 的 `Salvaged` 映射携带 `card_id`，其余 13 个构造点补 `None`。
- 生成物：`protocol/generated/protocol.rs` 与协议文档同步重新生成；`make protocol-check` / `protocol-rs-check` 通过。
- 测试：`room-service/tests/filter.rs::combat_result_maps_salvage` 增加 `cardId` 断言。
- 遗留：回放界面尚未展示「获得哪张牌」（不渲染手牌），本项只保证协议可消费。

### 3. 战斗回放不消费新动作

- 结论：已实现。`combat_playback.gd::_apply` 按服务端快照字段补全：`mimic`/`devour` 写来源与目标、`poison`/`detonate`/`convert` 写目标、`revive`/`recall` 与 `death`/`reborn` 同口径（状态在 `target`、侧别在 `sourceSide`）；`salvage` 只影响手牌，回放不动作。
- 测试：`client/tests/ui_smoke.gd::_check_combat_playback_new_actions` 覆盖 7 类动作的攻防/存活落地。
- 遗留：`convert` 召唤出的 Token 依赖独立的 `summon` 动作呈现，回放本身不额外生成。

### 4. 引爆阈值无卡池使用者

- 结论：确认为引擎预留能力，不改代码。已在产品 §3.11、技术 §3.11 与 `detonate-1` / `detonate-2` 技能文档注明：`threshold` 为预留参数，卡池暂无使用者；引擎测试保留，后续卡牌可直接启用。
- 无新增测试（阈值消灭、护盾不阻止阈值消灭的引擎测试此前已存在）。

### 5. 同层 `armor N` 不叠加

- 结论：已实现完整叠加，而非只统计重复次数。
- 方案：
  1. `combat::armor_value` 不再读取去重后的关键词集合，改为分别统计卡面关键词的逐次出现（新增 `base_keyword_occurrences`，`base_keywords` 改为其去重视图）与 `granted` 中的每条记录；
  2. `apply_grant_keyword` 的 `armor N` 分支把层数合并进已有的 `armor N` 条目（同名重复不再被 `record_grant` 丢弃），时长取较高层级；
  3. 光环 `accumulate` 把多个来源的同层减伤合并成一条 `armor N`（关键词集合按字符串去重，原先会互相覆盖）。
- 代价：`armor_value` 从集合遍历改为逐条遍历，复杂度仍是 O(关键词数)，无额外分配；`granted` 中 armor 由「多条同名记录」变为「单条累加记录」，哈希与视图关键词更稳定。产品/技术口径同步为「数值相同也叠加」。
- 测试：`combat_damage.rs::armor_stacks_same_layer_from_base_and_granted`、`effects_exec.rs::grant_armor_stacks_repeated_same_layer`、`auras.rs::aura_armor_stacks_across_sources`。
- 说明：该改动未改变模拟基线（第 9 条重定后的金标准在本次改动后仍通过），因为卡池内没有同层 armor 叠加的既有对局路径。

### 6. `Salvage` 池硬编码机械

- 结论：已实现按载体玩法取池。`apply_combat_salvage` 先解析 `ctx.source` 的卡牌（回退 Token）玩法，再按该玩法筛选卡池；机械携带时行为与之前一致。
- 文档：产品 §3.15「随机池按载体随从的玩法筛选」、技术 §3.15 同步。
- 测试：`turn_effects.rs::salvage_pool_follows_carrier_playstyle`（哺乳纲载体的随机牌仍为哺乳纲，而非机械）。
- 遗留：数据校验仍限定 `Salvage` 只能出现在机械卡上（归属校验未放开）；本项只修正引擎取池口径。

### 7. 幽灵侧 `Recalled` 动作

- 结论：已实现。`settle.rs` 幽灵战路径在丢弃幽灵侧 `hand_minions` 后，以保留的回手实例编号为准同步剔除 `Recalled` 动作及其并行状态快照（动作与状态按同下标删除），幽灵回放不再出现无对应手牌变化的回手。
- 测试：`death_effects.rs::ghost_recall_is_not_reported_as_combat_action`。
- 说明：己方回手必定存在对应 `hand_minions` 记录；手牌已满导致未记录动作的路径不受影响。

### 8. Token 命名

- 结论：已实现。`tokens/README.md` 注册表将 `termite-token` 更名为「白蚁（2/2）」，`make data` 重新生成文本表；`token.ammonite-token.name=菊石幼体` 等其它 Token 名称无漂移。
- 测试：`make data-check`（`gen_text.py --check` 文本表无漂移）。

### 9. `fire_delayed_battlecry` 重放缺陷

- 结论：已实现每个实例只结算一次。
- 机制：`MinionInstance` 新增 `delayed_battlecry_used`；`fire_delayed_battlecry` 在确认存在敌方选择器的 `OnPlay` 效果后、执行前置位（避免效果内再入重复结算）；战斗结束经 `write_back_persistent` 以「或」写回持久棋盘。回手产生的手牌副本与 `copy_of_catalog` 副本视为新入场，重置该标记。
- 口径：标记按实例持久化，因此同一随从在后续战斗不再重放；`OnCombatStart` 卡牌不读取该标记，行为不变。
- 代价：`state_hash` 新增一个布尔字段，`combat.rs` 与 `sim/tests/golden/seed-1-games-20.json` 两处金标准重定（模拟流程未变，仅哈希编码变化）；两处注释已记录重定原因。
- 测试：`settle.rs::delayed_battlecry_runs_once_across_combats`（首战击杀、第二战零动作）。

## 2. 新增与加强的测试

服务端：

- `room-service/tests/filter.rs::combat_result_maps_salvage`（加强：`cardId`）
- `rule-engine/tests/death_effects.rs::ghost_recall_is_not_reported_as_combat_action`
- `rule-engine/tests/settle.rs::delayed_battlecry_runs_once_across_combats`
- `rule-engine/tests/turn_effects.rs::salvage_pool_follows_carrier_playstyle`
- `rule-engine/tests/combat_damage.rs::armor_stacks_same_layer_from_base_and_granted`
- `rule-engine/tests/effects_exec.rs::grant_armor_stacks_repeated_same_layer`
- `rule-engine/tests/auras.rs::aura_armor_stacks_across_sources`

客户端：

- `client/tests/ui_smoke.gd::_check_board_stealth_badge`
- `client/tests/ui_smoke.gd::_check_combat_playback_new_actions`

金标准：

- `server/crates/sim/tests/golden/seed-1-games-20.json`（第 4 次重定，原因写入 `sim_determinism.rs`）
- `server/crates/rule-engine/tests/combat.rs` 的 `state_hash` 常量与注释

## 3. 验证输出摘要

| 命令 | 结果 |
| --- | --- |
| `make m0-check` | EXIT 0（data-check、`cargo test --workspace`、fmt+clippy、确定性重放均通过） |
| `make data-check` | EXIT 0，审计通过，文本表无漂移 |
| `make protocol-check` | EXIT 0，样例校验通过，gdscript/文档无漂移 |
| `make protocol-rs-check` | EXIT 0 |
| `make client-check` | EXIT 0（边界、i18n、Godot 导入） |
| `make client-smoke` | EXIT 0，6 个场景 `UI_SMOKE_OK`，0 个 `SCRIPT ERROR` |
| `make m1b-e2e` | EXIT 0，`M1B_E2E_OK`（8 人局至 GameOver） |

## 4. 遗留与裁定建议

1. 战斗快照不含 `stealth` 与 `cardId` 之外的「手牌获得」展示：潜行徽记在战斗内被打破后不会即时消失，零件回收获得的牌在回放中没有表现。若产品要求战斗中实时反馈，需要扩展 `CombatMinionState`（例如增加 `stealth` 布尔）或在客户端渲染一条获得提示；建议作为独立小需求评估。
2. `Salvage` 的归属校验仍只允许机械：本次只把引擎池改为跟随载体，数据校验层没有放开非机械携带。若后续要让该技能通过光环/复制落到其它流派，需要同时调整 `validate_docs.py` 的白名单。
3. 引爆 `threshold` 为预留能力：当前无卡使用，规则与测试已保留。若下个版本仍无卡牌启用，可考虑在数据校验中禁止该参数，避免误配。
4. 延迟战吼标记会改变 `state_hash`：任何跨版本读取旧对局日志的比对工具需要注意本次基线重定；仓库内两处金标准已同步。
5. `fire_delayed_battlecry` 在「首次战斗无有效敌方目标」时同样消耗标记（视为已结算）。当前卡池的延迟战吼都指向敌方随从，实际对局不会出现空敌方棋盘；若产品希望「未结算成功不消耗」，需要改为按效果是否命中回写，代价是引入效果结果回传。
