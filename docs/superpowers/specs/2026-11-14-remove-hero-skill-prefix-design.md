# 去掉 `hero-` 技能前缀设计

> 状态：已确认，待实施
> 前置：[移除 `hero_skill` 字段](2026-11-14-remove-hero-skill-field-design.md)

## 1. 目标

上一轮把英雄技能并入卡的 `skills`，但 13 个技能 id 仍带 `hero-` 前缀，校验器也靠这个前缀守「每张英雄卡恰好引用 1 个英雄技能」。本设计继续收敛：**技能 id 不再带 `hero-`，校验器不再识别「英雄技能」这个类别。**

## 2. 已确认的取舍

讨论过三种落法：

| 方案 | 做法 | 结论 |
| --- | --- | --- |
| A. 取消英雄技能概念 | 前缀去掉后不再反推「哪个是英雄技能」；校验只保留 13 张 `is_hero` 与旧字段残留检查 | **采用** |
| B. 保留一对一映射 | 继续要求每张英雄卡恰好 1 个「仅它自己引用」的技能 | 未采用：靠共享度反推，未来出现共享技能会误报 |
| C. 加 `hero_only` 标记 | 给技能加字段保住映射 | 未采用：与「英雄不特殊」方向相反，多一个字段 |

方案 A 的直接后果：**没有任何机器可判定的「英雄专属技能」标记。** 英雄卡就是带 `is_hero` 的普通随从卡，校验不关心它带了哪些技能。

## 3. 技能 id 重命名

逐个去掉前缀，13 个新 id 均与现有 105 个技能 id 无冲突：

| 原 id | 新 id | 显示名 |
| --- | --- | --- |
| hero-alternating-current | alternating-current | 交流电 |
| hero-division-of-labor | division-of-labor | 分工 |
| hero-efficient-algorithm | efficient-algorithm | 高效算法 |
| hero-evolution-discovery | evolution-discovery | 进化 |
| hero-faraday-cage | faraday-cage | 法拉第笼 |
| hero-heredity | heredity | 遗传 |
| hero-invention-factory | invention-factory | 发明工厂 |
| hero-light-speed-strike | light-speed-strike | 光速出击 |
| hero-microbiology | microbiology | 微生物学 |
| hero-observation | observation | 观测 |
| hero-radiation | radiation | 放射 |
| hero-shout | shout | 呐喊 |
| hero-universal-manuscript | universal-manuscript | 万能手稿 |

同步修改：技能文档文件名、front matter `id`、`## 属性` 的 `id` 行；13 张英雄卡 `skills` 与属性表「技能」行；`skills/README.md` 索引。技能 `text`、`effects`、`cost`、`uses_per_turn` 全部不变，因此卡牌 `## 效果` 不变。

## 4. 校验器变更

删除：

- 常量 `HERO_SKILL_PREFIX`。
- `skill_field_errors` 中 `hero-*` 技能强制 `name` / `match_text` 的分支。
- `check_hero_cards` 中「每张英雄卡恰好引用 1 个技能」「每个技能恰好被 1 张英雄卡引用」的成对规则。

保留：

- 英雄卡恰好 13 张；非英雄随从与效果牌不得有 `is_hero`。
- 卡牌不得残留 `LEGACY_HERO_FIELDS` 或 `## 英雄技能` 小节。
- 主动技能 `cost` / `uses_per_turn` 成对、取值合法、`name` 与 `match_text` 必填。
- 被效果牌引用的技能必须有 `match_text`。

`check_hero_cards` 简化为：计数 13、方向性检查 `is_hero`、旧字段与小节残留。

## 5. 文档变更

- `skill-system.md` §3.2：英雄卡与普通卡一样只通过 `skills` 引用技能，无前缀约定；§6 第 7 条改为「英雄卡 13 张、无已废弃字段或小节残留」。
- `card-system.md` §2.1、§5：去掉「`hero-*` 技能」与成对映射表述。
- `cards/README.md`：英雄卡小节改为「技能就是卡 `skills` 里的技能，无前缀区别」。
- `skills/README.md`：索引中 13 行改为新 id；「13 个 `hero-*` 技能」表述删除。
- `game-intro.md` §2.10：改为「每张英雄卡在 `skills` 中带一个主动或被动技能」。
- `keywords-and-mechanics.md` §9.3：原语注释 `hero-*` 技能专用 → 主动技能专用；§9.1 删去「英雄技能也在其中」。

## 6. 数据结果

| 维度 | 变更后 |
| --- | --- |
| 卡牌 | 220（随从 184 含 13 张英雄卡 / 效果 36） |
| 技能 | 118，无 `hero-` 前缀 |
| 无技能随从 | 7 |
| 卡牌技能引用 | 247 段（随从 211 + 效果 36） |
| 被引用的唯一技能 | 118 |

校验末行不变：`随从 184 张、效果牌 36 张、技能 118 个、英雄卡 13 张、成就 6 个，校验通过`。

## 7. 验收标准

1. 全仓无 `hero-` 前缀的技能 id，也无 `HERO_SKILL_PREFIX`（历史设计文档除外）。
2. 13 张英雄卡各在自己的 `skills` 中引用 1 个原英雄技能（新 id），且该技能不再被其他卡引用——这是数据事实，不再由校验器强制。
3. 校验器不再有「英雄技能」成对规则；`hero-` 相关常量与分支已删除。
4. `python3 scripts/validate_docs.py` 退出码 0，末行为第 6 节的输出。
5. `python3 -m unittest discover -s tests -v` 全部通过。
6. 文档内不再有「`hero-*` 技能」「英雄技能」作为类别名的表述。

## 8. 非目标

- 不调整技能文本、`effects`、`cost`、`uses_per_turn`。
- 不调整卡牌数值、星级、时代、流派、出处。
- 不重命名 `UseHeroPower` 与 `HERO_POWER_LIMIT`。
- 不新增技能标记字段。
