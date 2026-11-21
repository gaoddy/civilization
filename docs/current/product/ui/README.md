# 客户端 UI

> 状态：已实现
> 代码：`client/ui/theme.gd`（令牌）、`client/ui/controls.gd` 等共享构件、`client/pages/`
> 验证：`make client-check`、`make client-smoke`

本文记录客户端已实现的 UI 现状：视觉语言、令牌入口、页面与共享组件索引。设计流程与评审规则见项目 runbook（仓库外）。

## 1. 视觉语言

- 深色酒馆基调：近黑的背景、分层卡片、金色强调，状态色只表达结果（成功、危险、信息）。
- 资源约束：工程没有字体与主题资源文件，界面由 Godot 内置控件与 `StyleBoxFlat` 构建；位图资源只有卡牌立绘、头像、卡面与页面背景 `assets/ui/bg/bg_main.png`，清单与回退规则见 [client.md](../../technical/client.md#10-美术素材层)。英雄「头像」无素材时仍是玩法色块加名字首字的排版块。
- 代码绘制：卡框（`CardFrame`）、顶部星级、时代象形（`EraGlyph`）与攻防底牌（`StatPlate`）由代码绘制，不占位图；入口见第 4 节组件表。稀有度不是展示维度，卡面以 `level` 的 1–7 星表达成长档位。
- 深浅色：只有深色一套，不实现浅色主题与主题切换，UI 评审不做浅色检查。

## 2. 令牌与共享构件

| 入口 | 内容 |
| --- | --- |
| `client/ui/theme.gd`（`UITheme`） | 原子 → 语义 → 组件与场景三层令牌；取值见 [tokens.md](tokens.md) |
| `client/ui/controls.gd`（`UIControls`） | 共用控件构造：label、button、panel、margin、背景、胶囊、区块标题与空态 |
| `client/ui/navigation.gd`（`UINavigation`） | 页面路径常量、统一导航入口 `navigate` 与错误码去向 `error_route` |
| `client/ui/wording.gd`（`UIWording`） | 枚举展示与错误文案取用（`enum_text` / `error_text` / `error_tooltip`） |
| `client/ui/layout.gd`（`UILayout`） | 安全区包裹、内容框与共享图层 |
| `client/ui/art.gd`（`UIArt`） | 图片取用与铺层（`texture` / `layer`） |
| `client/ui/avatar.gd`、`hp_bar.gd`、`rank_badge.gd`、`skill_list.gd` | 单组件：头像块、血量条、名次角标、技能列表 |
| `client/ui/icons.gd`（`Icons`） | 代码绘制的矢量图标；清单与绘制约定见 [icons.md](icons.md) |
| `client/text/zh_CN.csv` | 唯一的运行时中文文本表：卡牌与技能名称、效果、枚举展示、错误与页面文案；来源与取用规则见 [client.md](../../technical/client.md)「国际化与文本表」 |

- 页面通过 `UIControls` 等共享构件与 `UITheme.metric` 取值，不直接写颜色、字号、间距与圆角。
- `UITheme` 的原子层与语义层按刻度扩展，不因单个页面新增。

## 3. 页面

| 页面 | 场景 | 文档 |
| --- | --- | --- |
| 登录 | `pages/login/login.tscn` | 未单独成文 |
| 大厅 | `pages/lobby/lobby.tscn` | 未单独成文 |
| 房间（开局前） | `pages/room/room.tscn` | [pages/room.md](pages/room.md) |
| 对局 | `pages/game/game.tscn` | [pages/game.md](pages/game.md) |
| 结算 | `pages/result/result.tscn` | [pages/result.md](pages/result.md) |
| 卡牌图鉴 | `pages/gallery/gallery.tscn` | [pages/gallery.md](pages/gallery.md) |

## 4. 组件

| 组件 | 入口 | 文档 |
| --- | --- | --- |
| 头像块 | `Avatar.create` | [components/avatar.md](components/avatar.md) |
| 数值胶囊 | `UIControls.pill_label` | [components/pill.md](components/pill.md) |
| 按钮变体 | `UIControls.styled_button` | [components/button.md](components/button.md) |
| 卡片容器 | `UIControls.card_panel` | [components/card-panel.md](components/card-panel.md) |
| 卡框与星级 | `CardFrame`（`client/ui/card_frame.gd`） | [tokens.md](tokens.md) |
| 攻防底牌 | `StatPlate`（`client/ui/stat_plate.gd`） | [tokens.md](tokens.md) |
| 时代象形 | `EraGlyph`（`client/ui/era_glyph.gd`） | [icons.md](icons.md) |
| 卡牌图鉴 | `CardGallery`（`client/ui/card_gallery.gd`） | [components/card-gallery.md](components/card-gallery.md) |
| 图鉴视图切换 | `GalleryModeBar` | [components/gallery-mode-bar.md](components/gallery-mode-bar.md) |
| 图鉴顶部筛选栏 | `GalleryFilterBar` | [components/gallery-filter-bar.md](components/gallery-filter-bar.md) |
| 图鉴分类目录 | `GalleryToc` | [components/gallery-toc.md](components/gallery-toc.md) |
| 图鉴卡面网格 | `GalleryGrid` | [components/gallery-grid.md](components/gallery-grid.md) |
| 图鉴卡面单元 | `GalleryCardTile` | [components/gallery-card-tile.md](components/gallery-card-tile.md) |
| 图鉴常驻详情 | `GalleryDetail` | [components/gallery-detail.md](components/gallery-detail.md) |
| 首局引导 | `FirstGameGuide` | [components/first-game-guide.md](components/first-game-guide.md) |
| 技能瞄准层 | `GameAimLayer`（`client/pages/game/game_aim_layer.gd`） | [components/aim-layer.md](components/aim-layer.md) |
| 房间候选英雄卡 | `RoomHeroCard`（`client/pages/room/room_hero_card.gd`） | [pages/room.md](pages/room.md) |
| 房间悬停详情层 | `RoomHeroDetailLayer`（`client/pages/room/room_hero_detail_layer.gd`） | [pages/room.md](pages/room.md) |

## 5. 交互约定

通用交互、状态与无障碍规则见 [interactions.md](interactions.md)；iOS 端的横屏、缩放与触控规则见 [mobile-adaptation.md](mobile-adaptation.md)；页面特有交互写在页面文档内，同一规则只写一处。

## 6. 线框图

- 入口：`make ui-wireframe PAGE=gallery`；`ALL=1` 生成全部页面，`DEVICE=phone` 按手机档位（960×540）生成，产物在 `output/wireframes/<页面>[-<设备>].{json,svg,png}`，该目录不提交。
- 做法：`client/tests/wireframe_probe.tscn` 作为主场景装载页面，等布局解算后导出每个可见 Control 的 `get_global_rect()`；`scripts/ui_wireframe.py` 画成 SVG，再交给本机浏览器截图出 PNG（没有浏览器时退回引擎栅格化，只有框线没有文字）。
- 边界：矩形来自真实场景，不是手绘估算；但页面内容依赖服务端状态，`room`、`game`、`lobby` 在无数据时只出加载/空态骨架。`gallery` 用离线卡表，最适合核对布局。
- 线框图只作布局核对，不替代实机验证；颜色、字体与状态表现仍以 `make client-run` 与页面文档为准。
