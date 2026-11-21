# 战斗开场 · 设计（已确认）

> 日期：2026-11-21
> 范围：对局页战斗阶段的居中开战画面
> 状态：已获用户确认（2026-11-21），总时长按用户要求由 1 秒缩短为 0.6 秒

## 1. 意图与原因

战斗阶段目前从招募视图直接切到双方随从行并立刻逐条播放动作，只有音效提示，没有视觉开场，阶段切换缺少仪式感。用户要求「战斗开始时在屏幕中间显示一个战斗开始的画面，像炉石一样」。

已确认方向：**双方对阵 VS 画面**（三方向比较中的第二项）。

## 2. 目标与非目标

**目标**

- 战斗首次装载、首条动作尚未播放时，在战斗组合中央显示约 0.6 秒的双方对阵画面：我方头像与名字、对手头像与名字、中间 VS 标记，背景压暗。
- 画面显示期间暂停战斗数据推进与碰撞演出，首条攻击在画面淡出后才发生。
- 重连续播进入战斗中段时不显示，与战斗开始音效的判定一致。

**非目标**

- 不改服务端、协议、规则引擎与战斗节奏常量（`PACE_MS` / `TAIL_MS`）。
- 不新增音效；已有 `combat_start` 音效随画面对齐播放。
- 不做可点击跳过、不做棋盘分开位移、不改招募视图。
- 不改主题、不新增设计令牌与图片资源。

## 3. 已确认的选择

| 决策 | 选择 | 备选与放弃原因 |
| --- | --- | --- |
| 视觉方向 | 双方对阵 VS 画面 | 中央题字缺少对阵感；棋盘裂开实现与回归成本最高 |
| 暂停方式 | 开场期间不调用 `CombatPlayback.advance()` / `CombatFx.advance()` | 只压暗不暂停会让首次攻击藏在画面下 |
| 显示判定 | `elapsed_ms < CombatPlayback.PACE_MS` | 与开始音效同一判定，不另造标志 |
| 数据来源 | 双方 `players` 条目 + `heroCardId`，复用 `CardDB` 与 `GamePortrait` | 不新增身份推导逻辑 |
| 幽灵战 | 对手块压暗并加「幽灵」角标 | 与顶部身份块保持一致 |
| 对手缺失 | 渲染回退首字色块与「未知英雄」 | 开场仍完整，不阻塞演示 |

## 4. 设计

### 4.1 组件

新增 `client/scenes/combat_intro.gd`（`class_name CombatIntro extends Control`），沿用 `CombatFx` 的帧驱动写法，不用 `Tween`，便于无头冒烟按帧断言。时长常量集中在文件顶部：

- `INTRO_MS := 600.0`，入场 `ENTER_MS := 160.0`，出场 `EXIT_MS := 160.0`。

节点结构（全屏、`mouse_filter = MOUSE_FILTER_IGNORE`）：

```
CombatIntro（全屏，盖在 CombatFxLayer 之上）
├─ Scrim        ColorRect，UITheme.alpha("color-bg", 0.6)
└─ VsCenter     CenterContainer
   └─ VsCard    PanelContainer（panel_style("raised")，独立底板遮住河道与随从卡）
      └─ VsColumn VBoxContainer（space-md）
         ├─ VsRow HBoxContainer（space-xl）
         │  ├─ OwnBlock      VBox：GamePortrait 64/48 + 名字 + 英雄名
         │  ├─ VsMark        HBox：金色短线 + "VS"（font-hero，color-accent）+ 金色短线
         │  └─ OpponentBlock VBox：同上；幽灵战压暗并加「幽灵」胶囊
         └─ Caption Label「战斗开始」（font-body，color-text-dim，居中）
```

- 头像尺寸取 `UITheme.stage_metric("portrait-size")`（桌面 64、iPhone 48），名字 `font-title`，英雄名 `font-caption` + `color-text-dim`。
- 双方身份取 `state.view.players`：我方按 `GameStore.player_id`，对手按 `state.view.opponentPlayerId`；头像纹理 `CardDB.art_path(hero_id, "avatar")`，底色 `UITheme.playstyle_color(CardDB.primary_playstyle(...))`，与 `GameOpponentView` 一致。
- 幽灵判定与顶部身份块一致：`GameStore.combat_is_current() and last_combat.mirror`；名字追加 `ui.game.combat.ghost_suffix`，并加 `ui.game.side_panel.ghost` 胶囊。

### 4.2 动效

`advance(delta_ms) -> bool`（返回是否仍在显示）：

- 入场：`k = t / ENTER_MS`，底板整体 `scale 0.94 → 1.0`、`modulate.a 0 → 1`（实机截图发现无底板时与河道回合数重叠，补上 `VsCard`）。
- 停留：保持满不透明。
- 出场：最后 `EXIT_MS` 内 `modulate.a 1 → 0`、`scale 1.0 → 1.04`（在入场缩放上追加，避免回跳）。
- 结束隐藏并清空节点。

`show(state)` 重建双方内容并归零计时；`clear()` 隐藏并释放。

### 4.3 接入

`client/scenes/game_combat_view.gd`：

- `_intro := CombatIntro.new()` 在 `_fx` 之后 `add_child`，自然盖住阵容与 FX；不参与布局。
- `_start_combat()`：在 `seek()` 与 `_combat_turn = GameStore.turn` 之后，若 `elapsed_ms < CombatPlayback.PACE_MS`，`_intro.show(_state)`；与 `Sfx.CUE_COMBAT_START` 同一分支。
- `advance(delta)`：先 `_intro.advance(delta * 1000.0)`；返回真时直接返回，不推进 `_combat` 与 `_fx`。返回假后走原有推进、渲染与横幅刷新。
- `reset()`：`_intro.clear()`，下一场重新判定。
- `playing()` / `finished()` 不变；战斗结束回报仍由播放器完成，开场只延后其时间点。

### 4.4 文案

`client/text/zh_CN.csv` 新增人工 key：

- `ui.game.combat.intro` → `战斗开始`
- `ui.game.combat.vs` → `VS`

VS 标记用 `ui.game.combat.vs`，名字/英雄名复用现有显示名与 `ui.game.side_panel.unknown_hero`。新增字若不在字体子集内，按 `make font-subset` 重新生成。

## 5. 状态覆盖

| 状态 | 表现 |
| --- | --- |
| 普通对战 | 双方头像、名字、英雄名与 VS，0.6 秒后淡出 |
| 幽灵战 | 对手块压暗，名字追加「（幽灵）」，带「幽灵」胶囊 |
| 对手缺失 | 对手位显示「?」色块与「未知英雄」，开场照常 |
| 本人已淘汰 | 我方块正常显示，开场结束后进入常规观战视图 |
| 重连续播（`elapsed_ms ≥ PACE_MS`） | 不显示开场，直接进入当前进度 |
| iPhone / iPad / 桌面 | 头像用紧凑或标准令牌，整层居中，不超出战斗组合 |

## 6. 验证

- 无头冒烟 `client/tests/ui_smoke_combat.gd`：
  - 首次装载可见 `CombatIntro`；推进 0.5s 时仍在开场且播放器未应用首条动作；推进满 0.6s 后开场隐藏；再推进 0.4s 后首条动作已应用。
  - `startedAt` 已在 10 秒前时不显示开场，且不重复开始音效。
  - 幽灵战夹具在开场内出现「幽灵」胶囊。
  - 开场存在与否都不影响播完后的 `CombatPlayed` 回报。
- `make client-check`、`make client-smoke`、`make data-check`。
- 桌面与 iPhone 各跑一次 `make client-run` 目视确认开场与淡出。

## 7. 文档回写

- `docs/current/product/ui/pages/game.md`：状态表增加「战斗开场」行，补充约 0.6 秒暂停与不可交互说明。
- `docs/current/technical/client.md`：组件清单加入 `combat_intro.gd`，说明判定、暂停方式与时长常量。
- 不改版本功能表（不新增产品能力，只是现有战斗阶段的开场表现）。
