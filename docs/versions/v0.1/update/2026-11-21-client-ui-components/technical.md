# 客户端 UI 组件化重构

> 状态：已实施
> 需求归属：v0.1
> 产品依据：无产品改动，仅代码结构与文档落位调整

## 1. 改动边界

- 只改客户端 UI 的组织方式与文档入口，不改视觉、布局、交互行为，不改服务端、协议与卡牌数据。
- 不引入 `.tscn` 组件，沿用 GDScript `class_name` + 静态构造；文件名沿用 snake_case。

## 2. 分层落位

| 层 | 目录 | 内容 |
| --- | --- | --- |
| 共享设计层 | `client/ui/` | 令牌、图标、通用控件、卡面绘制、跨页组件 |
| 页面 | `client/pages/<page>/` | 页面 `.gd`/`.tscn` 与只服务该页的组件 |
| 状态与数据源 | `client/autoload/`、页面目录 | 现状不变 |

- 页面只依赖 `ui/` 与自身目录，不依赖其他页面目录。
- 图鉴组件被图鉴页与对局覆盖层共用，落 `client/ui/`；`pages/gallery/` 只保留页面壳。
- 对局与房间共用详情层，`GameDetailLayer` 迁入 `ui/` 并更名为 `DetailLayer`。

## 3. `ui.gd` 拆分

| 新文件 | 职责 |
| --- | --- |
| `ui/layout.gd`（`UILayout`） | 安全区、内容框、共享图层 |
| `ui/navigation.gd`（`UINavigation`） | 路由常量、页面导航、错误码去向 |
| `ui/wording.gd`（`UIWording`） | 枚举与错误文案 |
| `ui/format.gd`（`UIFormat`） | RFC3339 时间转换 |
| `ui/controls.gd`（`UIControls`） | 标签、按钮、面板、背景、胶囊、区块标题、清空、置灰 |
| `ui/art.gd`（`UIArt`） | 美术贴图加载 |
| `ui/drag_preview.gd`（`UIDrag`） | 拖拽预览包裹层 |
| `ui/avatar.gd`、`hp_bar.gd`、`rank_badge.gd`、`skill_list.gd` | 单组件单文件 |
| `ui/long_press.gd`（`LongPressTracker`） | 长按判定，阈值参数化 |

- 删除无调用方的 `UI.hero_tile`。

## 4. 页面私有组件

| 页面 | 组件 |
| --- | --- |
| login | `LoginField` |
| lobby | `LobbyTopBar`、`LobbyActions`、`LobbyBottomBar`、`FriendRow`、`FriendRequestRow`、`RecentPlayerRow` |
| room | `RoomPlayerRow`、`RoomCandidateList`、`RoomInviteRow`、`RoomActionBar` |
| result | `ResultRanking`、`ResultStatsSection`、`ResultBoard` |
| game | 沿用现有视图组件；详情层内容块拆至 `ui/detail_sections/` |

## 5. 验证

- `make client-import`、`make client-check`、`make client-smoke` 全部通过。
- `python3 -m unittest tests.test_client_boundary` 通过。
- `res://scenes/` 引用清零，`UI.` 引用清零。

## 6. 实施结果

- `ui.gd` 已拆空删除；页面迁入 `client/pages/<page>/`，跨页组件与图鉴组件留在 `client/ui/`。
- 详情层内容块拆至 `ui/detail_sections/`，公共辅助收为 `DetailKit`；`RoomHeroDetailLayer` 保留覆写点。
- 长按状态机抽为 `LongPressTracker`，详情层与选英雄页共用；触控冒烟改读追踪器状态。
- 图鉴页只保留背景、组件装配与返回大厅，未再拆新组件。
- 结算页字号、间距与高亮色全部按令牌取值；设计系统补充 `font-title-xl`（20）、`space-md-plus`（10）与 `color-highlight`（`#8fd3ff`），结算页与另外两处 10 间距的复用点一并替换，无遗留裸值。
- 删除无调用方的 `UI.hero_tile`、`hero-tile.md` 与 `hero-tile-width` 令牌。
