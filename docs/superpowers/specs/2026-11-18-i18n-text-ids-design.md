# 国际化：文本 id 与中文文本表

> 状态：待确认
> 范围：客户端文案、卡牌 / 技能 / Token 展示文本、规则标识英文化、协议视图取值、文档与校验
> 依据：[客户端技术基线](../../versions/v0.1/technical/architecture/client-stack.md)、[卡牌规范与数据](../../versions/v0.1/product/content/card-system.md)、[关键词与机制](../../versions/v0.1/product/rules/keywords-and-mechanics.md)

本文只描述国际化改造的目标、边界、机制与迁移方式；卡牌数值、技能效果与产品规则不在此复制，按链接引用。

## 1. 意图与为什么

当前程序里有两类中文：

1. **文案**：客户端界面文案、卡名、技能名与效果文本、错误提示。它们直接写在 `.gd` 源码或生成数据里，新增语言必须改代码。
2. **标识**：`card_type=随从/效果`、`tribe=人类/机械/…`、`era=史前/…`、关键词、技能 DSL 的过滤值、进化选项。它们既参与规则判定，又直接显示给玩家。

本次改造要把两类中文分开处理：**文案全部改为文本 id，标识全部改为英文 id**。运行时只有 id；所有中文集中在 `client/text/zh_CN.csv` 一个语言文件里。新增语言只需增加同结构文本文件，不改代码、不改规则、不改协议取值。

首版只支持中文，因此文本表只有 `zh_CN` 一份；英文缺失时由 Godot 回退到中文，而不是显示 key。

## 2. 目标、非目标、反目标

### 2.1 目标

1. 客户端运行时代码中不存在面向玩家的中文字符串字面量；
2. 卡牌、技能、Token 的展示文本不再进入生成数据，运行时按实体 id 从文本表取；
3. 规则标识、协议视图取值、文档 front matter 与技能 DSL 全部使用英文 id；
4. 文本表一个语言一个文件，中文集中在 `client/text/zh_CN.csv`；
5. 漏翻、漏 id、文本表漂移都能在 `make` 校验中被拦下；
6. 新增语言只增加文本文件与一行工程配置，不触碰业务代码。

### 2.2 非目标

- 本次不制作英文或其他语言文本，不引入翻译平台与翻译流程；
- 不翻译服务端日志、CLI 输出、测试失败信息、代码注释与产品文档正文；
- 不改变任何卡牌数值、技能效果、结算顺序与产品规则；
- 不引入运行时语言切换入口；只保留 Godot 原生的 locale 机制；
- 不把产品文档正文改写成英文；文档属性表继续用中文，便于阅读与校对。

### 2.3 反目标

- 客户端保留中文兜底文案；
- 生成数据里同时存在 id 与展示文本两份；
- 用中文作为协议取值或规则标识，再在客户端做中文到 id 的映射；
- 为单张卡在代码里写文案特判；
- 文本表与代码各自维护一份 id 清单。

## 3. 约束与已验证事实

### 3.1 既有约束

- 客户端 Godot 4.7 + GDScript，界面全部由代码构建，没有场景内静态文案；
- 服务端 Rust，规则引擎消费 `server/data/v0.1/*.json`，数据由产品文档生成；
- 客户端只消费服务端视图，不做规则判定；
- 卡牌 / 技能 / Token 的源头是 `docs/versions/v0.1/product/content/` 下的实体文档；
- 构建、生成与校验统一从 `make` 入口执行。

### 3.2 Godot 原生翻译的验证结论

已用 Godot 4.7.2 最小工程验证：

| 结论 | 结果 |
| --- | --- |
| CSV 导入 | `client/text/zh_CN.csv` 由 Godot 导入为同目录的 `zh_CN.zh_CN.translation` |
| 工程配置 | `project.godot` 的 `internationalization/locale/translations` 必须指向 `.translation` 资源，不能指向 `.csv` |
| 缺失回退 | `locale/fallback="zh_CN"` 时，系统 locale 为 `zh_Hans_CN` 或 `en` 都能取到中文 |
| CSV 转义 | 逗号、双引号、`\n` 换行转义、`%%` 百分号均正常 |
| 缺键行为 | `TranslationServer.translate(id)` 返回 id 本身，不报错 |
| 格式化 | `TranslationServer.translate("fmt") % 3` 正常 |

因此本次采用 Godot 原生 CSV → `.translation` → `TranslationServer` 管线，不自建文本字典。

## 4. 方案选择

| 方案 | 做法 | 取舍 |
| --- | --- | --- |
| A：JSON + 自建 `I18n` | 扁平 JSON，自建 autoload 查表 | 无导入步骤，但语言切换、复数与编辑器集成都要自己补 |
| **B：Godot 原生翻译（选用）** | CSV 导入为 `.translation`，代码用 `tr()` / `TranslationServer.translate()` | 原生管线、工程配置即可切换语言；代价是多一道导入步骤，缺键静默返回 id，需要测试兜底 |
| C：文本表 + 常量代码生成 | 文本表人工维护，生成 `Text` 常量 | 类型安全更好，但多一份生成物，动态内容 id 仍要拼字符串 |

选 B。缺键风险由第 10 节的静态校验覆盖；导入步骤由 `make` 目标统一处理。

## 5. 术语与文本 id

### 5.1 两类中文

- **标识**：机器使用的稳定取值，必须英文，例如 `minion`、`human`、`battlecry`。它出现在数据、协议、DSL 与代码比较中。
- **文案**：给玩家看的自然语言，只出现在文本表里，例如「随从」「人类」「战吼」。

### 5.2 id 命名

| 类别 | 形式 | 示例 |
| --- | --- | --- |
| 界面文案 | `ui.<页面>.<元素>` | `ui.login.title`、`ui.lobby.create_room` |
| 对局子视图 | `ui.game.<子视图>.<元素>` | `ui.game.side_panel.reroll`、`ui.game.combat.result_win` |
| 通用文案 | `ui.common.<元素>` | `ui.common.cancel` |
| 错误提示 | `error.<错误码>` | `error.ROOM_NOT_FOUND` |
| 枚举展示名 | `enum.<组>.<id>` | `enum.tribe.human`、`enum.era.industrial` |
| 卡牌 | `card.<卡牌 id>.<字段>` | `card.adam-smith.name`、`card.<id>.match_text` |
| 技能 | `skill.<技能 id>.<字段>` | `skill.division-of-labor.name`、`skill.<id>.text`、`skill.<id>.match_text` |
| Token | `token.<token id>.name` | `token.seedling.name` |

规则：

- 实体 id 已经唯一，直接作为文本 id 的命名空间，不另造短名；
- 文本 id 一旦发布不再改名；改文案只改值；
- 值为空表示没有文案，不写空字符串；
- 需要格式化的文案把占位符写进值，例如 `第 %d 名`，调用侧用 `%` 传入参数；
- 只进入日志、断言、注释的字符串不建文本 id。

### 5.3 文件与所有权

唯一运行时文本文件：`client/text/zh_CN.csv`。

```csv
keys,zh_CN
card.adam-smith.name,亚当·斯密
enum.tribe.human,人类
error.ROOM_NOT_FOUND,房间不存在
skill.division-of-labor.text,使一个友方人类随从获得一次成长；若你控制至少 3 个人类，改为两次
ui.login.title,文明：七时代演进
```

所有权按 key 前缀划分：

| 前缀 | 所有权 | 来源 |
| --- | --- | --- |
| `card.`、`skill.`、`token.` | 生成 | 产品文档的 name / text / match_text |
| `enum.card_type.`、`enum.tribe.`、`enum.era.`、`enum.keyword.`、`enum.subtype.` | 生成 | 产品文档属性表与关键词判定表 |
| `enum.evolve.`、`enum.stat_layer.` | 人工 | 进化选项、攻防分层名 |
| `ui.`、`error.` | 人工 | 客户端代码 |

生成器规则：

1. 读取现有 CSV，丢掉所有生成前缀的 key；
2. 从产品文档生成新的内容 key 与枚举 key；
3. 保留全部人工 key；
4. 人工 key 必须落在 `ui.`、`error.`、`enum.evolve.`、`enum.stat_layer.` 前缀内，其余前缀报错；生成前缀内的 key 一律按生成物处理，会被覆盖或删除；
5. 按 key 排序、LF 换行、UTF-8 无 BOM 写出；
6. 值中的反斜杠与换行按 Godot CSV 规则转义，换行写成 `\n`。

`zh_CN.csv` 与其导入产物 `zh_CN.zh_CN.translation` 都提交吗？

- `.csv` 提交；
- `.translation` 与 `.csv.import` 是 Godot 生成物，不提交，加入 `client/.gitignore`；
- 所有 Godot 入口先执行 `make client-import`，见第 11 节。

## 6. 运行时机制

### 6.1 工程配置

`client/project.godot` 增加：

```ini
[internationalization]

locale/fallback="zh_CN"
locale/translations=PackedStringArray("res://text/zh_CN.zh_CN.translation")
```

当前只有中文，系统 locale 不是中文时由 fallback 回退到中文。将来增加英文时，新增 `en.csv` 与对应 `.translation`，并把英文资源加入 `locale/translations`；fallback 仍保留中文作为缺翻兜底。

### 6.2 取文本 API

- 实例方法与 autoload 内：`tr(id)`；
- 静态函数内：`TranslationServer.translate(id)`（GDScript 静态函数不能调用 `tr()`）；
- 格式化：`tr("ui.result.place") % rank`；
- 缺键：Godot 返回 id。业务封装在返回自检时回退到实体 id 或错误码，不显示带点的内部 id。

不新增自建 `I18n` autoload，也不维护第二份字典。

### 6.3 封装点

| 入口 | 行为 |
| --- | --- |
| `CardDB.display_name(id)` | 先判断是否 Token：是则取 `token.<id>.name`，否则取 `card.<id>.name`；缺键返回实体 id |
| `CardDB.display_text(id)` | 按 `skills` 顺序取 `skill.<id>.text`，用换行拼接；空文本跳过 |
| `CardDB.era_name(era:int)` | 1–7 映射到时代 id，再取 `enum.era.<id>` |
| `UI.enum_text(group, id)` | 取 `enum.<group>.<id>`；缺键返回 id |
| `UI.error_text(code)` | 取 `error.<code>`；缺键返回 code |
| `UI.error_text_by_id(id, params)` | 取 `id` 并按 `params` 做 `%s` 格式化；缺键返回空串，调用方回退 `UI.error_text(code)` |

其余页面直接调用 `tr()` / `TranslationServer.translate()`，不经过中间层。

## 7. 标识英文化映射

### 7.1 固定映射

| 字段 | 中文 | 英文 id |
| --- | --- | --- |
| `card_type` | 随从 / 效果 | `minion` / `effect` |
| `tribe` | 人类 / 机械 / 生物 / 植物 / 病毒 / 学者 / 通用 | `human` / `machine` / `biology` / `plant` / `virus` / `scholar` / `general` |
| `era` | 史前 / 原始 / 古典 / 封建 / 工业 / 现代 / 未来 | `prehistoric` / `primitive` / `classical` / `feudal` / `industrial` / `modern` / `future` |
| 进化选项 | +2/+2 / 护盾 / 复生 | `stats` / `divine_shield` / `reborn` |
| 攻防分层 | 初始 / 永久 / 回合 / 光环 / 受击 | `initial` / `permanent` / `turn` / `aura` / `damage` |

### 7.2 关键词

以 [关键词与机制](../../versions/v0.1/product/rules/keywords-and-mechanics.md) 的英文名为准，空格转下划线：

| 中文 | 英文名 | id |
| --- | --- | --- |
| 成长 | Growth | `growth`（效果原语，不登记为技能 `keyword`） |
| 繁衍 N | Breed N | `breed` |
| 战吼 | Battlecry | `battlecry` |
| 亡语 | Deathrattle | `deathrattle` |
| 复生 | Reborn | `reborn` |
| 嘲讽 | Taunt | `taunt` |
| 护盾 | Divine Shield | `divine_shield` |
| 连击 | Windfury | `windfury` |
| 传染 | Contagion | `contagion` |
| 最优攻击 | Optimal Attack | `optimal_attack` |
| 装配 N | Assemble N | `assemble` |
| 播种 | Sow | `sow` |
| 进化 | Evolve | `evolve` |
| 禁攻（内部标记） | Cannot Attack | `cannot_attack` |

带层数的关键词继续用「基名 + 空格 + 数字」表示：`breed 2`、`assemble 1`。规则引擎的 `strip_prefix("繁衍")` 改为 `strip_prefix("breed")`。

### 7.3 技能 DSL

| 位置 | 现在 | 改为 |
| --- | --- | --- |
| 过滤键 | `name=弓箭手` | `id=gongjianshou` |
| 过滤值 | `tribe=人类` | `tribe=human` |
| 过滤值 | `type=随从` | `type=minion` |
| 条件 | `timesIf=人类:2` | `timesIf=human:2` |
| 计数 | `condition=count:friendly:tribe=人类:>=3` | `condition=count:friendly:tribe=human:>=3` |
| 关键词 | `keyword=传染`、`keyword=繁衍 2` | `keyword=contagion`、`keyword=breed 2` |

`name` 过滤键改名为 `id`，取值是卡牌 id 而不是卡名。当前全仓只有一处 `name=弓箭手`，迁移为 `id=gongjianshou`；校验脚本新增「`id` 过滤值必须是已存在的卡牌 id」。

### 7.4 子类

`subtype` 从中文自由文本改为英文 slug（小写、`-` 分隔、匹配 `^[a-z][a-z0-9-]*$`），中文名进 `enum.subtype.<slug>`。86 个现有子类的 slug 在迁移时一次性确定；此后新增子类同时给出 slug 与属性表中文名，由校验脚本保证同一 slug 只有一个中文名。

## 8. 数据与文档迁移

### 8.1 产品文档 front matter

- 卡牌：`card_type`、`tribe`、`era`、`subtype` 改英文 id；`name`、`match_text` 保持中文（作为文本表内容源）。
- 技能：`keyword` 改英文 id（可带层数）；`effects` 的过滤值与关键词改英文 id；`name`、`text`、`match_text` 保持中文。
- Token 注册表：`名称` 保持中文，`关键词` 列若填写则用英文 id。

**正文属性表不改**：`| 类型 | 随从 |`、`| 流派 | 人类 |`、`| 时代 | 工业 |`、`| 子类 | 经济学 |` 继续用中文。校验脚本按 front matter 的 id 找到对应中文名后与属性表比对，而不是要求属性表写英文。

### 8.2 生成数据

`server/data/v0.1/` 与 `client/generated/` 不再包含展示文本：

| 文件 | 保留字段 | 删除字段 |
| --- | --- | --- |
| `cards.json` | `id`、`card_type`、`tribe`、`subtype`、`level`、`era`、`attack`、`health`、`skills`、`is_scholar`、`is_hero`、`cost`（客户端另有 `display_cost`） | `name`、`match_text` |
| `skills.json` | `id`、`keyword`、`effects`、`cost`、`uses_per_turn` | `name`、`text`、`match_text` |
| `tokens.json` | `id`、`attack`、`health`、`keywords`（客户端另有 `display_cost`） | `name` |

`match_text` 当前没有运行时消费方，但它是产品字段，保留在文档并生成文本 key（`card.<id>.match_text` / `skill.<id>.match_text`），供后续版本使用。

### 8.3 服务端数据结构

- `Card` 删除 `name`、`match_text`；
- `Skill` 删除 `name`、`text`、`match_text`；
- `Token` 删除 `name`；
- `CardType`、`Tribe`、`Era` 的 serde `rename` 改英文 id；
- 关键词常量、进化选项、过滤键与过滤值改英文 id；
- `view.rs` 的 `enabledTribes` 与 `keywords` 下发英文 id；
- 服务端日志、CLI 输出与错误 `message` 保持中文，`message` 继续只面向日志。

### 8.4 协议

- `MinionView.keywords` 的值从中文改为英文 id；
- `PendingChoice.options` 的进化选项从中文改为英文 id；
- `enabledFactions` 的取值从中文改为英文 id；
- `ErrorMessage` 新增：
  - `messageId`：必填字符串，面向玩家的文本 id，默认 `error.<CODE>`；
  - `messageParams`：可选字符串数组，默认空数组，供 `%s` 格式化；
- `ErrorMessage.message` 保持中文，仅用于日志，客户端不再展示；
- 除错误消息新增字段外，字段名、类型与结构不变；
- `protocol/schema/v1/` 的 description 更新，说明取值为英文 id；不新增枚举约束。

服务端错误构造：

- `error_message` / `error_update` 默认写入 `messageId = "error.<CODE>"`，保证每个错误码至少有一个可展示的 id；
- 需要更具体说明的调用点传专项 id，例如 `AUTH_FAILED` 的验证码错误写 `error.AUTH_FAILED.invalid_code`，`TOKEN_EXPIRED` 写 `error.TOKEN_EXPIRED.invalid_token`；
- 其它规则拒绝继续使用错误码默认 id，中文 `message` 只进日志。

## 9. 客户端迁移

### 9.1 文案

逐文件替换：

| 文件 | 内容 |
| --- | --- |
| `client/scenes/login.gd` | 标题、字段名、默认值、状态文案、按钮 |
| `client/scenes/lobby.gd` | 标题、创建 / 加入、房间码、状态 |
| `client/scenes/room.gd` | 选英雄、确认状态、等待文案 |
| `client/scenes/game.gd` | 手牌、状态、各类待选浮层、取消 / 跳过 |
| `client/scenes/game_side_panel.gd` | 商店操作、英雄技能、回合、身份、状态 |
| `client/scenes/game_detail_layer.gd` | 属性行名、状态值、售货员说明 |
| `client/scenes/game_players_rail.gd` | 战报、名次、幽灵、回合、伤害 |
| `client/scenes/game_combat_view.gd` | 阵营标题、等待、胜负、幽灵、战斗条 |
| `client/scenes/game_recruit_view.gd` | 出售提示、空位 / 空槽 |
| `client/scenes/game_card_face.gd` | 冻结标记、类型 / 部落、时代 |
| `client/scenes/result.gd` | 结算标题、名次、种子、返回 |
| `client/ui/ui.gd` | 错误码文案表改为 `error.*` 取文本 |
| `client/ui/theme.gd` | `TRIBE_COLOR` 的键改部落 id |

纯符号（`—`、`+%d`、`★`、`·`）与内部标识（`minion` / `effect` 拖拽类型）不进入文本表。

错误提示：状态栏继续用 `UI.error_text(code)`；tooltip 改用 `UI.error_text_by_id(messageId, messageParams)`，不再展示服务端 `message`。`messageId` 缺失或取不到文本时回退到 `UI.error_text(code)`。涉及页面：`login.gd`、`lobby.gd`、`room.gd`、`game.gd`。

### 9.2 数据展示

- `CardDB` 按第 6.3 节改为文本表取文本；
- `GameCardFace.type_text` / `tribe_text` 通过 `UI.enum_text` 翻译；
- `game_detail_layer` 的时代、类型、状态值通过 `enum.*` 翻译；
- `UITheme.tribe_color` 使用部落 id；
- 进化选项通过 `UI.enum_text("evolve", option)` 翻译。

### 9.3 测试

- `client/tests/fixture.gd` 的 `enabledFactions`、`keywords` 改英文 id；
- 冒烟测试对界面文案的断言改为对 `TranslationServer.translate(id)` 的比较，不再硬编码中文；
- 测试失败信息与注释可以保留中文，它们不进入运行时界面。

## 10. 校验

新增 `tests/test_i18n.py`（Python），覆盖：

1. `client/text/zh_CN.csv` 表头为 `keys,zh_CN`，key 唯一、非空，值非空；
2. 生成前缀的 key 与 `make data` 结果零差异；人工 key 不被生成器丢弃；
3. 客户端运行时代码中所有 `tr("...")` / `TranslationServer.translate("...")` 字面量 key 都存在于文本表；服务端源码中出现的 `error.*` 文本 id 也存在于文本表；
4. 所有卡牌、技能、Token 所需的 key 都存在；所有固定枚举取值都有 `enum.*` key；`protocol.gd` 中每个 `ERROR_*` 错误码都有 `error.<code>` key；
5. 客户端运行时代码（排除 `client/tests/`、`client/generated/`）的字符串字面量不含 CJK；注释、注释掉的代码不算；
6. `client/generated/*.json` 与 `server/data/v0.1/*.json` 的值不含 CJK；
7. `.csv` 中不存在未转义的字面换行；
8. 文本表没有以生成前缀出现的人工 key。

文档侧新增或调整校验：

- `scripts/validate_docs.py` 使用英文 id 集合；
- front matter id 与属性表中文名的映射由文档自身建立：同一 id 必须只有一个中文名；
- 关键词 id 与 [关键词与机制](../../versions/v0.1/product/rules/keywords-and-mechanics.md) 的英文名一致；
- `subtype` slug 合法且同一 slug 只有一个中文名；
- `id=` 过滤值必须命中已存在的卡牌 id；
- 文档统计按英文 id 计数，期望值不变。

`make` 入口调整：

- `data` / `data-check` 增加 `scripts/gen_text.py`；
- `client-smoke`、`m1b-e2e`、`dev-bots` 增加 `client-import` 前置（导入会产生 `.translation`）；
- `client-check` 增加 `tests/test_i18n.py`。

## 11. 实施顺序

1. **基础设施**：术语常量、`gen_text.py`、CSV 骨架与人工 key、`project.godot`、`.gitignore`、Makefile、`test_i18n.py` 骨架；
2. **文档与数据标识**：用一次性迁移脚本改写 239 份卡牌与 130+ 份技能文档的 front matter 与 DSL；更新校验脚本；重新生成服务端与客户端数据。这一步只要求 `make data-check` 与文档校验通过；`make m1-check` 要等到下一步服务端标识改造完成后才会通过，客户端在全部步骤完成前暂不可用；
3. **服务端规则与协议**：英文枚举、关键词、过滤键、进化选项、视图取值；`ErrorMessage` 新增 `messageId` / `messageParams`；更新 Rust 测试与协议样例；
4. **客户端**：按第 9 节替换文案与展示取值；更新冒烟测试与夹具；
5. **文档回写**：产品字段表、协议说明、客户端技术文档与 UI 文档；
6. **全量验证**：`make data-check`、`make m1-check`、`make client-check`、`make client-smoke`、`make m1b-e2e`。

第 2 步是一次性机械迁移，迁移脚本放在 `output/`（已忽略），不进仓库；迁移结果由校验脚本与 `git diff` 审查。

## 12. 风险与回滚

| 风险 | 处理 |
| --- | --- |
| 239 份文档机械替换出错 | 迁移后先跑 `make data-check` 与文档校验；属性表保持中文，diff 可逐行审查 |
| Godot 未导入导致运行时缺翻译 | 所有 Godot 入口加 `client-import`；CI 从 Makefile 入口执行 |
| 缺键显示内部 id | `test_i18n.py` 静态对账；封装点缺键回退实体 id / 错误码 |
| 文本表与文档漂移 | `make data-check` 比较生成结果；生成器保留人工 key 并拒绝前缀冲突 |
| 协议取值变更影响旧客户端 | v0.1 未发布，不做兼容层；客户端与服务端同分支升级 |
| 关键词带层数的解析回归 | 保留「基名 + 空格 + 数字」格式，Rust 与 Python 两侧测试覆盖 |

回滚边界：本次为单分支一次性切换，回滚即回到 `develop` 当前提交，不涉及线上数据迁移。

## 13. 留给实施者的判断

以下不改变产品规则，由实施者按现有代码风格决定：

- `gen_text.py` 与 `gen_card_data.py` 之间共享的文档解析函数放在哪个模块；
- `i18n_terms.py` 的常量组织方式；
- 一次性迁移脚本的写法与执行顺序；
- `UI.enum_text` 的签名与缺键日志是否 `push_warning`；
- 86 个子类 slug 的具体英文选词（同一 slug 的中文名必须唯一）。

## 14. 验收

```bash
make data-check
make m1-check
make client-check
make client-smoke
make m1b-e2e
```

验收标准：

1. 上述命令全部通过；
2. `client/text/zh_CN.csv` 是仓库中唯一包含玩家可见中文的运行时文件；
3. 客户端、服务端生成数据、协议取值、产品文档 front matter 与技能 DSL 中不再有中文标识；
4. 所有界面文案与内容文本都能从文本表取到；
5. 新增一个语言文件与工程配置即可切换语言，业务代码无需改动。
