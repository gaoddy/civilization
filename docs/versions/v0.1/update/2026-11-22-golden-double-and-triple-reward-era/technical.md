# 金色效果翻倍与三连奖励星级（技术）

状态：已实施（2026-11-22）

## 1. 规则引擎（`server/crates/rule-engine`）

### 1.1 金色数值翻倍

- `Effect` 增加 `doubled_outputs()`：在 `Params` 层只翻倍产出字段，条件 / 阈值字段原样保留。
  - `Growth`：`times`、`times_if.times` ×2；`condition` 不变。
  - `ModifyStats`：`attack`、`health`、`per_attack`、`per_health` ×2；`max_count` 与 `condition` 不变。
  - `GrantKeyword`：`keyword.value` / `value` ×2；`condition` 不变。
  - `DealDamage.amount`、`Summon.count`、`AddShopSlot.count`、`Salvage.count`、`Revive.attack/health` ×2。
  - `Detonate.amount` ×2，`threshold` 不变；`Convert.count` ×2，`threshold` 不变。
  - `Latent.amount`、`Latent.count` ×2，`threshold` / `when` 不变；`Devour.value` 不变。
  - `SevenVoyages.turns_max`、`Discover.choose` 与无数字原语不变。
- `apply_op` 与 `apply_combat_effect` 在 `ctx.golden` 为真时先取 `doubled_outputs()` 再结算。
- 删除三处「金色执行两遍」循环：`run_effects`、`fire_death_recruit`、`fire_combat_effects`。
- 分解：`apply_combat_decompose` 的 `amount` 在 `ctx.golden` 时 ×2（若未通过 `Params` 翻倍）。
- 繁衍 `apply_breed` 与播种召唤数量保留现有 ×2 实现，不叠加第二次翻倍。
- 光环：`player_auras` 对金色来源的 `attack` / `health` / `per_attack` / `per_health` 贡献 ×2；`max_count`、`condition` 与群聚计数不变。

### 1.2 三连奖励星级

- `grant_triple_reward`：`level = player.era + 1`，`min(6)`；移除来源卡 `level` 依赖。
- `HandCard::Effect.reward_level` 继续保存获得时的锁定星级，视图角标与出牌取卡逻辑不变。

### 1.3 测试

- 更新「金色触发两次」类断言为「单次结算 + 数值 ×2」（战吼、响应者、分解、回合开始、潜伏）。
- 新增：金色光环 ×2、金色繁衍 ×2、金色回合开始 +0/+2、金色潜伏阈值不变且伤害 ×2、金色分解 ×2、金色条件阈值不翻倍。
- 三连奖励：时代 3 → 4 星；升时代不追溯；6 星封顶。
- 重定模拟器 golden 基线（三连奖励星级与金色结算改变随机流）。

## 2. 文案生成（`scripts/`）

- 模板 front matter 增加可选 `text_golden`；技能实体增加可选 `text_golden`。
- `Renderer.text(call, golden=True)`：金色状态下自动把数字占位符（`int` / `count` / 关键词层数）翻倍。
- 条件 / 阈值模板用 `text_golden` 显式覆盖，新增占位修饰符 `.x2`：金色状态下取原值 ×2，普通文案取原值。
- `instance_text(doc, golden=True)` 生成金色文案；`gen_text.py` 输出 `skill.<id>.text.golden`。
- 显式 `text` 的技能在文档内补 `text_golden`。
- `skill-system.md` 补充 `text_golden` 与 `.x2` 的字段说明。

## 3. 客户端

- `CardDB.display_text(id, golden)` / `_skill_text(skills, golden)` 优先读取 `skill.<id>.text.golden`，缺键回落普通文案。
- 手牌卡面（`HandCardFace` → `CardFaceBase.add_info`）、手牌 tooltip、棋子详情（`MinionDetailSection`）按实例 `golden` 展示金色文案。
- 棋盘卡面不显示介绍，不改；商店与图鉴卡不可能为金色，保持普通文案。
- 冒烟用例覆盖：金色手牌卡面显示 `+2` 文案。

## 4. 文档回写

- `product/rules/core-rules.md` §6：合成与奖励口径。
- `product/rules/keywords-and-mechanics.md`：金色「触发两次」改为「数值产出 ×2，条件 / 阈值不变」。
- `product/overview/design-overview.md`、`game-intro.md` 同步口径。
- `product/content/skill-system.md`：`text_golden` 字段。
