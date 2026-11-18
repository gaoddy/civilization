# 卡牌参数重排与玩法差异化 · 技术方案

> 状态：方案未实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 数据内容 | 309 张随从的攻击力、生命值与技能组合；63 张新效果牌；配套技能文档；3 张 2 星双技能卡合并 |
| 文档校验 | 效果牌允许 1 个 `play:` 关联；技能数量上限；卡牌/技能/效果牌数量期望；身材与效果牌占比的目标校验 |
| 规则引擎 | 不新增原语、关键词或触发时机；`card_in_pool` 已支持中立卡按 `play:` 关联过滤，只补测试锁住行为 |
| 模拟器 | AI 打出效果牌；购买时跳过与自己主导玩法不匹配的定向牌；按批次重定金标准；跑 1000 局报告 |
| 客户端/协议/房间服务 | 不改代码；文本表与客户端卡牌数据随 `make data` 重新生成 |
| 文档 | 卡牌规范、数值与经济、平衡测算、卡牌索引、技能索引、分类统计同步 |

不涉及：金币曲线、升时代费用、商店概率、牌池份数、伤害公式、回合上限、英雄技能机制、玩法数量。

## 2. 数据契约

### 2.1 效果牌的玩法关联

现有 `rule_engine::data::card_in_pool` 已经按以下口径处理中立卡：

```rust
// server/crates/rule-engine/src/data.rs
if card.playstyle != Playstyle::Neutral {
    return enabled.contains(&card.playstyle);
}
let associations = card.specialty.iter()
    .filter(|specialty| specialty.kind == SpecialtyKind::Play);
// 有 play: 时至少一个命中本局玩法；没有 play: 时始终在池中
```

`recruit.rs::is_eligible` 对随从与效果牌都调用 `card_in_pool`，因此**引擎不需要改动**。本次只放开数据校验并补测试：

- 定向效果牌：`playstyle: neutral`、`taxon: phenomenon`、`specialty: [play:<playstyle>]`，恰好 1 条。
- 通用效果牌：`playstyle: neutral`、`taxon: phenomenon`、`specialty` 为空。
- 效果牌不得出现 `prey:` / `host:`，也不得出现第二条 `play:`。
- 属性表新增「关联分类」行，值为「关联：<玩法名>」；通用牌不写该行。`expected_card_attrs` 已按 `specialty_display` 自动处理，无需改渲染逻辑。
- **有效玩法**：新增 `effective_playstyle(doc)`。效果牌带 `play:<id>` 时返回该玩法，否则返回 `doc["playstyle"]`。`keyword_errors`、`mechanism_errors` 与 `PICK_PLAYSTYLE` 校验一律使用有效玩法，否则「毒牙」「硬化甲壳」「播种季」「俯冲编队」这类中立定向牌会被误判为越权。随从仍按自身 `playstyle` 校验。

### 2.2 技能数量

`scripts/validate_docs.py` 增加随从技能数量校验：

| 星级 | 上限 | 附加规则 |
| --- | --- | --- |
| 1–2 | 1 | 现有 2 星双技能卡必须在本次内容改动中合并 |
| 3–4 | 2 | 第二个技能必须来自该玩法机制词汇，由人工评审检查 |
| 5–7 | 3 | 其中最多 2 个技能登记关键词 |

- 无技能随从只允许是中立非学者工具卡，且全池恰好 1 张（`einstein-fridge`）。
- 现有 `validate_docs.py` 的 `skillless != 7` 改为 `skillless != 1`；其余 6 张各补 1 个既有技能，不做新技能：
  - `nitaizhangyu`（头足纲 2 星）→ `mimic-1`
  - `shizuniao`（鸟纲 1 星）→ `dive-1`
  - `sibadayongshi`（哺乳纲 2 星）→ `keyword-taunt`
  - `tufu`（哺乳纲 2 星）→ `battlecry-friendly-plus-0-plus-1`
  - `xiake`（哺乳纲 3 星）→ `keyword-windfury`
  - `yuanshijixiekuilei`（机械 3 星）→ `keyword-assemble-1`
- 关键词时代解锁校验保持不变；本次不新增关键词，也不改变任何关键词的最小星级。2 星合并时：
  - `sea-anemone` 保留 `keyword-armor-1`，移除 `keyword-taunt`；`armor 1` 的最小星级仍为 2。
  - `juelei` 保留 `keyword-breed-2`，移除 `keyword-sow`；`breed 2` 的最小星级仍为 2。
  - `shuishengjue` 保留 `keyword-breed-1`，移除 `keyword-sow`；`breed 1` 的最小星级仍为 1。

### 2.3 数量期望

`scripts/validate_docs.py` 与 `scripts/balance_report.py` 使用以下期望：

| 项 | 期望 |
| --- | --- |
| 随从 | 309（学者 49、英雄 32 不变） |
| 效果牌 | 100（既有 37 + 新增 63） |
| 技能 | 实施后按实际生成数固定；新增技能只增不改 |
| 效果牌按星级 | 10 / 18 / 22 / 20 / 16 / 9 / 5 |
| 效果牌按费用 | 0 费 8、1 费 33、2 费 34、3 费 19、4 费 6 |
| 每星级效果牌占比 | ≤ 30% |
| 定向效果牌 | 39（13 玩法 × 3） |
| 通用效果牌 | 61（既有 37 + 新增 24） |

`validate_docs.py` 当前三处硬编码需要同步：

- `len(skills) != 197` → 实际技能数。
- `len(skillless) != 7` → 1。
- `counts != {"minion": 309, "effect": 37}` → `{"minion": 309, "effect": 100}`。

## 3. 实施方式

### 3.1 源与生成物

- 卡的源是 `docs/versions/v0.1/product/content/cards/*.md` 与 `skills/*.md`；`server/data/v0.1/*.json`、`client/text/zh_CN.csv` 与客户端卡牌数据都是生成物。
- 不手工编辑 `server/data/v0.1/*.json`。每次内容改动后跑 `make data`，用 `make data-check` 确认生成物无漂移。
- 卡牌 id、卡名、时代、星级、科学分类与技能文本不变；只改 `attack`、`health`、`skills`，以及新增效果牌与技能。

### 3.2 一次性迁移脚本

全量重排涉及 300 多份卡牌文档，手工逐张改容易漏属性表。实施时写两个一次性脚本，放在 `output/`（已被忽略，不提交）：

1. `output/rebalance_cards.py`
   - 输入：`card_id -> {attack, health, skills}` 的映射（按玩法和批次生成）。
   - 更新：front matter 的 `attack`、`health`、`skills`；`## 属性` 表的攻击力、生命值、关键词、技能行；`## 效果` 按新 `skills` 从技能文档重渲染。
   - 不更新：`## 设计说明` 与 `## 出处`；需要改定位说明的关键卡由人工补写。
2. `output/gen_effect_cards.py`
   - 输入：63 张效果牌的定义（id、卡名、费用、星级、时代、技能、`play:` 关联、出处）。
   - 输出：完整的卡牌文档；技能文本复用既有技能或由新技能文档提供。

脚本只做机械改写；每批改动后人工抽查 5–10 张卡的属性表与效果行。

### 3.3 分批

| 批次 | 内容 | 结束条件 |
| --- | --- | --- |
| 0 | 校验规则、`balance_report.py`、技能实体、2 星合并、测试 | `make data-check` 通过，卡牌数量仍为 309/37 |
| 1 | 模拟器 AI 打出效果牌 | 模拟金标准重定，`make test` 通过 |
| 2 | 哺乳纲 63 张 + 中立非学者 28 张 | 该批玩法攻血比与技能密度达到目标；模拟金标准重定 |
| 3 | 鸟、爬行、鱼、昆虫、头足、水螅（6 个玩法） | 同上 |
| 4 | 植物、真菌、细菌、原生、病毒、机械（6 个玩法） | 同上；全局技能密度达标 |
| 5 | 39 张定向效果牌 + 24 张通用效果牌 + AI 定向购买过滤 | 效果牌 100 张，星级/费用/占比校验通过；模拟金标准重定 |
| 6 | 模拟调参、最终金标准、文档回写 | 1000 局报告达到产品验收区间 |

每批单独提交；批次 2–5 不合并成一个大提交，便于评审和回滚。

## 4. 校验与测试

### 4.1 新增校验脚本

新增 `scripts/balance_report.py`，读取 `server/data/v0.1/cards.json`，输出并校验：

- 每个玩法的非学者随从数量、平均攻血比、各星级平均攻血比。
- 3–4 星与 5–7 星的技能密度（带 2 个及以上技能的比例）。
- 效果牌总数、按星级/费用分布、每星级占比、定向/通用数量。
- `--check` 模式：任一项越界时退出码 1。

`make balance-check` 单独运行，不并入 `make data-check`：批次 1–4 期间只有部分玩法完成重排，整表平衡校验必然失败，若并入数据校验会让每个中间提交都无法通过。最终任务在冻结数值后运行 `make balance-check`。

新增 `tests/test_balance_report.py`：用合成卡牌数据验证报告与越界判定（目标带、攻血比差、技能密度、效果牌星级/费用/占比、定向牌数量），不读取真实卡表，因此中间批次也能保持 `make test` 通过。

### 4.2 引擎测试

在 `server/crates/rule-engine/tests` 增加：

- `effect_card_playstyle_filter`：带 `play:fish` 的效果牌在未启用鱼纲时不可抽取，启用后可抽取；无 `play:` 的通用效果牌始终可抽取。
- 复用 `recruit` 的商店抽取入口或直接测 `card_in_pool`；两种口径都要覆盖，避免只测数据层。

### 4.3 验证矩阵

| 命令 | 覆盖 |
| --- | --- |
| `make data` / `make data-check` | 生成物无漂移、效果牌与技能数量、平衡目标 |
| `make test` | Python 测试与 Rust 全工作区测试（含新增 `card_in_pool` 测试） |
| `make lint` | fmt + clippy |
| `make m0-determinism` | 重放确定性 |
| `make m0-batch` / `make m0-report` | 1000 局胜率、回合数、存活人数 |
| `make protocol-check` / `make protocol-rs-check` | 协议生成物无漂移（本次不改协议） |
| `make client-check` / `make client-smoke` | 文本表、卡牌数据、卡面渲染 |
| `make m1b-e2e` | 8 人局端到端（内容改动后的冒烟） |

## 5. 模拟与调参

### 5.1 机器人适配

当前 `sim/src/ai.rs` 按「买得起的最贵卡」购买，且**从不打出效果牌**：效果牌买入后进入手牌，`choose_fill_board` 只找随从，效果牌会一直占用 `hand_slots`，手牌满后 AI 停止购买。37 张时影响有限，100 张时会让模拟严重失真。本次增加两条只影响模拟器的规则：

- 在「填满棋盘」之后、「购买」之前增加 `choose_play_effect`：按手牌下标顺序尝试打出效果牌。先试不带目标（群体/自身效果），被引擎拒绝后改试己方最左随从，两个版本都被拒再换下一张手牌。
- 购买效果牌时，若该牌带 `play:` 关联且玩家主导玩法不是 `Neutral` 且不在关联内，跳过该槽位；主导玩法为 `Neutral`（棋盘还空）时照常购买，避免开局买不到定向牌。

两条规则分两步落地：**打出效果牌必须先于随从重排**（否则批次调参基于「买而不打」的失真 AI）；购买过滤依赖定向效果牌，放在效果牌扩充之后。

`dominant_playstyle` 已存在，可直接复用；该改动只影响 `sim` crate，不进入规则引擎与房间服务。买家仍可能买到「当前无合法目标」的效果牌，引擎拒绝后该牌留在手牌；`choose_play_effect` 会跳过被拒版本，继续尝试其它效果牌。

### 5.2 调参顺序

1. 先跑 1000 局，记录每个玩法夺冠率、平均回合、第 20 回合存活人数。
2. 对低于 4% 的玩法先检查是否被身材曲线压得过低（攻击/生命），再检查技能密度是否不足。
3. 对高于 12% 的玩法先检查 5–7 星组合是否过强，再检查关键词折算是否低估。
4. 单次只改一个玩法的一个维度（身材或技能），每轮重跑；避免同时改多个变量无法归因。
5. 达到区间后冻结数值，重定金标准。

### 5.3 金标准重定

- `server/crates/rule-engine/tests/combat.rs` 的 `state_hash` 常量（只有共享池条目数或哈希字段变化时才变，按测试结果决定是否更新）。
- `server/crates/sim/tests/golden/seed-1-games-20.json`。
- `server/crates/sim/tests/sim_determinism.rs` 中的重定说明注释。

`make test` 包含两项金标准测试，不能用跳过测试来回避。因此每个会改变对局的内容批次和模拟器 AI 改动之后都重定一次模拟金标准，注释写明本批原因；最终调参冻结后再重定一次并写明「最终冻结」。每次重定用 `sim batch --games 20 --seed 1` 的输出哈希重写基线文件，不手工改写。

## 6. 文档回写

| 文档 | 改动 |
| --- | --- |
| `docs/versions/v0.1/product/content/card-system.md` | 效果牌允许 1 个 `play:` 关联；数量统计 346 → 409；效果牌 37 → 100；技能数量上限 |
| `docs/versions/v0.1/product/systems/economy-and-numbers.md` | 关键词折算口径与本次实际参数带对齐；4 费效果牌允许 6–7 星 |
| `docs/versions/v0.1/product/content/balance-analysis.md` | 按新基线重算攻血比矩阵、玩法对照与失衡判据 |
| `docs/versions/v0.1/product/content/cards/README.md` | 卡牌索引数量与新增效果牌条目 |
| `docs/versions/v0.1/product/content/skills/README.md` | 技能索引与引用数 |
| `docs/versions/v0.1/product/content/taxonomy.md` | 分类节点统计（若新增 effect 的 taxonomy 无需变化则只更新数量） |
| `docs/current/technical/rule-engine-and-simulator.md` | 效果牌按玩法过滤的既有行为说明；模拟器 AI 适配 |

## 7. 风险与回滚

- **大 diff**：按批次提交，每批只改一个玩法组或一类内容；评审只看本批 diff。
- **关键词最小星级漂移**：新增/合并技能时先跑 `make data-check`，确认 `armor`/`breed`/`poison`/`stealth` 的最小星级未前移。
- **效果牌占比过高**：批次 5 完成后立即运行 `make balance-check`；超出 30% 时调整该批效果牌的星级分布。批次 2–4 用 `--playstyle` 只检查已完成重排的玩法。
- **模拟金标准重定**：`make test` 包含金标准测试，不能跳过；每个内容批次与 AI 改动后各重定一次，最终冻结时再重定一次。重定只改哈希与注释，不改测试逻辑。
- **回滚**：全部改动是内容与文档，无运行时数据迁移；回滚分支即可。已合并后回滚需同时恢复 `server/data/v0.1/*.json`、`client/text/zh_CN.csv` 与客户端卡牌数据，直接回退对应提交。
- **客户端文本**：新增效果牌与技能会新增文本 key；`make data` 重新生成 `client/text/zh_CN.csv`，`make client-check` 验证无缺失。
