# 最优攻击卡池迁移为无视嘲讽 · 技术方案

> 状态：方案未实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 实体文档 | 12 张随从、5 个技能、卡牌索引与技能列表改为 `ignore_taunt` |
| 数据与校验 | 删除 `optimal_attack` 的遗留兼容；`KW_UNLOCK` 改为 `ignore_taunt: 3` |
| 规则引擎 | 不改；`ignore_taunt` 的选目标逻辑已实现 |
| 协议与客户端 | 不改代码；重新生成文本与客户端数据 |
| 基线 | 重新生成金标准并按迁移后的对局结果比对 |

不涉及：卡牌攻防、星级、时代与玩法的数值调整；客户端界面改版；新的战斗规则。

## 2. 实体文档与数据

- `docs/versions/v0.1/product/content/skills/keyword-optimal-attack.md` 改名为 `keyword-ignore-taunt.md`，front matter 的 `id` 与 `keyword` 改为 `keyword-ignore-taunt` / `ignore_taunt`，正文与属性表同步。
- 12 张随从的 front matter `skills`、属性表「关键词」与「效果」文本把 `keyword-optimal-attack` / 最优攻击换成 `keyword-ignore-taunt` / 无视嘲讽。
- 4 个授予技能的实体文档与结构化效果：
  - `alternating-current`、`combat-optimal-attack`、`aura-mech-optimal-attack`、`trigger-mech-optimal-attack-this-turn`。
  - `effects` 中 `grant_keyword` 的 `keyword` 改为 `ignore_taunt`；技能 id 是否改名按文档主题决定，若改名则同步卡牌引用。
  - 技能文本改为「获得无视嘲讽」或「你的机械随从获得无视嘲讽」。
- `docs/versions/v0.1/product/content/skills/README.md`、`cards/README.md` 与 `card-system.md` 的关键词统计同步。
- `scripts/gen_card_data.py` 重新生成 `server/data/v0.1/`；`scripts/gen_client_data.py` 与 `scripts/gen_text.py` 重新生成客户端数据与文本。

## 3. 清理遗留兼容

- `server/crates/rule-engine/src/effects/validate.rs`：从 `KEYWORD_BASES` 删除 `optimal_attack`。
- `scripts/i18n_terms.py`：从 `KEYWORD_BASES` 删除 `optimal_attack`，删除 `LEGACY_KEYWORD_LABELS`。
- `scripts/validate_docs.py`：
  - `_KEYWORD_NAMES` 删除 `optimal_attack`。
  - `keyword_labels()` 不再合并遗留标签。
  - `KW_UNLOCK` 把 `optimal_attack: 3` 改为 `ignore_taunt: 3`。
  - `keyword_cost` 删除 `optimal_attack` 分支，保留 `ignore_taunt` 的 2 点。
- 删除 `server/crates/rule-engine/tests/combat.rs` 的 `legacy_optimal_attack_is_inert`，因为该 id 已不存在；`ignore_taunt` 的战斗用例保留。

## 4. 测试与基线

| 文件 | 调整 |
| --- | --- |
| `server/crates/rule-engine/tests/combat.rs` | 删除遗留词用例；保留 `ignore_taunt_can_target_non_taunt` 与嘲讽强制用例 |
| `server/crates/rule-engine/tests/effects_exec.rs` | 拟态用例已使用 `ignore_taunt`，无需再改 |
| `server/crates/sim/tests/golden/seed-1-games-20.json` | 迁移后重新生成 20 局哈希；若结果变化则重定，并更新 `sim_determinism.rs` 注释 |
| Python 测试 | `test_validate_effects`、`test_structured_effects` 的关键词期望同步为 `ignore_taunt` |

验证命令：

```bash
make data
make data-check
make m1-check
make client-check
```

## 5. 文档回写

- 迁移完成后删除本 `update/` 目录，产品与技术结论已进入版本基线。
- `docs/current/technical/rule-engine-and-simulator.md` 删除遗留 `optimal_attack` 的说明。
- `docs/current/` 不新增页面或接口说明。

## 6. 风险

- **平衡变化**：16 张卡失去击杀优先，胜率可能下降；迁移后用真实对局数据复核，必要时调整数值或折算值。
- **金标准重定**：迁移会改变卡池与战斗结果，必须重新生成并确认重放一致性。
- **技能改名引用**：若技能 id 改名，所有卡牌引用、文档索引与生成物必须同批更新，避免悬空引用。
