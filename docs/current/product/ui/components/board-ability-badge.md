# 棋盘随从技能角标

> 状态：已实现
> 入口：`client/pages/game/board_ability_badge.gd`（`class_name BoardAbilityBadge`，`extends Button`）
> 验证：`make client-smoke`（`client/tests/ui_smoke.gd` 的随从技能提交与瞄准用例；`client/tests/ui_smoke_side.gd` 的角标渲染与可用性用例）

## 用途

棋盘随从卡上的主动技能入口：显示技能费用与剩余次数，点击或按住即发动服务端下发的技能。角标不计算规则、不构造意图，可用性与数值全部由对局页按服务端视图注入。

## 外观与尺寸

- 叠在卡片右上角、纵向排列，不进入卡面内部；顶部避开 ★/潜行角标行，底部为攻血数值留出空间。
- 文本按文案键 `ui.game.minion_ability.badge`（默认模板 `◆<费用>×<剩余次数>`）；`◆` 是费用字形，内置字体子集不含 `⚡`。
- 高度上限取 `hit_small()`，宽度取 `space-3xl + space-lg`，字号 `font-caption`；样式取 `UITheme.button_style("slot", state)`，可发动时用强调色描边。
- 不可用（`can_use = false`）时置灰、文本转 `color-text-muted`，不响应点击。
- 悬停 tooltip 依次显示技能名与技能文本（`CardDB.skill_info`），缺省时留空，不暴露技能 id。

## 接口

- 字段：`minion_id`、`skill_id`、`cost`、`uses_left`、`can_use`。
- `refresh()`：按字段刷新文本、tooltip 与可用状态；尺寸由调用方按卡片剩余空间设置。
- 只发出继承的 `pressed` / `button_down`，不携带意图参数，也不做本地扣费或次数预判。
- 可用性由对局页注入：服务端 `abilities[].canUse` 与拖拽同批的 `can_rearrange` 相与；`disabledReason` 随视图下发，但控件与页面当前都不消费。

## 与页面的协作

- `GameRecruitView._minion_card` 在卡片外层建 `AbilityBadges` 容器，按 `abilities` 顺序实例化 `MinionAbility<position>_<skillIndex>`，并把角标登记到 `_ability_badges` 供 `ability_rect()` 查询。
- 页面连接 `minion_ability_pressed` / `minion_ability_down`：不需要目标时点击直接发 `UseMinionAbility { minionId, skillId }`；需要目标且可用时按下缓存含 `minionId` / `skillId` 的意图，从 `ability_rect()` 起手复用 `GameAimLayer.begin_skill`，命中候选后补 `target` 提交。
- 拖动棋盘卡仍从卡身开始，角标不参与拖拽；拖拽进行期间角标让出命中（`MOUSE_FILTER_IGNORE`），落点仍由棋盘容器处理；视图重建时清空登记并重排角标。

## 边界

- 不计算候选、不缓存意图、不做本地结算；一切以服务端下发的 `abilities` 与后续视图为准。
- 不渲染对手棋盘的技能：对手视图不下发 `abilities`，角标自然不出现。
- `HERO_POWER_LIMIT` 的文案由页面按最近一次意图区分英雄与随从技能，组件本身不读错误码。

## 使用者

- 对局页：[game.md](../pages/game.md)
- 通用交互：[interactions.md](../interactions.md) §3.1
- 客户端技术现状：[client.md](../../../technical/client.md)
