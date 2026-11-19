# 对局页酒馆战棋结构改版：技术方案

> 状态：已实施
> 需求归属：v0.1
> 产品依据：[product.md](product.md)
> 设计依据：[对局页酒馆战棋结构改版 设计](../../../../superpowers/specs/2026-11-20-game-page-battlegrounds-layout-design.md)

## 1. 改动边界

- 只改客户端对局页及其页面内组件；服务端、协议、规则、卡牌数据、生成物全部不变。
- 不新增协议字段：对手英雄所需的名字、生命、名次、精灵与英雄均来自现有 `view.players[]`，存活判定与幽灵标记沿用 `GameStore.combat_is_current()`、`last_combat.mirror` 与 `view.opponentPlayerId`。
- 不改页面入口与返回：仍由房间页进入，对局结束进结算页，退出对局回大厅，错误去向沿用 `UI.error_route`。
- 不引入图片、字体或主题资源；本轮只固定槽位尺寸与节点名。
- 不新增动效与节奏参数；战斗演示的 `COMBAT_PACE_MS`、`COMBAT_TAIL_MS` 与 `CombatPlayback` 不变。

## 2. 分层落点

### 2.1 页面与页面内组件

对局页仍由 `client/scenes/game.tscn` + `game.gd` 承载，只做装配、意图与弹层；展示区块拆到页面目录下的独立文件，不在 `game.gd` 内联。

| 文件 | 归属 | 职责 | 来源 |
| --- | --- | --- | --- |
| `client/scenes/game.gd` | 页面 | 装配顶部工具区、排名栏与中央舞台；组装 `GameUiState`；发送意图；维护选择浮层、详情层、图鉴、战报与首局引导 | 改 |
| `client/scenes/game_top_bar.gd` | 页面组件 | 左侧状态文本，右侧图鉴 / 提示 / 退出对局；只发信号 | 新增 |
| `client/scenes/game_shop_view.gd` | 页面组件 | 商店操作行（升时代图标—售货员—刷新/冻结图标）、商店 6 槽、出售落点与出售提示；图标按钮为矢量绘制 | 新增，吸收 `GameSidePanel` 的售货员与操作按钮、`GameRecruitView` 的商店 |
| `client/scenes/game_river.gd` | 页面组件 | 河道行：回合数、倒计时翻转、结束招募、战斗结果横幅与「战斗中」；招募实例是页面唯一计时源 | 新增 |
| `client/scenes/game_hero_row.gd` | 页面组件 | 组合 64×84 `HeroCard`、金币、英雄技能图标与费用/次数角标；淘汰观战态；战斗阶段隐藏技能图标 | 新增，吸收 `GameSidePanel` 的英雄段 |
| `client/scenes/game_hero_card.gd` | 页面组件 | 64×84 英雄卡：封面、名字行、底部攻击/时代/生命；只读展示 | 新增 |
| `client/scenes/game_icon_button.gd` | 页面组件 | 40×40 图标按钮：右上费用角标、左上次数角标、激活/禁用态；点击只发信号 | 新增 |
| `client/ui/icons.gd` | 共享设计层 | 图标共享入口：升时代、刷新、冻结、英雄技能四个矢量图标的绘制路径与统一网格 | 新增 |
| `client/autoload/card_db.gd` | 数据展示 | 读取 `client/generated/skills.json`，暴露 `skill(id)` 与 `skill_cost(id)`；只用于展示技能费用 | 改 |
| `client/scenes/game_opponent_view.gd` | 页面组件 | 战斗阶段顶部对手英雄；幽灵与等待配对态 | 新增，吸收 `GameSidePanel` 的战斗身份块 |
| `client/scenes/game_hand_row.gd` | 页面组件 | 贴底手牌行：10 张单行、重叠、悬停放大、拖拽出牌；空手牌提示 | 新增，从 `game.gd` 抽出 |
| `client/scenes/game_portrait.gd` | 页面组件 | 64×64 无图头像槽；售货员、对手英雄共用；自己英雄不再使用 | 新增 |
| `client/scenes/game_recruit_view.gd` | 页面组件 | 招募组合：商店、河道、我方棋盘；只做装配与棋盘刷新 | 改 |
| `client/scenes/game_combat_view.gd` | 页面组件 | 战斗组合：对手英雄、对手随从行、河道、我方随从行、`CombatFx`；继续持有 `CombatPlayback` | 改 |
| `client/scenes/game_side_panel.gd` | — | 右栏整体删除，内容按上表归位 | 删除 |
| `client/scenes/game_players_rail.gd` | 页面组件 | 左排名栏，不改 | 不变 |

### 2.2 状态与数据

- 页面状态仍为 `GameUiState`（`client/scenes/game_ui_state.gd`）：在现有字段上加 `can_drag(playable) -> bool`，作为手牌拖拽唯一门控；`game.gd` 的静态 `drag_gate` 保留给棋盘与测试，语义不变。
- 计时状态只保留一份：招募河道持有截止时间字符串，暴露 `refresh(state)`、`has_deadline()`、`seconds_left()`；`game.gd` 每 0.25 秒向招募河道取值，保留「没有截止时间」与「已过期（秒数为 0）」的区分。
- 数据层不变：页面继续只读 `GameStore.view`、`GameStore.last_combat`、`GameStore.phase`、`GameStore.deadline`，不直接访问网络；所有意图经 `_send_intent`。
- 共享设计层加三处：`UITheme.SCENARIO` 的布局度量（含 `top-bar-height` 28、`river-height` 32、`hero-row-height` 84、`hero-card-width` 64、`hero-card-height` 84、`shop-icon-size` 40、`opponent-hp-width` 360），`UITheme.panel_style` 的零边距透明变体（`bare` / `accent-bare`），以及图标共享入口 `client/ui/icons.gd`。图标语义与网格统一在共享入口定义，页面组件只引用，不在页面内联路径。

## 3. 页面结构与几何

### 3.1 结构

```text
背景
└─ 外边距（左/右 16，上 12，下 8）
   └─ 纵向根容器（间距 8）
      ├─ TopBar（28）
      │  ├─ StatusLabel（左，自适应）
      │  └─ GalleryButton / HintButton / LeaveGameButton（右）
      └─ 主体（横向，间距 12）
         ├─ PlayersRail（88，不变）
         └─ CenterPanel（自适应，间距 8）
            ├─ RecruitPanel（招募可见）
            │  ├─ ShopView
            │  │  ├─ ShopActions（64：UpgradeButton 40×40 — SellerTile 64×64 — RerollButton/FreezeButton 40×40）
            │  │  └─ ShopPanel（SellDrop，居中 6 槽 + SellHint）
            │  ├─ 弹性留白
            │  ├─ RecruitRiver（32）
            │  └─ BoardPanel（居中 7 槽）
            ├─ CombatPanel（战斗可见）
            │  ├─ OpponentView（64）
            │  ├─ CombatOpponentRow（146）
            │  ├─ 弹性留白
            │  ├─ CombatRiver（32，含结果横幅）
            │  ├─ CombatOwnRow（146）
            │  └─ CombatFx（全区域覆盖，不参与布局）
            ├─ HeroRow（84，常驻）
            │  ├─ HeroCard（64×84：HeroCardCover / HeroCardName / HeroCardStats）
            │  └─ HeroSide（GoldTile / HeroPowerButton 40×40）
            └─ HandStrip（146，常驻，页面最底）
               └─ HandScroll → 居中容器 → HandRow
```

### 3.2 纵向预算

| 区块 | 高度 |
| --- | --- |
| 页面上边距 + TopBar + 根间距 | 12 + 28 + 8 = 48 |
| 中央舞台可用高度 | 720 − 48 − 8 = 664 |
| CenterPanel 内组合 + 间距 + HeroRow + 间距 + HandStrip | 418 + 8 + 84 + 8 + 146 = 664 |

- 招募固定占用：操作行 64 + 间距 6 + 商店 146 + 间距 18 + 河道 32 + 间距 6 + 棋盘 146 = 418，无额外弹性留白。
- 战斗固定占用：对手英雄 64 + 间距 6 + 对手行 146 + 间距 18 + 河道 32 + 间距 6 + 我方行 146 = 418，无额外弹性留白。
- 18 间距位于商店/对手行与河道之间；河道下方另留 6 与棋盘/我方行相接。两种组合的河道都在组合顶部下方 234–266，下方行从 272 开始。
- 英雄行 84；手牌 146，手牌卡底边 712，距页面底部 8。
- 硬阈值：任一组合的固定占用不得超过 418；手牌卡底边不得低于 712。实现后用冒烟断言量取实际矩形，超出时先压缩组合内部的 18 间距，不缩小 112×146 卡面，也不缩小 64×84 英雄卡。

### 3.3 横向

- 舞台宽 1148：商店 6 槽 712、棋盘 7 槽 832、手牌 10 张步进 80 共 832，均居中且单行。
- 商店面板与棋盘面板占满舞台宽，内部用 `CenterContainer` 居中槽位；面板本身使用零边距透明样式，出售高亮时只换描边与底色，不改变内容矩形。

## 4. 状态、刷新与阶段

| 场景 | 页面动作 |
| --- | --- |
| 视图为空 | TopBar 状态「等待对局状态…」；引导阶段置空 |
| 招募 | 显示 `RecruitPanel`；`refresh(state)` 重画商店、棋盘与招募河道；隐藏 `CombatPanel`；`HeroRow` 与 `HandStrip` 常驻 |
| 战斗/结算 | 显示 `CombatPanel`；`refresh(state)` 重画对手英雄、双方随从行与战斗河道；隐藏 `RecruitPanel`，但调用 `RecruitPanel.refresh_clock(state)` 让唯一计时源仍跟随当前阶段 |
| 回到招募 | `CombatView.reset()` 丢弃本场演示归属；`RecruitPanel` 全量刷新；`CombatPanel` 整块隐藏 |
| 断线/错误 | 只改 TopBar 左侧文本、颜色与 tooltip；按钮可用性按 `state.connected` |
| 倒计时归零 | 招募河道显示「等待结算」；`_deadline_expired` 经 `_refresh` 进入 `GameUiState.expired`，商店、棋盘、手牌与结束招募同时禁用 |

- `_set_combat_mode(in_combat)` 只切换两个组合的可见性与英雄技能图标；`HeroCard` 始终保留，战斗阶段技能图标隐藏。
- 战斗演示仍由 `CombatView.playing()/advance()/finished()` 驱动；战斗河道中央的 `CombatBanner` 在战斗阶段始终显示「第 N 回合」，播完后由 `CombatView` 在同一位置追加结果与颜色；不再单独建 `CombatTurnLabel`，`CombatPlayed` 回报逻辑不变。
- 手牌拖拽门控统一走 `GameUiState.can_drag(playable)`；页面落点回调继续用 `drop_gate` / `_can_play_now()` 复查，两者条件一致。

## 5. 数据映射

| 展示 | 来源 | 备注 |
| --- | --- | --- |
| 回合数 | `view.turn`，回退 `GameStore.turn` | 两条河道都显示 |
| 剩余秒数 | `GameStore.deadline`，回退 `view.phaseDeadline` | 招募河道；解析失败视为没有截止时间 |
| 结束招募可用性 | `self.affordances.canEndRecruit` + 页面 `stale/expired/target_pending` | 与改版前一致 |
| 售货员 | 静态文案与中性玩法色 | 不变 |
| 自己英雄卡 | `view.self.heroCardId` → `CardDB`（名字、`attack`）+ `view.self.era` + `view.self.hp` / `view.hpCap` | 名字单独一行；底部攻击左下、时代居中、生命「当前/上限」右下；卡牌缺失时攻击回退 0 |
| 英雄技能图标 | `view.self.heroPower` + `CardDB.skill_cost(skillId)` | 费用角标右上、剩余次数角标左上；费用取客户端 `generated/skills.json` 中该技能的 `cost`（协议不下发费用）；1 次隐藏数字、0 次置灰禁用；禁用原因沿用 |
| 对手英雄 | `view.players[]` 中 `view.opponentPlayerId` 对应项 + `view.hpCap` | 头像 64×64、名字、名次、HP、英雄；幽灵战读 `mirror` 并去血量条 |
| 淘汰观战 | `view.players[]` 中本人 `alive` | 只在 `HeroRow` 显示「已淘汰 · 观战中」 |
| 手牌 | `view.self.hand` + `affordances.handCards` | 卡面、角标、tooltip 与 payload 全部沿用 |

- 招募阶段不渲染 `OpponentView`，因此不存在「招募阶段显示对手」的分支。
- 战斗阶段 `OpponentView` 的判定顺序：`opponentPlayerId` 为空 → 等待配对；`mirror` 为真且本场战报有效 → 幽灵（角标、压暗、无 `HpBar`）；其余 → 普通对手。本人淘汰不再拦截对手展示，淘汰态只落在 `HeroRow`。
- 所有可用性仍只读服务端 affordance 与页面既有门控，不新增本地推导。

## 6. 既有机制冲突与处理

| 冲突点 | 影响 | 处理 |
| --- | --- | --- |
| 删除 `GameSidePanel` | `game.gd` 的计时、身份、英雄、商店操作与工具入口全部悬空 | 按第 2 节拆分；`game.gd` 只保留装配与信号连接，右栏文件删除 |
| 倒计时归属 | 原 `_side_panel.has_deadline()/seconds_left()` 是过期门控唯一来源 | 迁到招募河道；战斗阶段仍刷新该河道；`game.gd` 继续区分「无截止时间」与「归零」 |
| 身份判定链 | 原链「自身淘汰 → 幽灵 → 对手 → 等待配对」混在右栏身份块 | 拆成两处：招募售货员在商店操作行；战斗对手在顶部；本人淘汰只进 `HeroRow`；不再让淘汰拦截对手展示 |
| 手牌面板删除 | `HandPanel` 节点与「手牌」标题消失；首局引导步骤 2 目标失效 | 新节点 `HandStrip` + `HandRow`；引导步骤 2 改指 `HandRow` 与 `BoardRow` |
| 河道节点重名 | 招募/战斗各有一条河道，`EndButton`、`TimerLabel` 若同名会让 `find_child` 命中隐藏实例 | 招募河道用 `RecruitTurnLabel/RecruitTimerLabel/RecruitEndButton/RecruitCountdownFlip`；战斗河道用 `CombatBanner/CombatStatusPill`，`CombatBanner` 同时承担回合数与结果；引导步骤 3 改指招募节点 |
| 英雄卡高度 84 | 原英雄行 64，组合可用 438；新英雄行 84，组合可用 418 | 商店/对手行与河道之间的 18 间距保持固定网格，河道仍在 234–266；冒烟断言组合 418 与手牌底边 712，不缩小 112×146 卡面 |
| 图标替代文字按钮 | 现有按钮带中文文本，测试与可访问性按文本查找；改为图标后可见文本消失 | 新增 `GameIconButton`，根节点仍是 `Button`，保留中文 tooltip、禁用态与键盘焦点；测试改按节点名与 tooltip 断言 |
| 战斗面板外框与标题 | 原 `CombatPanel` 带 `surface` 面板与两段标题，占 48 高并破坏连续舞台 | 去掉标题；面板改用零边距透明样式，`CombatFx` 仍作为全区域覆盖子节点 |
| 棋盘与商店面板内边距 | 原 `surface` 面板 12 内边距会把 146 行撑到 170 | 新增 `bare` / `accent-bare` 零边距样式；出售高亮不改变布局 |
| 拖拽冒泡链 | 河道夹在棋盘与商店之间，若接收鼠标会截断拖放 | 河道外层与所有子节点 `MOUSE_FILTER_IGNORE`（倒计时翻转区除外），不作为落点；`BoardDrop`、`SellDrop` 的判定不变 |
| 手牌悬停覆盖英雄 | 手牌在英雄行下方，向上放大 1.12 会压住英雄行 | `HandStrip` 位于 `HeroRow` 之后（更高的绘制顺序），滚动容器 `clip_contents = false`；移出立即复位 |
| 状态行迁移 | 「未连接，操作未发出」与错误文案原在页面底部 | 迁到 `TopBar.StatusLabel`；`_send_intent`、`_on_error`、`_refresh_connection` 改写入 TopBar，文案、颜色与 tooltip 不变 |
| 战斗顶部恢复对手英雄 | 与 2026-11-17 HUD 改版「删除战斗英雄条」相反 | 产品 `product.md` 已列为本次增量，不作为缺陷回归 |
| 首局引导锚点 | `GoldTile`、`EndButton`、`TimerLabel`、`HandPanel` 位置或名称变化 | 步骤 1 仍为 `ShopPanel/ShopRow/GoldTile`；步骤 2 改 `HandRow/BoardRow`；步骤 3 改 `RecruitEndButton/RecruitTimerLabel`；步骤 4 仍为 `CenterPanel/ReportLayer` |
| 右栏测试语义 | `ui_smoke_side.gd` 断言「身份块与英雄块等高」「状态块居中」「售货员 ≥150」等已不成立 | 重写为舞台结构测试，不保留右栏语义断言 |

## 7. 导航

- 路由与跳转入口不变：仍集中在 `game.gd` 的 `_on_game_over`、`_on_leave_game` 与 `UI.error_route` 分支，不新增页面路径。
- 顶部工具区按钮只发信号，由页面处理，与右栏时期一致。

## 8. 性能

- 节点上限不变：8 座位、按 `boardSlots` 的棋盘点位、6 商店槽、10 手牌、双方战斗行；新增两条河道、一个 64×84 英雄卡、一个 40×40 图标按钮组件；售货员与对手英雄继续用 64×64 头像组件，数量固定。
- 战斗演示仍按 320ms 一步、按 `minionId` 原地更新；`CombatFx` 仍覆盖整块战斗组合，不参与布局。
- 计时：只有招募河道运行 `_process` 更新倒计时文案；战斗河道是静态胶囊。页面的 0.25 秒过期复核保持。
- 视图刷新仍整块重建页面内组件；手牌与商店的节点数量与改版前相同，不引入图片资源。图标用 `_draw()` 矢量绘制，图标按钮只有一层绘制，费用与次数角标是两个 `Label`，不新增贴图或额外绘制层。

## 9. 适配与可用性

- 逻辑分辨率固定 1280×720，按 `canvas_items` 缩放；不新增其他断点。
- 英雄卡 64×84、售货员/对手头像 64×64、图标 40×40 固定；攻击/时代/生命与费用/次数角标不改变槽位尺寸；后续美术版本只替换槽位内容。
- 图标按钮保留中文 tooltip、禁用态与键盘焦点；图标语义为升时代向上箭头、刷新环形箭头、冻结雪花、英雄技能星形；费用与次数角标同时写入 tooltip，保证无图标时也能理解。
- 结束招募沿用改版前的悬停翻转交互，不新增键盘路径。
- 河道只做展示与倒计时翻转，不拦截指针；拖拽经过河道不改变落点与光标状态。

## 10. 国际化

- 不新增文案 key；复用现有 `ui.game.side_panel.*`、`ui.game.recruit.*`、`ui.game.combat.*`、`ui.game.hand.*`、`ui.gallery.entry_match`、`ui.guide.hint`、`ui.room.exit_game`、`ui.game.status.*`。`side_panel` 前缀是历史命名，本轮不改 key，避免牵动翻译文件与测试。
- `ui.game.combat.banner` 的 key 不变，按产品文案把值改为「%s%s · 对 %s 造成 %d 点伤害」，由 `CombatBanner` 在前面拼 `ui.game.side_panel.turn`；不新增 key。文案源在 `client/text/zh_CN.csv`，随 `make data` 重建。
- 图标按钮的 tooltip 复用现有 `ui.game.side_panel.*` 文案；费用与次数为数字，不新增 key。
- 文案仍走 `tr()`，数字与回合数沿用现有格式；不新增日期、货币或 RTL 逻辑。

## 11. 测试设计

### 11.1 稳定节点名

| 区域 | 节点 |
| --- | --- |
| 顶部 | `TopBar`、`StatusLabel`、`GalleryButton`、`HintButton`、`LeaveGameButton` |
| 商店 | `ShopView`、`ShopActions`、`SellerTile`、`UpgradeButton`、`UpgradeCostBadge`、`RerollButton`、`RerollCostBadge`、`FreezeButton`、`ShopPanel`、`ShopRow`、`ShopSlotN`、`SellHint` |
| 招募河道 | `RecruitRiver`、`RecruitTurnLabel`、`RecruitCountdownFlip`、`RecruitTimerLabel`、`RecruitEndButton` |
| 棋盘 | `BoardPanel`、`BoardScroll`、`BoardRow`、`BoardMinionN`、`EmptySlot` |
| 战斗 | `CombatPanel`、`CombatBody`、`OpponentView`、`OpponentPortrait`、`OpponentIdentity` / `GhostIdentity`、`CombatOpponentRow`、`CombatRiver`、`CombatBanner`、`CombatStatusPill`、`CombatOwnRow`、`CombatFxLayer` |
| 英雄 | `HeroRow`、`HeroCard`、`HeroCardCover`、`HeroCardName`、`HeroCardStats`、`HeroAttack`、`HeroEra`、`HeroHp`、`GoldTile`、`HeroPowerButton`、`HeroPowerCostBadge`、`HeroPowerUsesBadge`、`EliminatedLabel` |
| 手牌 | `HandStrip`、`HandScroll`、`HandRow`、`HandSlotN`、`HandCardN` |

### 11.2 需要改写的测试

| 文件 | 处理 |
| --- | --- |
| `client/tests/ui_smoke_side.gd` | 重写为舞台结构测试：无 `SidePanel/SideIdentity/SideStatus/SideHero`；招募顺序为商店操作行—商店—河道—棋盘—英雄—手牌；售货员 64×64；英雄卡 64×84；英雄行 84；图标按钮 40×40；回合数在 `RecruitRiver` 中央；手牌贴底；倒计时翻转在 `RecruitCountdownFlip` |
| `client/tests/ui_smoke.gd` | 布局检查改节点：`SideHero` → `HeroCard`；`HandPanel` → `HandStrip`；`EndButton`/`TimerLabel` → `RecruitEndButton`/`RecruitTimerLabel`；新增商店操作图标与角标、英雄卡底部攻击/时代/生命、回合居中的断言；保留「商店在棋盘上方、河道在两者之间、手牌在英雄下方」的顺序断言与 712 底边断言 |
| `client/tests/ui_smoke_combat.gd` | 断言 `CombatBody` 子节点顺序为 `OpponentView`—对手行—`CombatRiver`—我方行；`CombatRiver` 恒为 `river-height`；`CombatBanner` 在播完前只显示「第 N 回合」，播完后追加结果（不再有 `CombatBannerSlot`）；不再断言 `CombatTurnLabel` |
| `client/tests/ui_smoke_guide.gd` | 步骤 2/3 的高亮目标改指 `HandRow`、`RecruitEndButton`、`RecruitTimerLabel`；`GoldTile` 仍在英雄行 |
| `client/tests/fixture.gd` | 不改数据；如新增舞台矩形辅助，只放在测试侧 |

### 11.3 新增断言

- 招募与战斗两个阶段都不存在 `SidePanel`、`SideIdentity`、`SideStatus`、`SideHero`。
- 页面顺序：`ShopPanel` 在 `BoardRow` 上方；`RecruitRiver` 位于两者之间；`HeroRow` 在 `BoardRow` 下方；`HandStrip` 在 `HeroRow` 下方。
- 售货员与对手头像均为 64×64；英雄卡 64×84；英雄行高度 84；图标按钮 40×40；手牌卡底边 ≤ 712。
- 英雄卡内：名字在时代上方单独一行；攻击在左下、生命在右下、时代在中间；生命文本为「当前/上限」。
- 商店操作：`UpgradeButton` 在 `SellerTile` 左侧，`RerollButton`/`FreezeButton` 在右侧；费用角标在图标右上；冻结无费用角标；冻结激活时 `FreezeButton` 高亮。
- 英雄技能：`HeroPowerCostBadge` 在右上、`HeroPowerUsesBadge` 在左上；1 次时次数角标隐藏；0 次时 `HeroPowerButton` 禁用并置灰；战斗阶段 `HeroPowerButton` 隐藏、`HeroCard` 保留。
- 回合数在 `RecruitRiver` 中央，倒计时在右端；悬停 `RecruitCountdownFlip` 翻成 `RecruitEndButton`，点击只发一次 `EndRecruit`；移出恢复。
- 战斗阶段 `CombatBanner` 始终显示回合数，播完后追加结果；右侧 `CombatStatusPill` 显示「战斗中」；`OpponentView` 显示对手名、HP 与名次；幽灵战有 `GhostBadge` 且无 `HpBar`；商店与商店操作不可见；手牌只读。
- 拖拽经过 `RecruitRiver` 不改变 `BoardDrop` / `SellDrop` 的落点判定；出售提示仍在 `SellHint`。
- 手牌悬停放大后仍完整可见，且绘制在英雄行之上；移出即恢复。

### 11.4 验证命令

| 命令 | 覆盖 |
| --- | --- |
| `make client-check` | GDScript 静态检查与场景边界 |
| `make client-smoke` | 对局页结构、状态门控、拖拽、引导与战斗演示 |
| `make m1b-e2e` | 客户端到服务端一局真实链路 |

## 12. 实施顺序

1. 共享层：`theme.gd` 增加 `top-bar-height` 28、`river-height` 32、`hero-row-height` 84、`hero-card-width` 64、`hero-card-height` 84、`shop-icon-size` 40、`opponent-hp-width` 360、`hero-cover-height` 40、`hero-name-height` 14、`hero-stats-height` 30，`portrait-size/portrait-height` 改为 64，删除 `side-width`、`combat-banner-height`；`panel_style` 增加 `bare` 与 `accent-bare`；`font-micro` 8 与原子字号 `8`；`client/ui/icons.gd` 提供四个图标；`CardDB` 读取 `skills.json`；`GameUiState.can_drag`。
2. 页面内组件：`game_icon_button.gd`、`game_hero_card.gd`、`game_portrait.gd`、`game_river.gd`、`game_top_bar.gd`、`game_hero_row.gd`、`game_opponent_view.gd`、`game_hand_row.gd`、`game_shop_view.gd`。
3. 组合：重写 `game_recruit_view.gd`，改造 `game_combat_view.gd`。
4. 页面：重排 `game.gd` 的构建、刷新、阶段切换与计时取值，删除 `GameSidePanel` 引用。
5. 引导：`first_game_guide.gd` 步骤 2/3 目标改为新节点。
6. 测试：按 11.2 改写并补 11.3 断言。
7. 验证与回写：跑 11.4；回写 `docs/current/product/ui/pages/game.md`、`tokens.md`、`interactions.md`、`components/first-game-guide.md` 与索引。

## 13. 验收映射

| product.md 验收 | 技术落点 |
| --- | --- |
| 1 无右栏 | 删除 `GameSidePanel`；`ui_smoke_side` 无右栏节点断言 |
| 2 招募顺序 | `RecruitPanel` 子节点顺序与 3.1 结构；商店操作图标 40×40；结构冒烟 |
| 3 战斗顺序 | `CombatPanel` 子节点顺序；商店与操作行随招募视图隐藏 |
| 4 河道 | `RecruitRiver` 中央回合、右端倒计时翻转；`CombatRiver` 中央 `CombatBanner` 始终显示回合并在播完后追加结果、右端「战斗中」 |
| 5 手牌 | `HandStrip` 无面板/标题，贴底；`HandRow` 10 张单行、悬停 1.12、空态 |
| 6 英雄行 | `HeroRow` 居中、84 高；`HeroCard` 64×84；攻击左下、生命右下、时代居中；`HeroPowerButton` 费用/次数角标 |
| 7 对手英雄 | `OpponentView` 战斗顶部；幽灵角标与无血量条 |
| 8 顶部工具区 | `TopBar` 状态左、三入口右；页面底部无状态行 |
| 9 左栏不变 | `GamePlayersRail` 不改 |
| 10 操作可用性 | 商店操作、出售、拖拽、结束招募、英雄技能的 affordance 与门控沿用；图标 tooltip 说明含义与费用/次数 |
| 11 既有浮层 | 图鉴、引导、选择浮层、详情层、战斗演示、战报只改锚点 |
| 12 1280×720 单行 | 3.2 预算 + 11.3 底边与单行断言 |

## 14. 开放问题

- 无。协议、数据与美术资源均已确认不在本轮范围内。
