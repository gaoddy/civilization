# 卡牌参数重排与玩法差异化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 按 13 个玩法重排 309 张随从的身材与技能组合，把效果牌从 37 张扩到 100 张，并让 1000 局模拟的每个玩法夺冠率进入 4%–12%。

**Architecture:** 先建立平衡报告与校验口径（可独立运行、中间批次不误报），并修好模拟器 AI「只买不打效果牌」的问题，再按玩法分三批重排随从身材与技能，随后新增 63 张效果牌与配套技能及定向购买过滤，最后做调参与金标准重定。卡牌文档是唯一数据源，生成物一律由 `make data` 重出。

**Tech Stack:** Python（文档校验、数据生成、一次性迁移脚本）、Rust（rule-engine、sim）、JSON 生成物、Godot 文本表。

**Spec:**
- 产品规则：[product.md](../../versions/v0.1/update/2026-11-18-card-differentiation/product.md)
- 技术方案：[technical.md](../../versions/v0.1/update/2026-11-18-card-differentiation/technical.md)

## Global Constraints

- 只修改卡牌文档的 `attack`、`health`、`skills`，以及新增效果牌与技能；不改 `id`、卡名、`level`、`era`、`taxon`、`subtype`、`is_scholar`、`is_hero`。
- 不手工编辑 `server/data/v0.1/*.json` 与 `client/text/zh_CN.csv`；每次内容改动后跑 `make data`。
- 不新增引擎原语、关键词、触发时机；不修改协议、房间服务与客户端代码。
- 关键词最小星级不得前移：`KW_UNLOCK = {breed:1, taunt:2, windfury:3, optimal_attack:3, contagion:3, reborn:4, divine_shield:5, poison:3, armor:2, stealth:3}`。
- 每张随从满足 `attack + health + keyword_cost ≤ CAP[level]`（4/6/9/13/18/22/29 白板 +4：8/10/13/17/22/26/33）。
- 随机只用既有确定性随机源；不引入 `thread_rng`。
- 每个任务结束前 `make data-check` 与 `make test` 必须通过；Task 2–5 结束前按技术方案 §5.3 重定模拟金标准。
- 文档与提交信息使用简体中文；提交按任务拆分。
- Task 3–5 期间整表平衡校验必然失败，只用 `scripts/balance_report.py --check --playstyle …` 检查本批玩法；整表检查留到 Task 7。

## Review Focus

- 定向效果牌是 `neutral` 卡，但关键词/原语/选择器校验必须按 `play:` 关联的玩法判定，否则「毒牙」「播种季」「俯冲编队」会被误判越权。
- 给 5–7 星卡加技能时，不得让 `reborn`、`divine_shield`、`poison`、`stealth`、`windfury` 等关键词出现在低于 `KW_UNLOCK` 的星级。
- 多技能会累加关键词折算；加第二个/第三个技能后必须下调身材，否则触发「超模」。
- 模拟器 AI 目前只买不打效果牌；不补「打出」步骤时，100 张效果牌会堵死手牌，模拟结果不可用。
- 效果牌按星级占比超过 30%、总数不等于 100、定向/通用数量不匹配时，`make balance-check` 必须失败。

---

### Task 0: 平衡报告脚本与测试

**Files:**
- Create: `scripts/balance_report.py`
- Create: `tests/test_balance_report.py`
- Modify: `Makefile`（新增 `balance-check`，并加入 `.PHONY`）

**Interfaces:**
- Produces: `TARGETS: dict[str, tuple[float, float]]`，13 个玩法 + `neutral` 的攻血比目标带。
- Produces: `playstyle_stats(cards: list[dict]) -> dict[str, dict]`，每项含 `count`、`attack_share`、`by_level`。
- Produces: `skill_density(cards: list[dict]) -> dict[str, float]`，键为 `"3-4"`、`"5-7"`。
- Produces: `effect_stats(cards: list[dict]) -> dict`，含 `total`、`by_level`、`by_cost`、`level_share`、`targeted`、`generic`。
- Produces: `violations(cards: list[dict], playstyles: list[str] | None = None) -> list[str]`。
- Produces: CLI `python3 scripts/balance_report.py [--check] [--json] [--playstyle ID ...] [--cards PATH]`；`--check` 有越界时退出码 1。

- [ ] **Step 1: 写失败测试**

创建 `tests/test_balance_report.py`：

```python
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import balance_report as br  # noqa: E402


def card(cid, playstyle, level, attack, health, skills=(), card_type="minion", cost=None, specialty=()):
    return {
        "id": cid, "playstyle": playstyle, "level": level, "attack": attack,
        "health": health, "skills": list(skills), "card_type": card_type,
        "cost": cost, "specialty": [{"kind": "play", "value": s[5:]} for s in specialty],
        "is_scholar": False, "is_hero": False,
    }


class BandTest(unittest.TestCase):
    def test_inside_tolerance_passes(self):
        cards = [card(f"m{i}", "mammal", 3, 4, 5) for i in range(10)]
        self.assertEqual(br.violations(cards, ["mammal"]), [])

    def test_outside_tolerance_fails(self):
        cards = [card(f"m{i}", "mammal", 3, 6, 5) for i in range(10)]
        errors = br.violations(cards, ["mammal"])
        self.assertTrue(any("mammal" in e for e in errors))

    def test_spread_requires_020(self):
        cards = [card(f"m{i}", "mammal", 3, 4, 5) for i in range(10)]
        cards += [card(f"p{i}", "plant", 3, 3, 7) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("攻血比差" in e for e in errors))


class SkillDensityTest(unittest.TestCase):
    def test_three_four_star_needs_half(self):
        cards = [card(f"a{i}", "mammal", 3, 4, 5, ["x", "y"] if i < 4 else ["x"]) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("3–4 星" in e for e in errors))

    def test_five_seven_star_needs_eighty_percent(self):
        cards = [card(f"b{i}", "mammal", 5, 8, 10, ["x", "y"] if i < 7 else ["x"]) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("5–7 星" in e for e in errors))


class EffectTest(unittest.TestCase):
    def test_total_must_be_100(self):
        cards = [card(f"e{i}", "neutral", 3, 0, 0, ["s"], "effect", 2) for i in range(99)]
        errors = br.violations(cards)
        self.assertTrue(any("效果牌总数" in e for e in errors))

    def test_level_share_cap(self):
        cards = [card(f"m{i}", "mammal", 1, 1, 3) for i in range(10)]
        cards += [card(f"e{i}", "neutral", 1, 0, 0, ["s"], "effect", 1) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("占比" in e for e in errors))

    def test_targeted_count(self):
        cards = [card(f"e{i}", "neutral", 3, 0, 0, ["s"], "effect", 2,
                      ["play:fish"] if i < 38 else []) for i in range(100)]
        errors = br.violations(cards)
        self.assertTrue(any("定向效果牌" in e for e in errors))

    def test_three_targeted_cards_per_playstyle(self):
        cards = [card(f"e{i}", "neutral", 3, 0, 0, ["s"], "effect", 2, ["play:fish"])
                 for i in range(39)]
        cards += [card(f"g{i}", "neutral", 3, 0, 0, ["s"], "effect", 2) for i in range(61)]
        errors = br.violations(cards)
        self.assertTrue(any("fish" in e and "3 张" in e for e in errors))
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest discover -s tests -p 'test_balance_report.py' -v`
Expected: FAIL，`ModuleNotFoundError: No module named 'balance_report'`。

- [ ] **Step 3: 实现 `scripts/balance_report.py`**

`TARGETS` 逐项抄录产品 §3：

```python
TARGETS = {
    "mammal": (0.42, 0.46), "bird": (0.50, 0.55), "insect": (0.48, 0.54),
    "fish": (0.43, 0.48), "reptile": (0.28, 0.33), "cnidarian": (0.28, 0.33),
    "plant": (0.26, 0.31), "fungus": (0.35, 0.40), "protozoa": (0.42, 0.47),
    "cephalopod": (0.39, 0.44), "bacteria": (0.31, 0.36), "virus": (0.34, 0.39),
    "machine": (0.42, 0.47), "neutral": (0.42, 0.47),
}
TOLERANCE = 0.04
MIN_SPREAD = 0.20
EFFECT_LEVELS = {1: 10, 2: 18, 3: 22, 4: 20, 5: 16, 6: 9, 7: 5}
EFFECT_COSTS = {0: 8, 1: 33, 2: 34, 3: 19, 4: 6}
EFFECT_SHARE_CAP = 0.30
TARGETED = 39
GENERIC = 61
```

实现要点：

- 统计集合 = `card_type == "minion"` 且 `is_scholar` 为假；学者攻击力为 0，不参与攻血比。
- `attack_share = sum(attack) / sum(attack + health)`，分母为 0 时记为 `0.0`。
- `by_level[level]` 同样按该玩法该星级求和。
- `skill_density`：`"3-4"` 的分子是 `level in (3,4)` 且 `len(skills) >= 2` 的卡，分母是该区间全部非学者随从；`"5-7"` 同理。
- `effect_stats.level_share[level] = 该星级效果牌数 / 该星级全部卡牌数`；`targeted_by_playstyle` 统计每个玩法被多少张定向牌关联。
- `violations` 的顺序：先按传入 `playstyles`（默认全部 14 项）检查目标带，再检查攻血比差、技能密度、效果牌总数/星级/费用/占比/定向与通用数量/每个玩法定向牌数（13 个玩法各 3 张）。`playstyles` 非空时只做目标带检查，跳过攻血比差与效果牌检查。
- `--check` 打印每条违规并返回 1；无违规打印摘要并返回 0。
- `--json` 输出顶层键：`playstyles`（玩法统计）、`spread`（最高与最低攻血比差）、`skill_density`、`effects`、`violations`。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest discover -s tests -p 'test_balance_report.py' -v`
Expected: PASS（9 个用例）。

- [ ] **Step 5: 加 Makefile 入口**

在 `.PHONY` 中加入 `balance-check`，并新增：

```make
balance-check:
	$(PYTHON) scripts/balance_report.py --check
```

Run: `make balance-check`
Expected: 以当前 37 张效果牌的数据运行，退出码 1 并输出效果牌数量/分布违规（这是预期的，Task 6 后归零）。

- [ ] **Step 6: 提交**

```bash
git add scripts/balance_report.py tests/test_balance_report.py Makefile
git commit -m "feat(balance): 新增玩法身材与效果牌平衡报告"
```

---

### Task 1: 校验口径、技能数量上限与基础卡修补

**Files:**
- Modify: `scripts/validate_docs.py`
- Modify: `docs/versions/v0.1/product/content/cards/juelei.md`
- Modify: `docs/versions/v0.1/product/content/cards/sea-anemone.md`
- Modify: `docs/versions/v0.1/product/content/cards/shuishengjue.md`
- Modify: `docs/versions/v0.1/product/content/cards/nitaizhangyu.md`
- Modify: `docs/versions/v0.1/product/content/cards/shizuniao.md`
- Modify: `docs/versions/v0.1/product/content/cards/sibadayongshi.md`
- Modify: `docs/versions/v0.1/product/content/cards/tufu.md`
- Modify: `docs/versions/v0.1/product/content/cards/xiake.md`
- Modify: `docs/versions/v0.1/product/content/cards/yuanshijixiekuilei.md`
- Create: `tests/test_validate_balance_rules.py`
- Create: `server/crates/rule-engine/tests/pool_filter.rs`

**Interfaces:**
- Produces: `effective_playstyle(doc) -> str`；效果牌带 1 条 `play:<id>` 时返回该 id，否则返回 `doc["playstyle"]`。
- Produces: `skill_count_errors(doc, skill_map) -> list[str]`。
- Produces: `card_in_pool` 的单元测试（不新增函数）。

- [ ] **Step 1: 写失败测试**

创建 `tests/test_validate_balance_rules.py`：

```python
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import validate_docs as vd  # noqa: E402


class EffectivePlaystyleTest(unittest.TestCase):
    def test_effect_uses_play_association(self):
        doc = {"card_type": "effect", "playstyle": "neutral", "specialty": ["play:fish"]}
        self.assertEqual(vd.effective_playstyle(doc), "fish")

    def test_effect_without_association_is_neutral(self):
        doc = {"card_type": "effect", "playstyle": "neutral", "specialty": []}
        self.assertEqual(vd.effective_playstyle(doc), "neutral")

    def test_minion_keeps_own_playstyle(self):
        doc = {"card_type": "minion", "playstyle": "bird", "specialty": ["play:fish"]}
        self.assertEqual(vd.effective_playstyle(doc), "bird")


class SkillCountTest(unittest.TestCase):
    def setUp(self):
        self.skill_map = {
            "k1": {"keyword": "taunt"},
            "k2": {"keyword": "windfury"},
            "k3": {"keyword": "poison"},
            "p": {},
        }

    def test_one_two_star_allows_one(self):
        doc = {"level": 2, "playstyle": "mammal", "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2"]}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("最多 1 个技能" in e for e in errors))

    def test_three_four_star_allows_two(self):
        doc = {"level": 4, "playstyle": "mammal", "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2"]}
        self.assertEqual(vd.skill_count_errors(doc, self.skill_map), [])

    def test_five_seven_star_max_two_keywords(self):
        doc = {"level": 6, "playstyle": "mammal", "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2", "k3", "p"]}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("最多 2 个登记关键词" in e for e in errors))

    def test_skillless_tool_card_allowed(self):
        doc = {"level": 3, "playstyle": "neutral", "taxon": "life-medical",
               "is_scholar": False, "skills": []}
        self.assertEqual(vd.skill_count_errors(doc, self.skill_map), [])

    def test_skillless_non_tool_rejected(self):
        doc = {"level": 2, "playstyle": "bird", "taxon": "archaeopterygidae",
               "is_scholar": False, "skills": []}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("无技能" in e for e in errors))
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest discover -s tests -p 'test_validate_balance_rules.py' -v`
Expected: FAIL，`AttributeError: module 'validate_docs' has no attribute 'effective_playstyle'`。

- [ ] **Step 3: 实现校验改动**

在 `scripts/validate_docs.py`：

1. 新增 `effective_playstyle(doc)`：`card_type == "effect"` 且 `specialty` 中恰好一条 `play:` 时返回该值，否则 `doc.get("playstyle")`。
2. `classification_errors` 的效果牌分支改为：`specialty` 允许空列表或恰好 1 条 `play:<PLAYSTYLE_IDS>`；仍禁止 `prey:`/`host:`；仍要求 `playstyle == neutral`、`taxon == phenomenon`。
3. `keyword_errors` 与 `mechanism_errors` 把 `doc.get("playstyle")` 换成 `effective_playstyle(doc)`。
4. 新增 `skill_count_errors(doc, skill_map)`：星级 ≤2 上限 1，3–4 上限 2，5–7 上限 3；5–7 统计 `skill_map[sid].get("keyword")` 非空的技能，超过 2 条报错；无技能时，只有 `playstyle == neutral`、`is_scholar` 为假、`taxon in CRAFT_TOOL_TAXA` 才允许。
5. 在 `check_cards` 的随从循环里调用 `skill_count_errors`；`skillless != 7` 改为 `!= 1`，提示改为「无技能随从 {n} 张，期望 1 张（仅中立工具卡）」。

- [ ] **Step 4: 合并 3 张 2 星双技能卡**

按技术方案 §2.2，同时更新 front matter 的 `skills`、「## 属性」的关键词/技能行、「## 效果」正文：

| 卡牌 | 保留 | 移除 |
| --- | --- | --- |
| `juelei` | `keyword-breed-2` | `keyword-sow` |
| `sea-anemone` | `keyword-armor-1` | `keyword-taunt` |
| `shuishengjue` | `keyword-breed-1` | `keyword-sow` |

「## 效果」只保留保留技能的文本，不留拼接句。

- [ ] **Step 5: 给 6 张无技能随从补 1 个既有技能**

| 卡牌 | 技能 |
| --- | --- |
| `nitaizhangyu` | `mimic-1` |
| `shizuniao` | `dive-1` |
| `sibadayongshi` | `keyword-taunt` |
| `tufu` | `battlecry-friendly-plus-0-plus-1` |
| `xiake` | `keyword-windfury` |
| `yuanshijixiekuilei` | `keyword-assemble-1` |

同步 front matter、属性表的「关键词」「技能」行，以及「## 效果」正文（取该技能文档的 `text`）。

- [ ] **Step 6: 运行测试确认通过**

Run: `python3 -m unittest discover -s tests -p 'test_validate_balance_rules.py' -v`
Expected: PASS。

Run: `make data && make data-check`
Expected: EXIT 0；卡牌仍为 309 随从 / 37 效果牌，技能仍为 197。

- [ ] **Step 7: 写引擎过滤测试**

创建 `server/crates/rule-engine/tests/pool_filter.rs`：

```rust
use std::collections::BTreeSet;

use rule_engine::data::{card_in_pool, Card, CardType, Playstyle, Specialty, SpecialtyKind};

fn card(playstyle: Playstyle, specialty: Vec<(&str, &str)>) -> Card {
    Card {
        id: "test-card".into(),
        card_type: CardType::Effect,
        playstyle,
        taxon: "phenomenon".into(),
        specialty: specialty
            .into_iter()
            .map(|(kind, value)| Specialty {
                kind: match kind { "play" => SpecialtyKind::Play, "prey" => SpecialtyKind::Prey, _ => SpecialtyKind::Host },
                value: value.into(),
            })
            .collect(),
        subtype: None,
        level: 3,
        era: None,
        attack: None,
        health: None,
        skills: vec![],
        is_scholar: false,
        is_hero: false,
        cost: Some(2),
    }
}

fn enabled(list: &[Playstyle]) -> BTreeSet<Playstyle> {
    list.iter().copied().collect()
}

#[test]
fn effect_with_play_association_follows_enabled_playstyle() {
    let c = card(Playstyle::Neutral, vec![("play", "fish")]);
    assert!(!card_in_pool(&c, &enabled(&[Playstyle::Bird])));
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Fish])));
}

#[test]
fn generic_effect_is_always_available() {
    let c = card(Playstyle::Neutral, vec![]);
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Bird])));
    assert!(card_in_pool(&c, &enabled(&[])));
}

#[test]
fn playstyle_minion_requires_its_playstyle() {
    let mut c = card(Playstyle::Bird, vec![]);
    c.card_type = CardType::Minion;
    assert!(!card_in_pool(&c, &enabled(&[Playstyle::Fish])));
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Bird])));
}
```

Run: `cd server && cargo test -p rule-engine --test pool_filter -v`
Expected: PASS（3 个用例）。

- [ ] **Step 8: 提交**

```bash
make data-check
cd server && cargo test --workspace
cd ..
git add scripts/validate_docs.py tests/test_validate_balance_rules.py \
  server/crates/rule-engine/tests/pool_filter.rs \
  docs/versions/v0.1/product/content/cards/{juelei,sea-anemone,shuishengjue,nitaizhangyu,shizuniao,sibadayongshi,tufu,xiake,yuanshijixiekuilei}.md
git commit -m "feat(cards): 放开效果牌玩法关联并收紧技能数量校验"
```

---

### Task 2: 模拟器 AI 打出效果牌

**Files:**
- Modify: `server/crates/sim/src/ai.rs`
- Test: `server/crates/sim/src/ai.rs`（`#[cfg(test)] mod tests` 内新增）
- Modify: `server/crates/sim/tests/golden/seed-1-games-20.json`
- Modify: `server/crates/sim/tests/sim_determinism.rs`（重定注释）

**Interfaces:**
- Produces: `choose_play_effect(player: &PlayerState, catalog: &Catalog, blocked: &[Intent]) -> Option<Intent>`。

先修「只买不打」：当前 AI 把效果牌买进手牌后从不打出，37 张时已影响经济，100 张时会堵死手牌。必须在重排调参前修好，否则批次 A–C 的胜率没有参考价值。

- [ ] **Step 1: 写失败测试**

在 `ai.rs` 测试模块追加（使用既有卡，不依赖 Task 6 的新牌）：

```rust
#[test]
fn plays_effect_card_from_hand() {
    let catalog = embedded_catalog().expect("内置卡表应能加载");
    let (_, mut state) = Engine::new(catalog.clone(), [7; 32], rule_engine::playstyles::draw(7));
    state.phase = Phase::Recruit;
    state.turn = 1;
    state.players[0].board = vec![MinionInstance::new(&catalog, 1, "saman", false)];
    state.players[0].hand = vec![rule_engine::state::HandCard::effect(20, "aolinpike-jidian")];
    assert_eq!(
        choose_play_effect(&state.players[0], &catalog, &[]),
        Some(Intent::PlayCard { hand_index: 0, position: None, target: None })
    );
}

#[test]
fn effect_play_falls_back_to_leftmost_target() {
    let catalog = embedded_catalog().expect("内置卡表应能加载");
    let (_, mut state) = Engine::new(catalog.clone(), [7; 32], rule_engine::playstyles::draw(7));
    state.phase = Phase::Recruit;
    state.turn = 1;
    state.players[0].board = vec![MinionInstance::new(&catalog, 1, "saman", false)];
    state.players[0].hand = vec![rule_engine::state::HandCard::effect(20, "faraday-cage")];
    let plain = Intent::PlayCard { hand_index: 0, position: None, target: None };
    assert_eq!(
        choose_play_effect(&state.players[0], &catalog, &[plain]),
        Some(Intent::PlayCard { hand_index: 0, position: None, target: Some(1) })
    );
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p sim plays_effect_card_from_hand effect_play_falls_back_to_leftmost_target -v`
Expected: FAIL，`choose_play_effect` 不存在。

- [ ] **Step 3: 实现 `choose_play_effect`**

```rust
fn choose_play_effect(
    player: &PlayerState,
    catalog: &Catalog,
    blocked: &[Intent],
) -> Option<Intent> {
    let leftmost = player.board.first().map(|minion| minion.instance_id);
    for (index, hand) in player.hand.iter().enumerate() {
        let Some(card) = catalog.cards.get(hand.card_id()) else { continue };
        if card.card_type != CardType::Effect {
            continue;
        }
        let hand_index = u8::try_from(index).ok()?;
        let plain = Intent::PlayCard { hand_index, position: None, target: None };
        if !blocked.contains(&plain) {
            return Some(plain);
        }
        if let Some(target) = leftmost {
            let targeted = Intent::PlayCard { hand_index, position: None, target: Some(target) };
            if !blocked.contains(&targeted) {
                return Some(targeted);
            }
        }
    }
    None
}
```

在 `plan` 的候选数组中、`choose_fill_board` 之后插入 `choose_play_effect(player, catalog, blocked)`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p sim plays_effect_card_from_hand effect_play_falls_back_to_leftmost_target -v`
Expected: PASS。

- [ ] **Step 5: 重定模拟金标准**

AI 行为变化会改变对局，必须重定：

Run: `cd server && cargo run -q -p sim -- batch --games 20 --seed 1 --out ../output/golden20-ai.json`

在仓库根目录重写金标准：

```bash
python3 - <<'PY'
import json, pathlib
src = json.load(open('output/golden20-ai.json'))
dst = {'seed': 1, 'games': 20, 'hashes': [g['final_hash'] for g in src['games']]}
path = pathlib.Path('server/crates/sim/tests/golden/seed-1-games-20.json')
path.write_text(json.dumps(dst, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
PY
```

在 `sim_determinism.rs` 注释追加「模拟器 AI 打出效果牌」。

- [ ] **Step 6: 全量校验并提交**

Run: `make data-check && make test && make lint`
Expected: EXIT 0。

```bash
git add server/crates/sim
git commit -m "feat(sim): AI 打出效果牌"
```

---

### Task 3: 重排批次 A（哺乳纲 + 中立工具卡）

**Files:**
- Create（不提交）: `output/rebalance.py`
- Create（不提交）: `output/roles-batch-a.json`
- Modify: 哺乳纲与非学者中立随从的卡牌文档（按脚本输出）

**Interfaces:**
- Produces: `derive(level: int, share: float, total: int, keyword_cost: float) -> tuple[int, int]`。
- Produces: `apply_batch(playstyles: list[str], roles: dict[str, str], skill_additions: dict[str, list[str]], overrides: dict[str, tuple[int, int]]) -> int`。
- 脚本命令：`python3 output/rebalance.py plan --playstyles mammal neutral --out output/roles-batch-a.json`、`python3 output/rebalance.py apply --roles output/roles-batch-a.json`。

- [ ] **Step 1: 写一次性重排脚本**

`output/rebalance.py` 要求：

- 解析 `docs/versions/v0.1/product/content/cards/*.md` 的 front matter 与「## 属性」表。
- 角色来源：优先 `output/role-overrides.json`（`{"card_id": "tank|output|balanced|function"}`）；否则按设计说明的「定位：」与技能关键词推断：含减伤/复苏/嘲讽/繁衍/生命成长 → tank；含俯冲/连击/最优攻击/高攻击 → output；含光环/战吼成长/召唤/分裂 → function；其余 balanced。
- **英雄卡跳过**：`is_hero` 为真的卡不参与批量重排，保持现有身材；`plan` 另列「英雄偏离表」，列出攻血比超出本玩法目标带的英雄，由人工在 `output/role-overrides.json` 中单独指定数值。
- 攻血比：tank 取玩法目标带下沿，output 取上沿，balanced 取中值，function 取中值 −0.01。
- 强势期窗口（来自产品 §3）：`mammal 2–5`、`bird 4–7`、`insect 1–4`、`fish 2–6`、`reptile 3–7`、`cnidarian 4–7`、`plant 5–7`、`fungus 3–6`、`protozoa 3–6`、`cephalopod 2–6`、`bacteria 4–7`、`virus 5–7`、`machine 2–7`、`neutral 2–7`。
- 总身材：`base = {1:4,2:6,3:9,4:13,5:18,6:22,7:29}[level]`；强势期内 +1、强势期外 −1（1 星不取负）；`total = min(base + tempo, base + 4 - keyword_cost)`。`keyword_cost` 直接复用 `scripts/validate_docs.py` 的 `keyword_cost(card, skill_map)`，避免两套口径。计算顺序必须是：先定最终技能列表（含 `skill-additions`）→ 用该列表算 `keyword_cost` → 再定 `total` 与攻血；不得用旧身材或输出反推折算。
- 若 `base + 4 - keyword_cost` 低于该星级最小身材和（`RANGE[level]` 的攻击下限 + 生命下限），脚本必须报错并列出该卡：这组技能无法在折算上限内成立，要换技能组合，不能硬压身材。
- 分配：`attack = round(total * share)`，按 `RANGE[level]` 的攻/血上下限夹取，`health = total - attack`。
- 写回：front matter 的 `attack`/`health`；「## 属性」的攻击力/生命值行；技能变更时同步「关键词」「技能」行与「## 效果」；「## 效果」直接调用 `validate_docs.rendered_text(doc, skill_map)` 生成，避免多技能拼接口径不一致；不动「## 设计说明」「## 出处」。
- `apply` 同时读取可选的 `output/skill-additions.json`（`{"card_id": ["skill-id", ...]}`），把技能追加到该卡并重渲染「## 效果」。
- `plan` 只输出角色表与建议数值，不写文件到 cards；`apply` 才写。

- [ ] **Step 2: 生成角色表并人工复核**

Run: `python3 output/rebalance.py plan --playstyles mammal neutral --out output/roles-batch-a.json`

检查输出中的角色分布：哺乳纲必须同时存在 tank/output/balanced/function 四类；中立 28 张按工具卡定位，多数为 balanced。把与设计说明明显冲突的卡写入 `output/role-overrides.json`。

- [ ] **Step 3: 应用并重生成数据**

Run: `python3 output/rebalance.py apply --roles output/roles-batch-a.json`
Run: `make data`

- [ ] **Step 4: 检查本批玩法**

Run: `python3 scripts/balance_report.py --check --playstyle mammal --playstyle neutral`
Expected: EXIT 0，两个玩法都在目标带 ±0.04 内，且哺乳纲平均攻血比 ≥0.38。

Run: `python3 - <<'PY'` 抽查 8 张卡（随机取哺乳纲 4 张、中立 4 张）：
```python
import json, random
d = json.load(open('server/data/v0.1/cards.json'))
cards = [c for c in d if c['playstyle'] in ('mammal','neutral') and c['card_type']=='minion' and not c['is_scholar']]
for c in random.Random(0).sample(cards, 8):
    print(c['id'], c['level'], c['attack'], c['health'], c['skills'])
PY
```
Expected: 每张都落在 `RANGE[level]` 内；技能数量符合星级上限。

- [ ] **Step 5: 重定金标准**

战斗金标准使用自造卡，只有共享池条目数变化时才改变。先运行：

Run: `cd server && cargo test -p rule-engine --test combat combat_state_hash_matches_golden_baseline`
Expected: 通过则不改；失败时按断言给出的新值更新 `combat.rs` 常量，并在上方注释追加「卡牌参数重排批次 A」。

模拟金标准一定会变：

Run: `cd server && cargo run -q -p sim -- batch --games 20 --seed 1 --out ../output/golden20-a.json`

在仓库根目录用以下脚本重写金标准（后续批次把 `golden20-a` 换成对应文件）：

```bash
python3 - <<'PY'
import json, pathlib
src = json.load(open('output/golden20-a.json'))
dst = {'seed': 1, 'games': 20, 'hashes': [g['final_hash'] for g in src['games']]}
path = pathlib.Path('server/crates/sim/tests/golden/seed-1-games-20.json')
path.write_text(json.dumps(dst, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
PY
```

在 `sim_determinism.rs` 的注释追加「卡牌参数重排批次 A」。

- [ ] **Step 6: 全量校验**

Run: `make data-check && make test`
Expected: EXIT 0。

- [ ] **Step 7: 提交**

```bash
git add docs/versions/v0.1/product/content/cards server/data/v0.1 client server/crates
git commit -m "feat(cards): 重排哺乳纲与中立随从身材"
```

---

### Task 4: 重排批次 B（鸟、爬行、鱼、昆虫、头足、水螅）

**Files:**
- Modify: 上述 6 个玩法的卡牌文档
- 复用: `output/rebalance.py`（Task 3 已创建，不提交）

**Interfaces:**
- Consumes: Task 3 的 `derive`、`apply_batch` 与命令格式。
- Produces: 6 个玩法的角色表 `output/roles-batch-b.json`。

- [ ] **Step 1: 生成角色表**

Run: `python3 output/rebalance.py plan --playstyles bird reptile fish insect cephalopod cnidarian --out output/roles-batch-b.json`

复核要点：鸟/昆虫以 output 为主；爬行/水螅以 tank 为主；鱼/头足 balanced 与 output 混合。

- [ ] **Step 2: 应用并重生成**

Run: `python3 output/rebalance.py apply --roles output/roles-batch-b.json && make data`

- [ ] **Step 3: 检查本批玩法**

Run: `python3 scripts/balance_report.py --check --playstyle bird --playstyle reptile --playstyle fish --playstyle insect --playstyle cephalopod --playstyle cnidarian`
Expected: EXIT 0；鸟与昆虫平均攻血比 ≥0.45，爬行与水螅 ≤0.36。

- [ ] **Step 4: 重定金标准**

同 Task 3 Step 5，注释写「卡牌参数重排批次 B」。

- [ ] **Step 5: 全量校验并提交**

Run: `make data-check && make test`
Expected: EXIT 0。

```bash
git add docs/versions/v0.1/product/content/cards server/data/v0.1 client server/crates
git commit -m "feat(cards): 重排鸟爬行鱼昆虫头足水螅身材"
```

---

### Task 5: 重排批次 C（植物、真菌、原生、细菌、病毒、机械）

**Files:**
- Modify: 上述 6 个玩法的卡牌文档
- 复用: `output/rebalance.py`

**Interfaces:**
- Consumes: Task 3 的 `derive`、`apply_batch`。
- Produces: `output/roles-batch-c.json`。

- [ ] **Step 1: 生成角色表**

Run: `python3 output/rebalance.py plan --playstyles plant fungus protozoa bacteria virus machine --out output/roles-batch-c.json`

复核要点：植物/细菌以 tank 为主；真菌/病毒偏 tank 与 balanced；原生/机械偏 balanced。

- [ ] **Step 2: 应用并重生成**

Run: `python3 output/rebalance.py apply --roles output/roles-batch-c.json && make data`

- [ ] **Step 3: 多技能密度补足（覆盖全部 13 个玩法）**

Run: `python3 scripts/balance_report.py --json > output/density-before-c.json`

这里检查的是全部 13 个玩法（批次 A/B/C 都可能欠密度），不限于本批。若 3–4 星带 2 技能的卡不足 50%，或 5–7 星带 2 个及以上技能的卡不足 80%，维护 `output/skill-additions.json`（`{"card_id": ["skill-id"]}`），按以下规则补技能后重跑 `apply`：

- 只从本玩法机制词汇中选（产品 §5.2），优先复用既有技能。
- 人工复核：第二个技能必须来自该卡玩法的词汇；表外技能只能作为 5–7 星的第三个「通用工具」技能，每卡至多 1 个；同一张卡不得出现两个触发时机相同、效果同类的技能。
- 不新增关键词；不得让任何关键词的最小星级前移。
- 5–7 星最多 3 个技能，其中最多 2 个登记关键词；第三个必须是不登记关键词的文本效果。
- 加技能后若 `attack + health + keyword_cost > CAP[level]`，脚本自动下调生命值直到合规。

- [ ] **Step 4: 检查本批玩法**

Run: `python3 scripts/balance_report.py --check --playstyle plant --playstyle fungus --playstyle protozoa --playstyle bacteria --playstyle virus --playstyle machine`
Expected: EXIT 0；植物 ≤0.35，细菌/病毒 ≤0.41，原生/机械 ≥0.40。

- [ ] **Step 5: 全量技能密度检查**

Run: `python3 scripts/balance_report.py --json | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['skill_density'])"`
Expected: `{"3-4": ≥0.50, "5-7": ≥0.80}`。

- [ ] **Step 6: 重定金标准**

同 Task 3 Step 5，注释写「卡牌参数重排批次 C」。

- [ ] **Step 7: 全量校验并提交**

Run: `make data-check && make test`
Expected: EXIT 0。

```bash
git add docs/versions/v0.1/product/content/cards server/data/v0.1 client server/crates
git commit -m "feat(cards): 重排植物真菌原生细菌病毒机械身材"
```

---

### Task 6: 63 张效果牌与配套技能

**Files:**
- Create: 63 个卡牌文档 `docs/versions/v0.1/product/content/cards/<id>.md`
- Create: 新技能文档 `docs/versions/v0.1/product/content/skills/<id>.md`
- Modify: `scripts/validate_docs.py`（效果牌 100、技能数、`EXPECT_ERA`）
- Modify: `tests/test_gen_data.py`（346→409、197→实际技能数）
- Modify: `server/crates/rule-engine/tests/catalog.rs`（346→409、197→实际技能数）
- Modify: `server/crates/rule-engine/tests/effects_exec.rs`（效果段总数与分类计数）
- Create（不提交）: `output/effect_cards.py`（从声明式表生成文档）

**Interfaces:**
- Consumes: 产品 §6.2 的 39 张定向牌与 §6.3 的 24 张通用牌。
- Produces: 100 张效果牌；每张 `skills` 非空；定向牌恰好 1 条 `play:`。

- [ ] **Step 1: 写一次性生成脚本**

`output/effect_cards.py` 维护 `TARGETED`（39 项）与 `GENERIC`（24 项）两张表，每项字段：`id`（拼音 slug）、`name`、`cost`、`level`、`era`、`play`（仅定向）、`skill`（复用 id 或新技能 DSL 描述）、`text`、`source`。

卡 id 固定供 Task 6 Step 9 的模拟器测试使用：洋流 = `yangliu`（鱼纲定向 1 费）。

脚本输出完整卡牌文档与缺失的技能文档：

- 卡牌 front matter：`id`、`name`、`card_type: effect`、`level`、`era`、`playstyle: neutral`、`taxon: phenomenon`、`cost`、`match_text: 同左`、`skills`，定向牌加 `specialty: [play:<play>]`。
- 「## 属性」表用 `make data-check` 的期望格式；定向牌写「关联分类 | 关联：<玩法中文名>」。
- 「## 效果」等于技能 `text`。
- 新技能：`OnPlayEffect|<Op>|<selector>|<params>`，`match_text: 同左`；仅当既有技能的效果、目标与持续时间完全一致时复用。

- [ ] **Step 2: 按数量配额分配星级与费用**

新增牌的星级数量固定为：

| 星级 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 新增 | 6 | 11 | 11 | 12 | 10 | 8 | 5 |

分配规则：定向牌前期 13 张 → 1–2 星（6/7），中期 13 张 → 3–4 星（8/5），后期 13 张 → 5–7 星（7/4/2）；通用牌补齐剩余名额（0/4/3/7/3/4/3）。

费用固定：定向牌前期 1 费、中期 2 费、后期 3 费；其中 3 张后期牌为 4 费，必须是 6–7 星（`深海回响`、`繁茂`、`珊瑚礁`）。通用牌按产品 §6.3 的费用列；其中 2 张 4 费牌（`终局`、`文明飞跃`）也必须分配在 6–7 星。

- [ ] **Step 3: 生成并重出数据**

先补 2 个衍生 Token（写入 `docs/versions/v0.1/product/content/tokens/README.md` 注册表）：

| id | 名称 | 攻击力 | 生命值 | 玩法 | 科学分类 | 生成来源 |
| --- | --- | --- | --- | --- | --- | --- |
| `neutral-token-2-2` | 基础细胞 | 2 | 2 | neutral | phenomenon | 通用效果牌「集结」「召唤」 |
| `amoeba-larva-3-3-token` | 变形虫幼体 | 3 | 3 | protozoa | amoebidae | 原生定向牌「细胞分裂」 |

Run: `python3 output/effect_cards.py && make data`
Expected: `server/data/v0.1/cards.json` 出现 100 张效果牌；`tokens.json` 出现 23 个 Token。

- [ ] **Step 4: 更新数量期望**

- `scripts/validate_docs.py`：`len(skills) != 197` 改为实际技能数并同步提示；`counts != {"minion": 309, "effect": 37}` 改为 `{"minion": 309, "effect": 100}`；`EXPECT_ERA` 用 `python3 -c` 统计实际时代分布后写回。
- `tests/test_gen_data.py`：346→409；197→实际技能数。
- `server/crates/rule-engine/tests/catalog.rs`：346→409；197→实际技能数；`tokens.len()` 21→23。
- `server/crates/rule-engine/tests/effects_exec.rs`：运行 `cargo test -p rule-engine --test effects_exec coverage_counts_all_effect_segments`，按失败信息把 184 与 3 个分类计数改为实际值。

Run: `make data-check`
Expected: EXIT 0。

- [ ] **Step 5: 检查效果牌分布**

Run: `make balance-check`
Expected: EXIT 0。效果牌总数 100、星级 10/18/22/20/16/9/5、费用 8/33/34/19/6、每级占比 ≤30%、定向 39 / 通用 61；攻血比与技能密度应已由 Task 3–5 满足，若有失败项回到对应批次修正。

Run: `python3 scripts/balance_report.py --json | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['effects'])"`
Expected: 同上分布。

- [ ] **Step 6: 重定模拟金标准**

新增 63 张效果牌与 2 个 Token 会改变卡池与对局。先检查战斗哈希：

Run: `cd server && cargo test -p rule-engine --test combat combat_state_hash_matches_golden_baseline`
通过则不改；失败时按断言给出的新值更新 `combat.rs` 常量，注释追加「效果牌扩充」。

模拟金标准：

Run: `cd server && cargo run -q -p sim -- batch --games 20 --seed 1 --out ../output/golden20-effects.json`

```bash
python3 - <<'PY'
import json, pathlib
src = json.load(open('output/golden20-effects.json'))
dst = {'seed': 1, 'games': 20, 'hashes': [g['final_hash'] for g in src['games']]}
path = pathlib.Path('server/crates/sim/tests/golden/seed-1-games-20.json')
path.write_text(json.dumps(dst, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
PY
```

在 `sim_determinism.rs` 注释追加「效果牌扩充与定向关联」。

- [ ] **Step 7: 抽查定向牌过滤**

Run: `cd server && cargo test -p rule-engine --test pool_filter -v`
Expected: PASS。

Run: `python3 - <<'PY'`
```python
import json
d = json.load(open('server/data/v0.1/cards.json'))
eff = [c for c in d if c['card_type'] == 'effect']
targeted = [c for c in eff if any(s['kind'] == 'play' for s in c.get('specialty') or [])]
generic = [c for c in eff if not c.get('specialty')]
print(len(eff), len(targeted), len(generic))
from collections import Counter
print(Counter(s['value'] for c in targeted for s in c['specialty']))
PY
```
Expected: `100 39 61`；13 个玩法 id 各 3 张。

- [ ] **Step 8: 提交**

```bash
make data-check
git add docs/versions/v0.1/product/content/cards docs/versions/v0.1/product/content/skills \
  docs/versions/v0.1/product/content/tokens/README.md \
  scripts/validate_docs.py tests/test_gen_data.py server/data/v0.1 client \
  server/crates/rule-engine/tests/{catalog.rs,effects_exec.rs,combat.rs} server/crates/sim/tests
git commit -m "feat(cards): 扩增 63 张效果牌与定向玩法关联"
```

---

- [ ] **Step 9: 定向效果牌购买过滤**

**Files:**
- Modify: `server/crates/sim/src/ai.rs`

**Interfaces:**
- Produces: `effect_matches_playstyle(card: &Card, dominant: Playstyle) -> bool`。

先写失败测试，追加到 `ai.rs` 测试模块：

```rust
#[test]
fn buy_skips_targeted_effect_of_other_playstyle() {
    let catalog = embedded_catalog().expect("内置卡表应能加载");
    let (_, mut state) = Engine::new(catalog.clone(), [7; 32], rule_engine::playstyles::draw(7));
    state.phase = Phase::Recruit;
    state.turn = 1;
    state.players[0].board = vec![MinionInstance::new(&catalog, 1, "saman", false)];
    state.players[0].gold = 10;
    state.players[0].shop = vec![Some(rule_engine::state::ShopSlot {
        instance_id: 10,
        card_id: "yangliu".into(),
        frozen: false,
    })];
    assert_eq!(
        choose_buy(&state, &state.players[0], &catalog),
        None,
        "哺乳玩家不买鱼的定向牌"
    );

    state.players[0].board = vec![MinionInstance::new(&catalog, 2, "clownfish", false)];
    assert_eq!(
        choose_buy(&state, &state.players[0], &catalog),
        Some(0),
        "鱼纲玩家可以买鱼的定向牌"
    );
}
```

Run: `cd server && cargo test -p sim buy_skips_targeted_effect_of_other_playstyle -v`
Expected: FAIL，哺乳玩家仍会购买 `yangliu`。

实现：

```rust
fn effect_matches_playstyle(card: &Card, dominant: Playstyle) -> bool {
    if card.card_type != CardType::Effect || dominant == Playstyle::Neutral {
        return true;
    }
    let mut any = false;
    for specialty in &card.specialty {
        if specialty.kind != rule_engine::data::SpecialtyKind::Play {
            continue;
        }
        any = true;
        if Playstyle::from_id(&specialty.value) == Some(dominant) {
            return true;
        }
    }
    !any
}
```

在 `best_affordable` 开头计算一次 `dominant = dominant_playstyle(player.board.iter().map(|m| m.card_id.as_str()), catalog)`；槽位循环里取到 `card` 后，若 `!effect_matches_playstyle(card, dominant)` 则 `continue`。

Run: `cd server && cargo test -p sim buy_skips_targeted_effect_of_other_playstyle -v`
Expected: PASS。

购买过滤会改变模拟对局，重定模拟金标准：

Run: `cd server && cargo run -q -p sim -- batch --games 20 --seed 1 --out ../output/golden20-filter.json`

```bash
python3 - <<'PY'
import json, pathlib
src = json.load(open('output/golden20-filter.json'))
dst = {'seed': 1, 'games': 20, 'hashes': [g['final_hash'] for g in src['games']]}
path = pathlib.Path('server/crates/sim/tests/golden/seed-1-games-20.json')
path.write_text(json.dumps(dst, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
PY
```

在 `sim_determinism.rs` 注释追加「AI 定向效果牌购买过滤」。

- [ ] **Step 10: 全量校验并提交 AI 过滤**

Run: `make data-check && make test && make lint`
Expected: EXIT 0。

```bash
git add server/crates/sim/src/ai.rs server/crates/sim/tests
git commit -m "feat(sim): AI 跳过不匹配的定向效果牌"
```

---

### Task 7: 模拟调参与最终金标准

**Files:**
- Modify: 卡牌文档与技能文档（按调参结论）
- Modify: `server/crates/rule-engine/tests/combat.rs`（`state_hash` 常量）
- Modify: `server/crates/sim/tests/golden/seed-1-games-20.json`
- Modify: `server/crates/sim/tests/sim_determinism.rs`（重定注释）

**Interfaces:**
- Consumes: `make m0-batch` 输出的 `output/m0-batch.json` 与 `make m0-report` 的胜率表。

- [ ] **Step 1: 跑 1000 局基线**

Run: `make m0-batch && make m0-report`
Record: 每个玩法 `win_rate_permille_by_playstyle`、`avg_turns`、`survivors_at_turn_20`。

- [ ] **Step 2: 按顺序调参**

只改低于 4% 或高于 12% 的玩法，每轮只改一个维度：

1. 夺冠率 <4%：先给该玩法 4–6 星在强势期内的卡总身材 +1（不超过 `CAP`），再检查 5–7 星是否都有 2 个技能，最后才提高签名参数 N（新建技能变体，如 `devour-5`），每次只动一项。
2. 夺冠率 >12%：反向操作，先降总身材，再降技能数量，最后降参数。
3. 不修改 `TARGETS`、不修改验收区间。
4. 若观察到 AI 在 1–2 星大量购买定向牌、拖慢经济：把前期定向牌从 1 星集中到 2 星，空出的 1 星名额用通用牌补足；每级效果牌总数不变，只改定向/通用配比。

每轮后重跑 `make m0-batch && make m0-report`，记录到 `output/tuning-log.md`（不提交）。最多 3 轮；3 轮后仍有玩法越界时停下来报告用户，不擅自放宽验收。

- [ ] **Step 3: 冻结数值并重定金标准**

Run: `cd server && cargo test -p rule-engine --test combat combat_state_hash_matches_golden_baseline`
通过则不改；失败时把断言给出的新值写入 `combat.rs`，注释追加「卡牌参数重排最终冻结（2026-11-18）」。

Run: `cd server && cargo run -q -p sim -- batch --games 20 --seed 1 --out ../output/golden20-final.json`

```bash
python3 - <<'PY'
import json, pathlib
src = json.load(open('output/golden20-final.json'))
dst = {'seed': 1, 'games': 20, 'hashes': [g['final_hash'] for g in src['games']]}
path = pathlib.Path('server/crates/sim/tests/golden/seed-1-games-20.json')
path.write_text(json.dumps(dst, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
PY
```

在 `sim_determinism.rs` 注释追加同一原因。

- [ ] **Step 4: 最终校验**

Run: `make balance-check`
Expected: EXIT 0。

Run: `make m0-determinism && make m0-batch && make m0-report`
Expected: 每个玩法 40–120‰；`avg_turns` 16–19；`survivors_at_turn_20` 2–4。

- [ ] **Step 5: 提交**

```bash
git add docs/versions/v0.1/product/content/cards docs/versions/v0.1/product/content/skills \
  server/data/v0.1 client server/crates
git commit -m "feat(balance): 收敛玩法胜率并重定模拟金标准"
```

---

### Task 8: 文档回写与全量验证

**Files:**
- Modify: `docs/versions/v0.1/product/content/card-system.md`
- Modify: `docs/versions/v0.1/product/systems/economy-and-numbers.md`
- Modify: `docs/versions/v0.1/product/content/balance-analysis.md`
- Modify: `docs/versions/v0.1/product/content/cards/README.md`
- Modify: `docs/versions/v0.1/product/content/skills/README.md`
- Modify: `docs/versions/v0.1/product/content/taxonomy.md`
- Modify: `docs/current/technical/rule-engine-and-simulator.md`

**Interfaces:**
- Consumes: Task 6 的实际数量与 Task 7 的最终胜率表。

- [ ] **Step 1: 更新卡牌规范**

`card-system.md`：效果牌允许 0 或 1 条 `play:`；效果牌 37→100；卡牌总数 346→409；技能数量上限 1–2 星 1 个、3–4 星 2 个、5–7 星 3 个且最多 2 个登记关键词；无技能随从只允许 1 张中立工具卡。

- [ ] **Step 2: 更新数值与经济**

`economy-and-numbers.md`：关键词折算表与本次参数带对齐；4 费效果牌允许 6–7 星；补充「效果牌按星级占比 ≤30%」的校验口径。

- [ ] **Step 3: 重写平衡测算**

`balance-analysis.md`：按最终数据重算每个玩法的平均攻血比、目标带、偏差、3–4 星与 5–7 星技能密度、1000 局胜率与回合数；结论一节写明「攻血比差 ≥0.20、每个玩法 4%–12%」是否达成。

- [ ] **Step 4: 更新索引与统计**

- `cards/README.md`：随从 309、效果牌 100、总数 409；新增效果牌条目。
- `skills/README.md`：技能总数与新增技能条目。
- `taxonomy.md`：分类节点统计（数量变化时）。
- `docs/current/technical/rule-engine-and-simulator.md`：补充效果牌按 `play:` 过滤的既有行为与模拟器 AI 的打出/过滤规则。

- [ ] **Step 5: 全量验证矩阵**

依次运行并记录结果：

```bash
make data-check
make test
make lint
make m0-determinism
make m0-batch
make m0-report
make protocol-check
make protocol-rs-check
make client-check
make client-smoke
make m1b-e2e
make balance-check
```

Expected: 全部 EXIT 0；`make m0-report` 的胜率与 Task 7 记录一致。

- [ ] **Step 6: 提交**

```bash
git add docs
git commit -m "docs: 回写卡牌重排与效果牌扩充结果"
```

---

## 验收对照

| 产品验收项 | 对应任务 | 证据 |
| --- | --- | --- |
| 随从 309 / 效果牌 100 / 学者 49 / 英雄 32 | Task 1、6、8 | `make data-check`、`catalog.rs` |
| 攻血比差 ≥0.20，每个玩法在目标带 ±0.04 | Task 3–5、7 | `make balance-check` |
| 单卡 `attack + health + keyword_cost ≤ CAP[level]` | Task 3–6 | `make data-check` |
| 3–4 星 ≥50% 双技能，5–7 星 ≥80% 双技能+，1–2 星 ≤1 技能 | Task 1、5 | `balance_report.py --json` |
| 效果牌 100、每级占比 ≤30%、定向 39 + 通用 24 | Task 6 | `make balance-check` |
| 1000 局每个玩法 4%–12%，平均回合 16–19，第 20 回合存活 2–4 | Task 7 | `make m0-report` |
| 引擎、协议、客户端回归 | Task 8 | 验证矩阵 |
| 文档回写 | Task 8 | 文档 diff |
