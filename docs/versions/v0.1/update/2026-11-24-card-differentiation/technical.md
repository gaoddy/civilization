方案未实施

# 卡池差异化：技术方案

版本：v0.1
日期：2026-11-24
产品规则：[product.md](product.md)（已确认）

## 1. 改动边界

只改内容数据、校验脚本与文档，不改规则引擎、房间服务、协议与客户端代码。客户端展示数据来自生成物，代码零改动。

不改：`server/crates/*`、`protocol/`、`client/*.gd`、`client/*.tscn`、金币与升时代、商店概率、牌池份数、伤害曲线、三连与发现。

## 2. 数据与规则落点

### 2.1 星级数值区间

`scripts/validate_docs.py` 的 `RANGE`（约第 157 行）改为：

```python
RANGE = {1: (1, 3, 2, 5), 2: (2, 4, 3, 6), 3: (3, 6, 4, 8),
         4: (4, 8, 6, 10), 5: (6, 10, 8, 13), 6: (8, 12, 10, 16),
         7: (10, 15, 14, 20)}
```

参考白板与「白板 + 4」上限不动，攻 0 生命值校验口径 `|生命值 − 上限×1.3| ≤ 1` 不动。

### 2.2 新增分布校验（`scripts/validate_docs.py`）

在卡牌校验阶段新增四项，作用于 `in_shop_pool: true` 的随从；多归属卡在每条 `play:` 下分别计数；违例逐条打印并以退出码 1 结束。

| 规则 | 判定 |
| --- | --- |
| 身材 | `攻击力 > 0` 时，`(play, 星级, 攻击力, 生命值)` 出现次数 ≤ 2 |
| 效果技能集 | `(play, 星级, 效果技能集)` ≤ 1；效果技能集 = `skills` 中剔除 id 以 `keyword-` 开头的技能后的有序元组 |
| 纯关键词卡 | 效果技能集为空时，`(play, 星级, 攻击力, 生命值, skills)` ≤ 1 |
| 攻 0 卡 | `(play, 星级, skills)` ≤ 1，跳过身材判定 |

关键词类技能以 `keyword-` 前缀识别（当前 17 个：taunt / sow / contagion / ignore-taunt / windfury / armor 1–2 / assemble 1–2 / breed 1–3 / poison / reborn / divine-shield / stealth / decompose / evolve）。`battlecry-*` 等带 `keyword: battlecry` 的技能属于效果技能，不剔除。

校验通过时打印四项统计（违规组数与涉及卡数均为 0），供文档引用。

改前基线（用于证明校验生效）：身材 23 组 93 条、效果技能集 20 组 46 条、纯关键词卡 9 组 20 条、攻 0 卡 3 组 8 条、效果牌完全重复 3 组。

### 2.3 攻 0 随从生命值

49 张攻 0 卡按新上限重算，取值落在 `上限 × 1.3 ± 1`：

| 星级 | 新上限 | 可取生命值 | 现有值 |
| --- | --- | --- | --- |
| 1 | 5 | 6、7 | 5 |
| 2 | 6 | 7、8 | 6、7 |
| 3 | 8 | 10、11 | 9 |
| 4 | 10 | 12、13、14 | 12 |
| 5 | 13 | 16、17、18 | 16 |
| 6 | 16 | 20、21 | 20 |
| 7 | 20 | 25、26、27 | 26 |

同一玩法同星级内按可用档位分摊，完整技能集保持唯一。

### 2.4 永久嘲讽：新增技能实例（不改引擎）

用现有模板新增 4 个技能文档 `content/skills/<id>.md`，并分配到爬行纲、水螅纲、机械、植物界各 1 张随从（原始或更晚时代）：

| 技能 id | 模板 | 文案 |
| --- | --- | --- |
| `trigger-friendly-permanent-taunt` | `on-play-keyword-single(keyword=taunt,duration=permanent)` | 战吼：使一个友方随从永久获得嘲讽 |
| `trigger-reptile-permanent-taunt` | `on-play-keyword-single-ps(playstyle=reptile,keyword=taunt,duration=permanent)` | 战吼：使一个友方爬行纲随从永久获得嘲讽 |
| `trigger-cnidarian-permanent-taunt` | `on-play-keyword-single-ps(playstyle=cnidarian,keyword=taunt,duration=permanent)` | 战吼：使一个友方水螅纲随从永久获得嘲讽 |
| `aura-plant-taunt` | `aura-keyword-all-ps(playstyle=plant,keyword=taunt)` | 你的植物界随从获得嘲讽 |

- 嘲讽自原始时代解锁；随从经由效果获得的常驻关键词同样受限，承载卡必须为原始或更晚时代。
- 光环不是关键词，不计关键词折算，强度由星级区间与 `白板 + 4` 约束。
- 技能实例不带 `name` 以外的新字段；`## 属性` 与 `## 效果` 由 `make docs-render` 生成。
- 技能总数由 255 变为 259；`scripts/validate_docs.py:1173` 的技能数量期望改为 259，`card-system.md` §5 的期望输出与 `skills/README.md` 索引同步更新。

### 2.5 效果牌完全重复

3 组各改其中 1 张：`trigger-all-friendly-plus-1-plus-1`（5 星 3 费）、`trigger-friendly-growth-twice`（3 星 2 费）、`trigger-all-mech-growth-once`（4 星 2 费）。改用同模板不同参数或同费用档位的既有技能实例。

## 3. 卡牌改动流程（每批一致）

1. 只改卡牌 front matter 的 `attack`、`health`、`skills`，以及 `## 设计说明` 的数值推导文字。
2. `make docs-render` 重写属性表与效果正文派生行；`--check` 确认无漂移。
3. `python3 scripts/validate_docs.py` 通过四项新校验。
4. `make data` 重新生成 `server/data/v0.1/cards.json`、`client/text/` 等。
5. `make balance-check` 与 `make data-check` 通过。
6. 按批提交。

## 4. 文档回写

| 文档 | 改动 |
| --- | --- |
| `docs/current/product/systems/economy-and-numbers.md` §5 | 1–5 星区间表、攻 0 取值说明 |
| `docs/current/product/content/card-system.md` §2、§3、§5 | 新增分布规则与校验说明；更新数据总览与期望输出 |
| `docs/current/product/content/balance-analysis.md` §1–§6 | 按新数据复算攻血比、身材表、技能密度、模拟结果 |
| `docs/current/product/content/balance-questions.md` | 增补本轮问题与裁定 |
| `docs/current/product/content/skills/README.md` | 由 `make docs-render` 更新索引与计数 |
| `docs/current/product/content/heroes/README.md`、`cards/README.md` | 由生成脚本更新卡表 |

全部实现并回写后删除 `update/2026-11-24-card-differentiation/`。

## 5. 验证矩阵

| 项 | 命令 |
| --- | --- |
| 文档与数据一致 | `make data-check` |
| 分布与区间校验 | `python3 scripts/validate_docs.py` |
| 平衡报告 | `make balance-check` |
| 全量检查 | `make check` |
| 模拟基线 | `make sim-batch && make sim-report` |

判定：全部退出码 0；13 玩法攻血比在目标带 ±0.04，极差 ≥ 0.20；模拟 `avg_turns` 20、13 玩法均有夺冠、哺乳纲 ≤ 376‰。

## 6. 实施顺序

按玩法分批，每批独立提交、独立验证：

1. 哺乳纲（重复最多：身材 23 组中占多数）
2. 植物界（技能集重复 41%）
3. 病毒
4. 爬行纲
5. 机械
6. 其余玩法 + 攻 0 卡生命重算
7. 效果牌 3 组 + 永久嘲讽 4 张
8. 文档回写与平衡复算

多归属卡在首批改动时即调整到位，后续批次只复核不重复改。

## 7. 风险与回滚

- 每批一个提交，模拟或胜率恶化时按批回退，不整分支回滚。
- 1–3 星是模拟胜率唯一有效调参窗口：先按形状重排，保持攻 > 0 卡平均总身材不上升；胜率下降时优先换技能，不抬数值。
- `server/data/v0.1/cards.json` 与文本是生成物，回退后重跑 `make data`。

## 8. 待确认

无。
