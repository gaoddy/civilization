# 销量预测外部数据与获取方式

> 日期：2026-11-25
> 配套文档：[销量预测](sales-forecast.md)
> 用途：记录预测引用的全部外部数据、原始值、获取方法与已知偏差，保证结论可复跑、可核对

---

## 1. 数据清单

| 编号 | 数据 | 来源 | 类型 | 可信度 |
| --- | --- | --- | --- | --- |
| D1 | 同类作品销量 / 评价 / 价格 | SteamSpy API | 一手（抓取） | 中：低销量区间桶粗糙 |
| D2 | Auto Battler 标签作品分布 | SteamSpy API | 一手（抓取） | 中：按拥有量排序截断 |
| D3 | 平台与语言占比 | Steam 官方硬件调查 | 一手（抓取） | 高：月度波动大 |
| D4 | Steam 整体收入分布 | Gamalytic / GameDiscoverCo / VG Insights | **二手（搜索转述）** | **低：未核对原文** |
| D5 | 项目实现状态 | 本仓库代码 | 一手（核对） | 高 |
| D6 | 炉石传说注册 / 活跃 / 收入 | Blizzard 公告、SuperData / KeyBanc 转述、ActivePlayer.io | 混合：官方一手 + 第三方估计 | 官方高；第三方低 |

---

## 2. 获取方式（可复跑）

以下命令在任意目录执行即可，不依赖仓库。

### 2.1 SteamSpy 标签分布（D2）

```bash
curl -s 'https://steamspy.com/api.php?request=tag&tag=Auto+Battler'
```

返回 JSON 对象，键为 appid，值为 `name / owners / positive / negative / price / ccu / average_forever`。

**注意**：该接口按拥有量排序、条数有上限，底部被截断，因此"低拥有量占比"只会被低估。

### 2.2 SteamSpy 单款详情（D1）

```bash
curl -s 'https://steamspy.com/api.php?request=appdetails&appid=2427700'
```

字段口径：

| 字段 | 含义 |
| --- | --- |
| `owners` | 拥有量区间字符串，如 `"500,000 .. 1,000,000"` |
| `positive` / `negative` | 好评 / 差评条数 |
| `price` | **美分**，`1199` 即 $11.99 |
| `ccu` | 抓取时刻的并发人数 |
| `average_forever` | 历史平均在线 |

**注意**：`owners` 在 2 万以下是粗桶（`"0 .. 20,000"`），且部分第三方分发的作品（如 The Bazaar、Super Auto Pets）数据陈旧，需交叉核对。

### 2.3 确定 appid

```bash
curl -s 'https://store.steampowered.com/api/storesearch/?term=Backpack%20Battles&cc=us&l=english'
```

返回匹配项的 `id` 与 `name`。**不要凭记忆写 appid**——本次核对时 `1380910` 实际是 Stardeus，不是 Backpack Battles。

### 2.4 当前在线人数（交叉验证）

```bash
curl -s 'https://api.steampowered.com/ISteamUserStats/GetNumberOfCurrentPlayers/v1/?appid=2427700'
```

### 2.5 Steam 硬件与软件调查（D3）

```bash
curl -s 'https://store.steampowered.com/hwsurvey/' | grep -iE 'macos|windows|linux|schinese|english'
```

页面为服务端渲染的统计表；`store.steampowered.com/hwsurvey` 也可直接浏览。

### 2.6 炉石传说与酒馆战棋（D6）

先确认是否在 Steam 上：

```bash
curl -s 'https://store.steampowered.com/api/storesearch/?term=Hearthstone&cc=us&l=english'
```

返回 `{"total":0,"items":[]}`——**炉石传说没有 Steam 版**，因此没有 SteamSpy 拥有量口径，也不能用 `GetNumberOfCurrentPlayers` 估算。核对日期 2026-11-25。

第三方月活：

- https://activeplayer.io/hearthstone/ （第三方模型，绝对值低可信，只取趋势）

官方与二手数字为 Wikipedia 汇总的 Blizzard 公告与媒体估计，见 §4.1。

---

## 3. 原始数据：Auto Battler 标签（D2）

抓取时间 2026-11-25，共 47 条。`price` 单位为美分，`0` 表示免费。

| # | 名称 | 拥有量 | 好评 | 差评 | 价格 | CCU |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Dota Underlords | 10M–20M | 74,801 | 16,167 | 0 | 991 |
| 2 | Minion Masters | 5M–10M | 58,410 | 6,942 | 0 | 281 |
| 3 | Vampire Survivors | 5M–10M | 249,158 | 4,174 | 499 | 2,668 |
| 4 | MS Flight Simulator (2020) 40th Anniv. | 1M–2M | 55,767 | 14,881 | 2,099 | 2,991 |
| 5 | Legion TD 2 | 1M–2M | 11,021 | 1,700 | 2,499 | 659 |
| 6 | Thronefall | 1M–2M | 22,504 | 927 | 649 | 1,096 |
| 7 | 9 Kings | 1M–2M | 21,316 | 1,776 | 999 | 3,450 |
| 8 | Backpack Battles | 500K–1M | 17,454 | 1,713 | 1,199 | 1,633 |
| 9 | Bad North: Jotunn Edition | 500K–1M | 11,786 | 786 | 299 | 131 |
| 10 | DPS: Desktop Pension Squad | 200K–500K | 1,359 | 439 | 0 | 2,924 |
| 11 | Idle Legions | 50K–100K | 109 | 267 | 0 | 589 |
| 12 | Swarm Queen | 20K–50K | 365 | 22 | 749 | 1 |
| 13 | Wooden Battles | 20K–50K | 411 | 106 | 999 | 1 |
| 14 | Against Heroes | 0–20K | 13 | 1 | 399 | 1 |
| 15 | Kur - Endless Descent | 0–20K | 28 | 18 | 0 | 53 |
| 16 | Mimic2 - Idle MMORPG | 0–20K | 26 | 23 | 0 | 59 |
| 17 | Arche PG | 0–20K | 15 | 16 | 0 | 14 |
| 18 | Daydream Cat: DreamSpirits Idle Adventure | 0–20K | 77 | 21 | 699 | 332 |
| 19 | Boss After Boss | 0–20K | 55 | 15 | 499 | 23 |
| 20 | BugPot | 0–20K | 4 | 0 | 399 | 0 |
| 21 | Kart League Manager | 0–20K | 26 | 0 | 900 | 6 |
| 22 | Habet | 0–20K | 10 | 8 | 1,999 | 1 |
| 23 | Fatespun | 0–20K | 2 | 0 | 999 | 0 |
| 24 | Moonbow Idle: Dark Fantasy Auto Battler | 0–20K | 15 | 2 | 179 | 30 |
| 25 | Tower Hero Survival | 0–20K | 3 | 0 | 399 | 0 |
| 26 | Everbastion | 0–20K | 1 | 0 | 499 | 1 |
| 27 | Shademarch | 0–20K | 4 | 0 | 699 | 0 |
| 28 | SIGNOS: AI Robot Builder | 0–20K | 3 | 0 | 499 | 0 |
| 29 | Gambapets | 0–20K | 0 | 0 | 999 | 0 |
| 30 | Prigod Idle | 0–20K | 8 | 3 | 799 | 7 |
| 31 | Tiny Ring Defense | 0–20K | 12 | 2 | 499 | 21 |
| 32 | Magua: Twenty Sorrows | 0–20K | 5 | 1 | 0 | 2 |
| 33 | Starfall Defense | 0–20K | 5 | 1 | 0 | 0 |
| 34 | Beastforger Autobattle | 0–20K | 4 | 0 | 299 | 2 |
| 35 | Midnight Rogues | 0–20K | 10 | 0 | 699 | 0 |
| 36 | Bannerborn | 0–20K | 4 | 1 | 799 | 1 |
| 37 | FableBrawl | 0–20K | 5 | 4 | 0 | 0 |
| 38 | I Only Moved My Mouse | 0–20K | 22 | 3 | 1,299 | 63 |
| 39 | Spooky Land Defenders | 0–20K | 3 | 0 | 499 | 0 |
| 40 | Mystic Realm | 0–20K | 1 | 0 | 699 | 1 |
| 41 | Only Axolotls | 0–20K | 1 | 1 | 499 | 1 |
| 42 | MMA Tactics - Martial Arts | 0–20K | 1 | 0 | 999 | 0 |
| 43 | Blooby Idle | 0–20K | 4 | 2 | 0 | 1 |
| 44 | Tabletop TD | 0–20K | 9 | 1 | 499 | 0 |
| 45 | Dungeoneers Idle | 0–20K | 9 | 1 | 849 | 8 |
| 46 | Before I Turn: Survival Auto Battler | 0–20K | 5 | 0 | 799 | 0 |
| 47 | Damage Numbers Go Brrr! | 0–20K | 8 | 2 | 299 | 0 |

**统计**：拥有量 ≥2 万 13 款（27.7%），0–2 万 34 款（72.3%）。

**偏差说明**：

- 标签含明显误标（Vampire Survivors、MS Flight Simulator、Bad North、Thronefall 不是严格意义的自走棋），剔除后头部会更薄。
- 接口按拥有量降序且条数受限，底部截断，真实低销量占比只会更高。
- `0–20K` 是粗桶，无法区分 50 份和 19,000 份。

---

## 4. 原始数据：重点作品详情（D1）

抓取时间 2026-11-25。

| appid | 名称 | owners | positive | negative | price（美分） | ccu | average_forever |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2427700 | Backpack Battles | 500,000–1,000,000 | 17,454 | 1,713 | 1,199 | 1,633 | 2,493 |
| 1948800 | Yi Xian: The Cultivation Card Game | 500,000–1,000,000 | 4,531 | 1,601 | 0 | 799 | 1,379 |
| 1617400 | The Bazaar | 0–20,000 ⚠ | 0 | 0 | 1,999 | 7,246 | 5,061 |
| 1714040 | Super Auto Pets | 0–20,000 ⚠ | 0 | 0 | null | 0 | 1,063 |
| 1380910 | Stardeus（用于验证 appid 解析） | 100,000–200,000 | 1,179 | 164 | 2,999 | 146 | — |

⚠ The Bazaar 与 Super Auto Pets 的 `owners` / `positive` / `negative` 明显陈旧（The Bazaar 的 CCU 为 7,246 却显示 0–2 万拥有量），**不作为销量依据**，仅用于确认品类存在需求。

**好评论换算**（用于 §3.2 表格）：

- Backpack Battles：17,454 / (17,454 + 1,713) = 91.1%
- Yi Xian：4,531 / (4,531 + 1,601) = 73.9%

### 4.1 炉石传说（D6）

核对日期 2026-11-25。炉石传说是免费游戏，**没有"销量"**，只有注册 / 活跃 / 收入口径。

| 指标 | 数值 | 时间 | 来源 | 类型 |
| --- | --- | --- | --- | --- |
| 累计注册 | 1 亿 | 2018-11 | Blizzard 公告 | 一手（经 Wikipedia 转述，未打开原文） |
| 活跃玩家 | 2,000 万+ | 2020 | Blizzard《Year of the Phoenix》回顾（2021-11） | 一手（经 Wikipedia 转述） |
| 月收入 | 约 $2,000 万 | 2015-06 | SuperData Research | 二手估计 |
| 年收入 | 约 $6 亿 | 2019-07 | KeyBanc Capital Markets | 二手估计 |
| 当前月活 | 3,406,146（近 30 天均值） | 2026-11 | ActivePlayer.io | 第三方模型 |

**已知空缺与偏差**：

- 酒馆战棋（Battlegrounds，2019-11 上线）的独立玩家数 **Blizzard 从未披露**，第三方拆分数不可信。
- 2023-10 微软完成收购后，不再按 IP 单独披露用户或收入。
- ActivePlayer 的 340 万月活与 Blizzard 2020 年 2,000 万+ 的口径相差近一个数量级，说明第三方模型不可用于绝对值；只取"2024 年起明显下台阶"的趋势。
- 2015-05 Activision Blizzard 称"炉石 + Destiny 合计近 $10 亿"，为合并口径，**不可拆分归因**，本文不使用。

---

## 5. 二级来源（未经原文核对）

以下数据来自网络搜索的**转述**，未能逐项打开原始报告核对，**可信度低**。前三行仅用于说明"Steam 整体收入分布极度头部化"这一趋势；炉石两行仅用于说明品类的收入量级，不进入销量计算。

| 数值 | 转述来源 |
| --- | --- |
| 中位游戏首年收入约 $1,000–2,000 | Gamalytic 博客、GameDiscoverCo |
| 58–70% 首年不到 $1,000 | Gamalytic、GameDiscoverCo、VG Insights |
| 2025 年 Steam 发行 18,000+ 款 | VG Insights |
| 炉石 2015 年月收入约 $2,000 万 | SuperData Research（经 Wikipedia 转述） |
| 炉石 2019 年收入约 $6 亿 | KeyBanc Capital Markets（经 Wikipedia 转述） |

参考入口（需自行核对原文）：

- https://gamalytic.com/blog/steam-revenue-2025
- https://gamediscover.co
- https://vginsights.com

**处理方式**：预测中只把该数据用于"分布极度头部化"这一定性判断，不用于任何精确计算。若需引用具体数值，应先核对原文并记录核对日期。

---

## 6. 原始数据：平台与语言（D3）

Steam 硬件与软件调查，2026-10：

| 维度 | 占比 | 环比 |
| --- | --- | --- |
| Windows | 93.70% | -1.33 pt |
| macOS | 2.35% | +0.43 pt |
| Linux | 3.95% | — |
| 简体中文 | 21.98% | **-9.41 pt** |
| 英文 | 39.04% | — |

**偏差说明**：

- 简体中文占比单月波动极大（本月较上月 -9.41 个百分点），任何单月读数都可能失真，做决策时建议取多月中位数。
- 调查为抽样，非全量；不同月份样本构成会变。
- **不可将平台占比与语言占比独立相乘**：中文玩家绝大多数在 Windows，Mac 玩家以英文环境为主，两者负相关。独立相乘会显著高估交集（预测文档 §2.1 已按此修正）。

---

## 7. 仓库核对项（D5）

用于修正预测中"现状"的事实基础，核对日期 2026-11-25。

| 结论 | 核对位置 |
| --- | --- |
| 432 张卡面、63 张立绘、44 个头像已存在 | `client/assets/cards/faces`、`client/assets/characters/` |
| 卡面统一风格基线已定义 | `docs/current/technical/card-cover-art.md` |
| 机器人补位已实现且默认开启 | `protocol/schema/v1/messages.schema.json`（`botFill`）、`server/crates/room-service/src/room.rs`（`bot_fill: true`）、`prepare_start` |
| 机器人只有一套策略、无难度分级 | `server/crates/sim/src/bot/brain.rs` |
| UI 已接入 `tr()`，但只有 zh_CN | `client/` 内 115 个 `.gd` 文件；`client/text/zh_CN.zh_CN.translation` |
| 无 Windows 导出预设 | `client/export_presets.cfg` |
| 设计总纲与代码在"AI 补位"上冲突 | `docs/current/product/overview/design-overview.md` §3、`docs/version-roadmap.md` §4 |
| 单人开发、11 天 1,199 次提交 | `git log` |

---

## 8. 复跑与更新约定

- 外部数据每季度重抓一次；抓取后更新本文的日期与数值，并在[销量预测](sales-forecast.md) §0 复算结论。
- 重抓时**必须**先按 §2.3 确认 appid，不沿用本文的 appid 而不验证。
- 新增来源时在 §1 登记类型与可信度；二级来源不得直接用于精确计算。
- 若 SteamSpy 接口不可用，替代顺序：SteamDB → Gamalytic → SteamSpy 镜像。
