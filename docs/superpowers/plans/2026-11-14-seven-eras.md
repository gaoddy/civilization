# 七时代基线重做 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 v0.1 产品基线从 5 个时代 / 星级 1–6 重做为 7 个时代 / 星级 1–7，并补齐 40 张随从与 4 张效果牌。

**Architecture:** 先改规则与数值口径，再扩展校验脚本，最后分批加入内容并用 `validate_docs.py` 作为每批的验收闸门。关键词解锁由卡池最小星级反推，不改存量 136 张卡。

**Tech Stack:** Markdown 文档基线 + 标准库 Python 校验脚本（`python3 scripts/validate_docs.py`）。

**Spec:** `docs/superpowers/specs/2026-11-14-seven-eras-design.md`

## Global Constraints

- 时代顺序固定：史前(1) → 原始(2) → 古典(3) → 封建(4) → 工业(5) → 现代(6) → 未来(7)。
- 星级固定 1–7；7 星数值区间 10–15 攻 / 14–20 血，白板 12/17，超模上限 33，牌池每张 5 份。
- 升级费用固定 5 / 7 / 9 / 10 / 11 / 12；每次只能升一级；降价每回合 −1、最低 2。
- 单局 20 回合、40 点生命；伤害上限 15、下限 1。
- 不新增关键词；现有 13 个关键词（通用 10 + 专属 3）词表不变。
- 存量 136 张随从的星级、关键词、数值、`era` 全部不改。
- 每批内容落地后必须 `python3 scripts/validate_docs.py` 退出码 0。
- 新卡 `id` 必须等于文件名，匹配 `^[a-z][a-z0-9-]*$`；中文名用拼音，外国人名用英文。
- 新卡 `source` 只能是 `素材` 或 `新增`。
- 学者 `tribe: 学者`、`is_scholar: true`、`attack: 0`；生命值取同星级上限 ×1.3 向下取整。
- 效果牌 `tribe` 用 `通用` 或已启用流派；`cost` 0–4；被引用的技能必须有 `match_text`。
- 不改扩展层（场景、陨落、坐镇、大决战）的实施状态，只更新「每时代一位坐镇英雄」的数量口径。

## Review Focus

- **7 星池耗尽**：未来时代 7 星每张只有 5 份，8 人局抽空后商店必须按现有重抽规则回退，而不是报错或留死槽；Task 3 的概率表与 Task 5 的校验要保证 7 星池非空。
- **关键词提前出现**：新卡若把某关键词的最小星级拉低，会破坏"史前不会出现嘲讽"的节奏；Task 5 的最小星级校验必须在每批内容后拦截。
- **学者数值**：7 星学者生命值没有区间校验，容易被写成超模；Task 8 要求逐张按 `上限 ×1.3` 核算。
- **伤害与时长**：星级和上升会抬高伤害，上限 15 必须保留；Task 3 的健康指标和 Task 11 的时长口径要一致。
- **旧口径残留**：规则改了但 README、路线图、总纲仍写 5 时代/1–6 星，会让实现按旧表编码；Task 11 的全局检索是硬闸门。

---

### Task 0: 提交设计文档与计划

**Files:**
- Create: `docs/superpowers/specs/2026-11-14-seven-eras-design.md`（已存在）
- Create: `docs/superpowers/plans/2026-11-14-seven-eras.md`（本文件）

- [ ] **Step 1: 提交**

```bash
git add docs/superpowers
git commit -m "docs: 七时代扩展设计与实施计划"
```

---

### Task 1: 七时代总表与推进规则

**Files:**
- Modify: `docs/current/product/rules/eras-and-scenarios.md`

**Interfaces:**
- Produces: 七时代总表（时代、商店等级、开放星级、升级费用、关键词首次可用、设计意图），供 Task 2/3/4 引用。

- [ ] **Step 1: 替换 §1 总表为七行**

| 时代 | 商店等级 | 开放星级 | 升级费用 | 关键词首次可用 | 素材来源 |
| --- | --- | --- | --- | --- | --- |
| 史前文明 | 1 | 1 | 5 | 繁衍 | 时代机制 |
| 原始社会 | 2 | 1–2 | 7 | 成长、嘲讽 | 时代机制 |
| 古典文明 | 3 | 1–3 | 9 | 连击、最优攻击、传染 | 新增（补奴隶社会/城邦阶段） |
| 封建社会 | 4 | 1–4 | 10 | 复生 | 时代机制 |
| 工业社会 | 5 | 1–5 | 11 | 护盾 | 新增（补工业革命阶段） |
| 现代社会 | 6 | 1–6 | 12 | — | 时代机制 |
| 未来社会 | 7 | 1–7 | — | — | 时代机制 |

同段保留并改写两套等级的说明：商店等级 = 玩家时代（1–7）；开放星级 = 可抽随从星级（1–7）。

- [ ] **Step 2: 替换 §2 推进表**

升级费用行改为 `5 / 7 / 9 / 10 / 11 / 12`；最早可升级回合改为 `第 3 / 5 / 7 / 8 / 9 / 11 回合`；其余规则（降价、免费重抽、不下降、同回合顺序）保持不变。

- [ ] **Step 3: 重排 §3 各时代设计意图为七节**

顺序改为 3.1 史前、3.2 原始、3.3 古典文明、3.4 封建社会、3.5 工业社会、3.6 现代社会、3.7 未来社会。新增两节写：

- 古典文明：城邦、帝国与方阵；首次出现「嘲讽」，站位开始有意义；出场流派以人类、学者、机械为主。
- 工业社会：蒸汽、工厂、铁路、电力；首次出现「护盾」；机械绕开嘲讽；出场流派以机械、人类、病毒为主。

- [ ] **Step 4: 更新 §6 素材映射说明**

把「素材的时代机制只写了五个时代名」改为七段口径，并注明：素材只给了五个时代名，古典文明与工业社会为本作新增，用于补齐演进跨度；两段的机制从既有关键词中分配，不新增词。

- [ ] **Step 5: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0（文档改动不影响校验）。

Run: `grep -n "五时代\|五个时代\|1–5" docs/current/product/rules/eras-and-scenarios.md`
Expected: 无输出。

- [ ] **Step 6: 提交**

```bash
git add docs/current/product/rules/eras-and-scenarios.md
git commit -m "docs: 时代规则扩展为七段演进"
```

---

### Task 2: 关键词解锁表

**Files:**
- Modify: `docs/current/product/rules/keywords-and-mechanics.md`（§7）

**Interfaces:**
- Consumes: Task 1 的七时代顺序。
- Produces: 七行解锁表与"以星级为准"的口径，供 Task 5 的 `KW_UNLOCK` 固定值使用。

- [ ] **Step 1: 替换 §7 解锁表**

| 时代 | 首次可用关键词 |
| --- | --- |
| 史前 | 繁衍 |
| 原始 | 成长、嘲讽 |
| 古典 | 连击、最优攻击、传染 |
| 封建 | 复生 |
| 工业 | 护盾 |
| 现代 | — |
| 未来 | — |

- [ ] **Step 2: 在表后补两段口径**

1. 抽取只看星级；「首次可用时代」等于该关键词在卡池中出现的最小星级，由卡池反推，不反向约束卡牌。
2. 现代与未来不引入新关键词，终局身份来自 6/7 星池与关键词组合。战吼、亡语从史前起可用；装配、播种、进化仍随流派出现、不参与时代解锁。

- [ ] **Step 3: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0。

Run: `grep -n "史前 | 繁衍" docs/current/product/rules/keywords-and-mechanics.md`
Expected: 命中新的 7 行表头附近。

- [ ] **Step 4: 提交**

```bash
git add docs/current/product/rules/keywords-and-mechanics.md
git commit -m "docs: 关键词解锁表按七时代重排"
```

---

### Task 3: 数值与经济

**Files:**
- Modify: `docs/current/product/systems/economy-and-numbers.md`

**Interfaces:**
- Consumes: Task 1 的费用与等级。
- Produces: 7 行概率矩阵、7 行星级区间、7 行份数，Task 5/6/7/8 的数值校验依据。

- [ ] **Step 1: 替换 §2 升时代费用表**

| 升级 | 基础费用 | 平均可负担回合 |
| --- | --- | --- |
| 史前 → 原始 | 5 | 第 3 回合 |
| 原始 → 古典 | 7 | 第 5 回合 |
| 古典 → 封建 | 9 | 第 7 回合 |
| 封建 → 工业 | 10 | 第 8 回合 |
| 工业 → 现代 | 11 | 第 9 回合 |
| 现代 → 未来 | 12 | 第 11 回合 |

保留"每回合 −1、最低 2、一次只能升一级"的说明；把示例改为第 3 回合升原始。

- [ ] **Step 2: 替换 §3 概率矩阵为七行**

| 商店等级 | 1 星 | 2 星 | 3 星 | 4 星 | 5 星 | 6 星 | 7 星 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 史前 | 100 | — | — | — | — | — | — |
| 2 原始 | 70 | 30 | — | — | — | — | — |
| 3 古典 | 50 | 35 | 15 | — | — | — | — |
| 4 封建 | 30 | 35 | 25 | 10 | — | — | — |
| 5 工业 | 20 | 25 | 25 | 20 | 10 | — | — |
| 6 现代 | 10 | 15 | 25 | 25 | 15 | 10 | — |
| 7 未来 | 5 | 10 | 15 | 20 | 20 | 18 | 12 |

- [ ] **Step 3: 替换 §4 牌池份数表为七行**

`1 星 16 / 2 星 15 / 3 星 13 / 4 星 11 / 5 星 9 / 6 星 7 / 7 星 5`，说明 7 星是终局抢卡。

- [ ] **Step 4: 替换 §5 星级数值区间为七行**

| 星级 | 攻击力区间 | 生命值区间 | 参考白板 | 超模上限 |
| --- | --- | --- | --- | --- |
| 1 星 | 1–2 | 2–4 | 1/3 | 8 |
| 2 星 | 2–3 | 3–5 | 2/4 | 10 |
| 3 星 | 3–5 | 4–7 | 4/5 | 13 |
| 4 星 | 4–7 | 6–9 | 6/7 | 17 |
| 5 星 | 6–9 | 8–12 | 8/10 | 22 |
| 6 星 | 8–12 | 10–16 | 10/12 | 26 |
| 7 星 | 10–15 | 14–20 | 12/17 | 33 |

学者规则补 7 星示例：`0/26`。关键词折算表不变。

- [ ] **Step 5: 更新 §7 健康指标中写死"5 时代/6 星"的行**

把「平均升到 5 时代的回合」改为「平均升到 7 时代的回合」，目标区间 `12–14`；把与星级上限相关的指标改为 1–7。

- [ ] **Step 6: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0。

Run: `grep -n "1–5\|1–6\|5 时代" docs/current/product/systems/economy-and-numbers.md`
Expected: 无输出。

- [ ] **Step 7: 提交**

```bash
git add docs/current/product/systems/economy-and-numbers.md
git commit -m "docs: 经济与星级数值扩展到七档"
```

---

### Task 4: 总纲、核心规则与概念口径

**Files:**
- Modify: `docs/current/product/overview/design-overview.md`
- Modify: `docs/current/product/rules/core-rules.md`
- Modify: `docs/current/product/overview/concept-and-positioning.md`
- Modify: `docs/versions/v0.1/README.md`

**Interfaces:**
- Consumes: Task 1–3 的表。
- Produces: 全仓统一口径，供 Task 10 的版本文档引用。

- [ ] **Step 1: design-overview.md 逐处改口径**

- §0/§1 的「五时代」改为「七时代」，时代序列改为 `史前 → 原始 → 古典 → 封建 → 工业 → 现代 → 未来`。
- §6.2 升时代费用改为 `5 / 7 / 9 / 10 / 11 / 12`，降价与回合示例同步。
- §6.3 商店等级改为 1–7，星级改为 1–7；删除"时代 5 同时开放 5/6 星"，改为"时代 7 开放 1–7 星"。
- §6.7 的映射改为 `史前(1)/原始(2)/古典(3)/封建(4)/工业(5)/现代(6)/未来(7)`。
- §6.9 伤害公式的时代上限改为 1–7，上限仍 15。
- §8 的数量表：时代 7、卡牌 176 随从 + 36 效果（本任务只改口径，最终数字以 Task 9 为准）；关键词仍 13。

- [ ] **Step 2: core-rules.md 改规则常量**

- `UpgradeEra` 的前置条件 `时代 < 5` 改为 `时代 < 7`。
- 伤害公式 `败者当前已升到的时代等级（1–5）` 改为 `（1–7）`。
- 文中所有"5 个时代/升到未来即满级"改为七段口径。

- [ ] **Step 3: concept-and-positioning.md 改对比表**

把「五时代演进」改为「七时代演进」；与文明 6 的对比句同步为七段。

- [ ] **Step 4: v0.1/README.md 改能力清单**

| 项 | 新值 |
| --- | --- |
| 时代 | 7 个：史前、原始、古典、封建、工业、现代、未来 |
| 卡牌 | 176 张随从 + 36 张效果牌 |
| 星级 | 1–7 |

关键词 13、英雄 13、成就 6、流派 6 不变。

- [ ] **Step 5: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0。

Run: `grep -rn "五时代\|五个时代\|1–5\|1–6" docs/current/product/overview docs/current/product/rules docs/versions/v0.1/README.md`
Expected: 无输出（历史规划文档不在本命令范围）。

- [ ] **Step 6: 提交**

```bash
git add docs/current/product/overview docs/current/product/rules/core-rules.md docs/versions/v0.1/README.md
git commit -m "docs: 总纲与核心规则同步七时代口径"
```

---

### Task 5: 校验脚本结构与关键词校验

**Files:**
- Modify: `scripts/validate_docs.py`

**Interfaces:**
- Consumes: Task 1–2 的表。
- Produces: `ERAS`、`RANGE[7]`、`CAP[7]`、`KW_UNLOCK`、`check_keyword_unlock()`，供 Task 6–8 的每批内容验收。

- [ ] **Step 1: 扩展常量**

```python
ERAS = ["史前", "原始", "古典", "封建", "工业", "现代", "未来"]
# 在现有 RANGE 字典中加入：
#     7: (10, 15, 14, 20)
# 在现有 CAP 字典中加入：
#     7: 33
KW_UNLOCK = {"繁衍": 1, "嘲讽": 2, "连击": 3, "最优攻击": 3, "传染": 3, "复生": 4, "护盾": 5}
```

`EXPECT_ERA`、`EXPECT_TRIBE`、`EXPECT_LEVEL` 与卡牌类型计数本任务不动，由内容任务逐批更新。

- [ ] **Step 2: 新增关键词最小星级校验**

```python
def check_keyword_unlock(minions, skill_map):
    actual = {}
    for c in minions:
        for k in derived_keywords(c, skill_map):
            base = k.split()[0]
            if base in KW_UNLOCK:
                actual[base] = min(actual.get(base, 99), c["level"])
    for keyword, era_level in KW_UNLOCK.items():
        if actual.get(keyword) != era_level:
            fail(f"关键词 {keyword} 的最小星级为 {actual.get(keyword)}，期望 {era_level}")
```

在 `main()` 的 `check_cards` 之后调用。

- [ ] **Step 3: 增加诊断输出（不参与判定）**

在成功分支追加两行打印：按关键词统计卡牌张数、按技能统计引用数。用于 Task 9 回写 `card-system.md` 与 `skills/README.md`。

```python
print("关键词", dict(sorted(collections.Counter(
    k.split()[0] for c in minions for k in derived_keywords(c, skill_map)).items())))
print("技能引用", dict(sorted(collections.Counter(
    sid for c in raw for sid in card_skills(c)).items())))
```

- [ ] **Step 4: 验证正向**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0；输出包含 `关键词` 与 `技能引用` 两行。

- [ ] **Step 5: 验证反向（证明校验会拦截）**

临时把 `KW_UNLOCK["嘲讽"]` 改为 `3`，运行脚本，Expected: 退出码 1，且输出包含 `关键词 嘲讽 的最小星级为 2，期望 3`。随后改回 `2`。

- [ ] **Step 6: 提交**

```bash
git add scripts/validate_docs.py
git commit -m "test: 校验脚本支持七时代与关键词最小星级"
```

---

### Task 6: 古典文明内容（14 随从 + 2 效果牌）

**Files:**
- Create: `docs/current/product/content/cards/<id>.md`（16 份）
- Modify: `scripts/validate_docs.py`（EXPECT_* 与类型计数）

**Interfaces:**
- Consumes: Task 5 的脚本。
- Produces: 14 个 `era: 古典` 随从、2 张效果牌；中间期望值 150 随从 / 34 效果。

- [ ] **Step 1: 按清单创建随从文档**

| id | 名称 | 流派 | 子类 | 星级 | 攻/血 | 技能 | 来源 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| chengbang-gongmin | 城邦公民 | 人类 | 城邦 | 1 | 1/3 | battlecry-friendly-plus-0-plus-1 | 新增 |
| shuizhong | 水钟 | 机械 | 计时 | 1 | 1/3 | battlecry-friendly-plus-0-plus-1 | 新增 |
| xinge | 信鸽 | 生物 | 通信 | 1 | 1/2 | battlecry-friendly-growth-once | 新增 |
| majidun-changqiangbing | 马其顿长枪兵 | 人类 | 兵种 | 2 | 3/5 | keyword-taunt | 新增 |
| sugeladi | 苏格拉底 | 学者 | 哲学 | 2 | 0/6 | battlecry-friendly-growth-once | 新增 |
| xiluoduode | 希罗多德 | 学者 | 史学 | 2 | 0/6 | battlecry-friendly-plus-0-plus-2 | 新增 |
| gongchengchui | 攻城槌 | 机械 | 攻城 | 2 | 3/4 | battlecry-random-enemy-damage-1 | 新增 |
| ganlanshu | 橄榄树 | 植物 | 作物 | 2 | 2/4 | keyword-sow | 新增 |
| doushouchang-shizi | 斗兽场狮子 | 生物 | 猛兽 | 2 | 3/4 | keyword-taunt | 新增 |
| yuanlaoyuan-yiyuan | 元老院议员 | 人类 | 政治 | 3 | 4/6 | battlecry-other-friendly-humans-plus-1-attack | 新增 |
| homer | 荷马 | 学者 | 文学 | 3 | 0/9 | battlecry-all-friendly-growth-once | 素材 |
| nupao | 弩炮 | 机械 | 攻城 | 3 | 4/5 | combatstart-leftmost-damage-3 | 新增 |
| zhanxiang | 战象 | 生物 | 猛兽 | 3 | 5/7 | keyword-taunt | 新增 |
| yadian-wenyi | 雅典瘟疫 | 病毒 | 瘟疫 | 3 | 3/5 | combatstart-random-enemy-contagion-1 | 新增 |

所有随从 `era: 古典`；学者行 `is_scholar: true` 且 `attack: 0`。

- [ ] **Step 2: 按清单创建效果牌**

| id | 名称 | 星级 | 流派 | 费用 | 技能 |
| --- | --- | --- | --- | --- | --- |
| chengbang-faling | 城邦法令 | 3 | 通用 | 1 | trigger-friendly-plus-1-plus-1 |
| aolinpike-jidian | 奥林匹克祭典 | 3 | 通用 | 2 | trigger-all-friendly-growth-once |

效果牌 `match_text: 同左`。

- [ ] **Step 3: 更新脚本中间期望值**

```python
EXPECT_ERA = {"史前": 27, "原始": 26, "古典": 14, "封建": 27, "现代": 36, "未来": 20}
EXPECT_TRIBE = {"学者": 31, "生物": 27, "人类": 31, "机械": 25, "植物": 19, "病毒": 17}
EXPECT_LEVEL = {1: 22, 2: 34, 3: 33, 4: 24, 5: 22, 6: 15}
# 类型计数 {"随从": 150, "效果": 34}
```

- [ ] **Step 4: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0；末行 `随从 150 张、效果牌 34 张、技能 118 个、英雄 13 位、成就 6 个，校验通过`。

- [ ] **Step 5: 提交**

```bash
git add docs/current/product/content/cards scripts/validate_docs.py
git commit -m "feat: 补齐古典文明卡牌"
```

---

### Task 7: 工业社会内容（14 随从 + 2 效果牌）

**Files:**
- Create: `docs/current/product/content/cards/<id>.md`（16 份）
- Modify: `scripts/validate_docs.py`

**Interfaces:**
- Consumes: Task 6 的 150/34 基线。
- Produces: 中间期望值 164 随从 / 36 效果。

- [ ] **Step 1: 按清单创建随从文档**

| id | 名称 | 流派 | 子类 | 星级 | 攻/血 | 技能 | 来源 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| fangzhiji | 纺织机 | 机械 | 工厂 | 2 | 2/4 | battlecry-friendly-growth-once | 新增 |
| mianhua | 棉花 | 植物 | 作物 | 2 | 2/4 | keyword-sow | 新增 |
| meikuang-jinsique | 煤矿金丝雀 | 生物 | 动物 | 2 | 2/3 | deathrattle-all-enemies-minus-1-health | 新增 |
| zhengqi-jiche | 蒸汽机车 | 机械 | 交通 | 3 | 4/6 | keyword-taunt | 新增 |
| tielu-gongren | 铁路工人 | 人类 | 工人 | 3 | 3/5 | battlecry-friendly-plus-0-plus-2 | 新增 |
| feijiehe | 肺结核 | 病毒 | 疾病 | 3 | 3/5 | keyword-contagion | 新增 |
| dianbaoji | 电报机 | 机械 | 通信 | 3 | 4/5 | combatstart-leftmost-damage-3 | 新增 |
| neiranji | 内燃机 | 机械 | 动力 | 4 | 6/8 | keyword-optimal-attack | 新增 |
| famingjia | 发明家 | 人类 | 发明 | 4 | 5/8 | battlecry-all-mech-growth-once | 新增 |
| watt | 瓦特 | 学者 | 工程 | 4 | 0/12 | aura-mech-plus-2-plus-2 | 新增 |
| huangrebing | 黄热病 | 病毒 | 疾病 | 4 | 5/8 | combatstart-random-enemy-contagion-2 | 新增 |
| dianhua-jiaohuanji | 电话交换机 | 机械 | 通信 | 5 | 7/10 | battlecry-all-friendly-growth-once | 新增 |
| faraday | 法拉第 | 学者 | 物理 | 5 | 0/16 | keyword-divine-shield | 新增 |
| gongchangzhu | 工厂主 | 人类 | 资本 | 5 | 7/11 | battlecry-other-friendly-humans-plus-1-attack | 新增 |

所有随从 `era: 工业`；学者行 `is_scholar: true` 且 `attack: 0`。

- [ ] **Step 2: 按清单创建效果牌**

| id | 名称 | 星级 | 流派 | 费用 | 技能 |
| --- | --- | --- | --- | --- | --- |
| zhengqi-geming | 蒸汽革命 | 5 | 通用 | 3 | trigger-all-friendly-plus-1-plus-1 |
| zhuanli-zhidu | 专利制度 | 4 | 通用 | 2 | trigger-all-mech-growth-once |

- [ ] **Step 3: 更新脚本中间期望值**

```python
EXPECT_ERA = {"史前": 27, "原始": 26, "古典": 14, "封建": 27, "工业": 14, "现代": 36, "未来": 20}
EXPECT_TRIBE = {"学者": 33, "生物": 28, "人类": 34, "机械": 30, "植物": 20, "病毒": 19}
EXPECT_LEVEL = {1: 22, 2: 37, 3: 37, 4: 28, 5: 25, 6: 15}
# 类型计数 {"随从": 164, "效果": 36}
```

- [ ] **Step 4: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0；末行 `随从 164 张、效果牌 36 张、技能 118 个、英雄 13 位、成就 6 个，校验通过`。

- [ ] **Step 5: 提交**

```bash
git add docs/current/product/content/cards scripts/validate_docs.py
git commit -m "feat: 补齐工业社会卡牌"
```

---

### Task 8: 七星级内容（12 随从）

**Files:**
- Create: `docs/current/product/content/cards/<id>.md`（12 份）
- Modify: `scripts/validate_docs.py`

**Interfaces:**
- Consumes: Task 7 的 164/36 基线。
- Produces: 最终期望值 176 随从 / 36 效果；7 星池 12 张。

- [ ] **Step 1: 按清单创建随从文档**

| id | 名称 | 流派 | 子类 | 攻/血 | 技能 | 来源 |
| --- | --- | --- | --- | --- | --- | --- |
| schrodinger | 薛定谔 | 学者 | 量子 | 0/26 | keyword-divine-shield, combatstart-random-enemy-contagion-2 | 新增 |
| heisenberg | 海森堡 | 学者 | 量子 | 0/26 | keyword-reborn, deathrattle-all-enemies-minus-1-health | 素材 |
| fermi | 费米 | 学者 | 核物理 | 0/26 | battlecry-all-friendly-growth-once | 素材 |
| dyson-sphere | 戴森球 | 机械 | 巨构 | 12/18 | keyword-divine-shield | 新增 |
| qulv-feichuan | 曲率飞船 | 机械 | 航天 | 12/19 | keyword-optimal-attack | 新增 |
| nami-fengqun | 纳米蜂群 | 机械 | 纳米 | 11/18 | keyword-assemble-2 | 新增 |
| xingji-jianzhang | 星际舰长 | 人类 | 舰长 | 12/19 | keyword-taunt | 新增 |
| jiyin-gaizaoren | 基因改造人 | 人类 | 改造 | 12/18 | keyword-reborn | 新增 |
| yongsheng-shuimu | 永生水母 | 生物 | 海洋 | 10/20 | keyword-reborn | 新增 |
| zhihui-jushou | 智慧巨兽 | 生物 | 巨兽 | 12/18 | keyword-windfury | 新增 |
| yuzhouteng | 宇宙藤 | 植物 | 宇宙 | 10/19 | keyword-divine-shield | 新增 |
| nami-bingdu | 纳米病毒 | 病毒 | 纳米 | 11/19 | keyword-contagion | 新增 |

所有随从 `level: 7`、`era: 未来`。多技能卡的 `## 效果` 按 skills 顺序用「；」拼接技能 text。

- [ ] **Step 2: 核算每张超模**

逐张计算 `攻击 + 生命 + 关键词折算 ≤ 33`：戴森球 32、曲率飞船 33、纳米蜂群 33、星际舰长 31.5、基因改造人 33、永生水母 33、智慧巨兽 33、宇宙藤 31、纳米病毒 32。学者跳过该项，但生命值必须为 `20 × 1.3 = 26`。

- [ ] **Step 3: 更新脚本最终期望值**

```python
EXPECT_ERA = {"史前": 27, "原始": 26, "古典": 14, "封建": 27, "工业": 14, "现代": 36, "未来": 32}
EXPECT_TRIBE = {"学者": 36, "生物": 30, "人类": 36, "机械": 33, "植物": 21, "病毒": 20}
EXPECT_LEVEL = {1: 22, 2: 37, 3: 37, 4: 28, 5: 25, 6: 15, 7: 12}
# 类型计数 {"随从": 176, "效果": 36}
```

- [ ] **Step 4: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0；末行 `随从 176 张、效果牌 36 张、技能 118 个、英雄 13 位、成就 6 个，校验通过`；星级输出含 `7: 12`。

- [ ] **Step 5: 提交**

```bash
git add docs/current/product/content/cards scripts/validate_docs.py
git commit -m "feat: 补齐七星级终局卡牌"
```

---

### Task 9: 卡牌规范、平衡测算与技能索引

**Files:**
- Modify: `docs/current/product/content/card-system.md`
- Modify: `docs/current/product/content/balance-analysis.md`
- Modify: `docs/current/product/content/skills/README.md`

**Interfaces:**
- Consumes: Task 5 的诊断输出、Task 8 的最终卡池。
- Produces: 与卡池一致的统计与引用数。

- [ ] **Step 1: 更新 card-system.md §3 数据总览**

| 维度 | 新值 |
| --- | --- |
| 总数 | 212（随从 176 / 效果 36） |
| 按时代 | 史前 27 / 原始 26 / 古典 14 / 封建 27 / 工业 14 / 现代 36 / 未来 32 |
| 按流派 | 学者 36 / 生物 30 / 人类 36 / 机械 33 / 植物 21 / 病毒 20 |
| 按星级 | 1 星 22 / 2 星 37 / 3 星 37 / 4 星 28 / 5 星 25 / 6 星 15 / 7 星 12 |
| 按来源 | 素材 53 / 新增 123 |

效果牌流派：通用 20 / 生物 4 / 学者 4 / 病毒 3 / 机械 2 / 人类 2 / 植物 1。

- [ ] **Step 2: 用诊断输出回写 §3.1 关键词分布**

Run: `python3 scripts/validate_docs.py | grep "^关键词"`
Expected: 得到 13 个关键词的最新张数。把输出逐项写入 §3.1 表格；`成长` 仍按效果文本统计，其余 12 词按技能 `keyword` 统计。保留专属词下限说明。

- [ ] **Step 3: 更新 card-system.md 的星级字段与新增卡规则**

- `level` 约束 `1–6` 改为 `1–7`；7 星数值区间与份数指向 `economy-and-numbers.md`。
- §4 的"新增 67 张"改为"新增 123 张"，并说明 40 张来自七时代扩展（古典 14、工业 14、7 星 12）。
- 校验说明中的期望末行改为 176/36。

- [ ] **Step 4: 更新 balance-analysis.md**

- §2 矩阵增加 7 星行，数值白板取 12/17；读表结论补一句"7 星为终局卡，样本量 12"。
- §1 的卡池总量、星级区间引用改为新值。
- §3 的八回合推演保留，但在节首标注：推演基于 5 时代口径，七时代与 7 星需 M0 模拟器重跑。
- §4 失衡判据增加"7 星学者占比"一行。

- [ ] **Step 5: 回写 skills/README.md 引用数**

Run: `python3 scripts/validate_docs.py | grep "^技能引用"`
Expected: 得到每个技能的最新引用数。只更新被新卡引用的行（约 24 行），其余不动。

- [ ] **Step 6: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0。

Run: `grep -n "136\|168\|1–6" docs/current/product/content/card-system.md`
Expected: 无输出。

- [ ] **Step 7: 提交**

```bash
git add docs/current/product/content
git commit -m "docs: 回写七时代卡池统计与平衡口径"
```

---

### Task 10: 版本文档与待确认事项

**Files:**
- Modify: `docs/versions/README.md`
- Modify: `docs/version-roadmap.md`
- Modify: `docs/current/product/content/hero-system.md`
- Modify: `docs/current/product/systems/modes-and-achievements.md`
- Modify: `docs/current/product/rules/README.md`
- Modify: `docs/versions/v0.1/open-questions.md`

**Interfaces:**
- Consumes: Task 1–9 的最终口径。
- Produces: 版本入口与路线图不再引用 5 时代。

- [ ] **Step 1: versions/README.md 与规则索引**

- `docs/versions/README.md`：v0.1 描述改为「七时代、六流派、212 张卡与 13 位英雄」。
- `docs/current/product/rules/README.md`：「五时代推进」改为「七时代推进」。

- [ ] **Step 2: version-roadmap.md 改规划口径**

- §1「首版必须一次做齐五个时代」改为「七个时代」；开发顺序补上古典与工业两段。
- §3 内容增量表的卡池基线改为 176 + 36；后续版本仍按 +20 递增。
- §6「先 1 个时代，后 5 个时代」改为「后 7 个时代」。
- §2 人周估算不改，但在表下补一句：七时代内容与校验工作量未计入，需在 M0 前重估。

- [ ] **Step 3: 坐镇英雄口径**

- `hero-system.md` 与 `modes-and-achievements.md` 中「每个时代可选一位英雄坐镇」的注释补一句：七时代对应 7 位坐镇候选；核心英雄仍为 13 位，不新增。
- 不改英雄实体文档与技能。

- [ ] **Step 4: open-questions.md 新增待确认项**

| 问题 | 影响范围 | 待确认 | 来源 |
| --- | --- | --- | --- |
| 7 星池每张 5 份在 8 人局是否过少 | 平衡 | M0 模拟后校准 | [数值与经济](../../current/product/systems/economy-and-numbers.md) |
| 升满七段的实测回合是否落在 12–14 | 节奏 | M0 模拟后校准 | [数值与经济](../../current/product/systems/economy-and-numbers.md) |
| 古典段一次开放三个关键词是否让前期复杂度陡增 | 平衡 | 观察新手首局 | [关键词与机制](../../current/product/rules/keywords-and-mechanics.md) |
| 七时代是否需要重估 22–31 人周 | 排期 | M0 前重估 | [版本规划与风险](../../version-roadmap.md) |

- [ ] **Step 5: 验证**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0。

Run: `grep -rn "五时代\|五个时代" docs/versions docs/version-roadmap.md --include="*.md"`
Expected: 仅历史计划或明确标注旧口径的句子命中；否则逐条改写。

- [ ] **Step 6: 提交**

```bash
git add docs/versions docs/version-roadmap.md
git commit -m "docs: 版本规划与待确认项同步七时代"
```

---

### Task 11: 全量校验与旧口径清理

**Files:**
- Modify: 按检索结果逐条修正

**Interfaces:**
- Consumes: Task 1–10 的全部产物。

- [ ] **Step 1: 全量校验**

Run: `python3 scripts/validate_docs.py`
Expected: 退出码 0；末行 `随从 176 张、效果牌 36 张、技能 118 个、英雄 13 位、成就 6 个，校验通过`。

- [ ] **Step 2: 旧口径检索**

Run: `grep -rn "五时代\|五个时代\|星级 1–6\|时代 1–5\|升到 5 时代" docs/versions docs/version-roadmap.md --include="*.md"`
Expected: 无输出；若命中历史规划文档，改写或在该句后加「（旧口径，已废弃）」。`docs/superpowers/` 下的设计与计划文档属于过程记录，不在检索范围。

- [ ] **Step 3: 相对链接校验**

创建 `output/check_links.py`（`output/` 已被忽略，不提交）：

```python
import pathlib, re
root = pathlib.Path("docs")
bad = []
for p in root.rglob("*.md"):
    for m in re.finditer(r"\]\(([^)#]+\.md)(#[^)]*)?\)", p.read_text(encoding="utf-8")):
        target = (p.parent / m.group(1)).resolve()
        if not target.exists():
            bad.append(f"{p}: {m.group(1)}")
print("\n".join(bad) if bad else "links ok")
```

Run: `python3 -I output/check_links.py`
Expected: `links ok`。随后删除该脚本。

- [ ] **Step 4: 工作区自检**

Run: `git status --short`
Expected: 只包含本次任务范围内的改动；`output/`、临时脚本和计划目录不出现。

- [ ] **Step 5: 提交**

```bash
git add docs/versions scripts docs/version-roadmap.md
git commit -m "fix: 清理残留的五时代口径"
```
