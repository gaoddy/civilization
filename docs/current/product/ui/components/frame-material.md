# 框体材质

> 状态：已实现
> 入口：`FrameMaterial`（`client/ui/frame_material.gd`）与 `client/ui/frame_material.gdshader`

## 用途

给卡、面板与按钮叠一层由 shader 绘制的黄铜倒角边框与内侧阴影：外沿受光、内沿压暗、边缘带金属颗粒，中间完全透明。材质只改外观，不参与布局、不拦截鼠标，也不改变点击与拖拽判定。

`surface` 打开时额外铺一块数字底板：整块深铜面补在内容之下，供段位、倒计时与数字这类没有整块底色的信息表面使用；边框仍压在内容之上。

## 变体

| 变体 | 触发 | 表现 |
| --- | --- | --- |
| 基础框 | `FrameMaterial.apply(target)` | `color-accent` 黄铜、单层倒角、悬停提亮 20% |
| 时代合金 | 卡面 `era`（`CardFaceBase.add_frame`） | 7 种合金基色，形状与光照不变 |
| 三连金框 | 卡面 `golden` | 足金 `color-golden`、边框加厚 0.8px、外发光更强，不加内圈线 |
| 引导流光 | 按钮、可拖拽卡 | `guide` 打开时沿对角线扫过一道亮光；其余表面保持静态 |
| 数字底板 | `opts.surface` | 内部铺满深铜面，左上受光、右下压暗，细颗粒；边圈与流光仍按自身权重叠在底板上 |

另有一个未启用的对照变体 `FrameMaterial.Style.ETCH`（蚀刻铜锈：暗铜底、强颗粒与锈斑、内侧第二道凹槽），shader 已实现但客户端全部走 `BEVEL`，只在需要换方向对比时手工切换。

## 属性与事件

| 接口 | 说明 |
| --- | --- |
| `apply(target, style, opts)` | 给单个控件叠材质；`opts` 支持 `tint`、`golden`、`guide`、`surface`。重复调用只更新参数 |
| `apply_to_tree(root, style, opts)` | 给整棵子树里最外层的表面统一叠材质，嵌套面板不重复描边 |
| `dress_page(page)` | 页面级接入：等首帧布局落定后整页扫一遍，供页面 `_ready()` 调用 |
| `set_guide(target, guide)` | 切换引导流光；`target` 自身与后代上的材质一起切换 |
| `set_guide_preference(target, guide)` | 设置控件是否跟随引导流光；偏好写入 meta，页面级 `dress_page` 重扫后仍生效 |
| `clear(target)` / `clear_tree(root)` | 移除材质 |
| `tint_for_era(era)` / `golden_tint()` | 取时代合金色与足金色，供绘制代码与预览共用 |

- 材质节点命名为 `FrameMaterial`，挂在目标控件下的 `FrameMaterialHolder`（`Node2D`）上并铺满目标矩形；容器只排列 `Control` 子节点，材质层因此不会被 `PanelContainer` 的内容内边距挤进内容区，祖先缩放照常生效。`mouse_filter` 为 `IGNORE`。
- 数字底板节点命名为 `FramePlate`，挂在 `FramePlateHolder`（`Node2D`）下并铺满目标矩形。`Label` 的文字由控件自身绘制，底板用 `show_behind_parent` 落到文字后面；`PanelContainer` / `Panel` 的底色先于子节点绘制，底板插到第一个子节点，内容仍在底板之上。
- 圆角取控件样式盒的圆角并收敛到短边的一半：胶囊与小角标的圆角不会被画成溢出的倒角。
- 悬停提亮由目标控件的 `mouse_entered` / `mouse_exited` 驱动，不新增状态机。
- 页面装载时隐藏的表面同样接入材质（例如排队卡、座位列表、战斗视图）：材质层随控件一起隐藏，显示后按 `resized` 补齐矩形；不按 `visible` 过滤，避免显示后一直没有边框。

## 需要材质的组件清单

| 组件 | 接入点 | 变体 | 说明 |
| --- | --- | --- | --- |
| 卡面 `CardFace`（手牌、棋盘、商店、图鉴、详情、结算） | `CardFaceBase.add_frame` | 时代合金 + 三连金框 | 由卡面 `card_id` 与 `golden` 自动取值 |
| 按钮 `UIControls.styled_button` 及复用 `apply_button_style` 的按钮子类 | `UIControls.apply_button_style` | 基础框 + 引导流光 | 所有可操作按钮默认带流光 |
| 胶囊按钮 `PillButton` | `PillButton._apply_styles` | 基础框 + 引导流光 | 与 `styled_button` 一致 |
| 面板 `UIControls.panel`、`UIControls.card_panel` | 控件工厂 | 基础框 | 弹窗、抽屉、区块面板统一走工厂 |
| 页面静态面板（页面内直接 `new` 的 `PanelContainer` / `Panel`） | `FrameMaterial.dress_page(page)` | 基础框 | 页面 `_ready()` 构建完成后调用一次 |
| 运行期新建的裸面板 | 组件内 `FrameMaterial.apply(control)` | 基础框 | 不经过工厂与卡面时由组件自行接入 |
| 可拖拽卡 `HandCard` / `ShopCard` / `BoardCard` | `drag_allowed` setter | 引导流光 | 仅在当前允许拖拽时亮起 |
| 胶囊标签 `UIControls.pill_label`（数值、状态与计时；`plain` 除外） | 控件工厂 | 数字底板 | 收费色变体取该变体色作底板基色 |
| 名次角标 `RankBadge` | `RankBadge.create` | 数字底板 | 座位头像内的名次数字 |
| 对局数字徽标（消耗 / 次数） | `GameIconButton._make_badge` | 数字底板 | 运行时创建，必须在创建时自挂；底色压到半透，底板才透得出来 |
| 无底色的倒计时 / 数字标签 | `UIControls.plate_wrap(label)` | 数字底板 | 排队等待计时、选英雄倒计时；透明面板 + 内边距，标签本身仍由调用方持有 |

### 不上材质的组件

| 组件 | 原因 |
| --- | --- |
| 卡面信息条 `card_panel("cover-info")` | 卡面已统一描边，再加一层会出现双线 |
| 卡内子面板（顶栏、信息区、冻结与潜行蒙版等） | 随卡面整棵子树跳过，一张卡只有一圈边框 |
| 整屏底图 `PageSurface` / `PageBackground` | 不是表面，加边框会把画面圈成一整块 |
| 纯文字信息行（个人主页字段行、结算积分与段位行等） | 已有承载面板，再加一层底板会与页面面板重复；需要独立成块时用 `plate_wrap` 显式包裹 |

## 禁用场景

- 不给整屏背景、遮罩与纯装饰层加材质；需要强调时用描边或发光，不套整圈边框。
- 不在页面里直接创建 `ColorRect` 挂 shader；统一走 `FrameMaterial`，否则悬停、尺寸与时代色不会同步。
- 引导流光只给「现在就该操作」的表面，不给静态信息卡。
- 页面可按需收窄流光范围：`FrameMaterial.set_guide_preference(control, false)` 把偏好写进 meta，整页重扫后仍保持静态。大厅主内容区只保留「开始匹配」按钮的流光；进入排队后按钮变「取消匹配」，流光随之移除，其余主内容区按钮一律静态；弹窗与抽屉内的操作按钮沿用默认规则。

## 使用者

- 卡面：手牌、棋盘、商店、图鉴、详情与结算页的全部卡牌。
- 按钮与面板：登录、大厅、房间、对局、结算、图鉴与账户页。
- 时代色与列表框由 [tokens.md](../tokens.md) 的 `color-era-*`、`color-golden` 提供。
- 预览入口：`client/tests/previews/app_ui.gd`（按键 1–9 逐屏查看，0 开关材质）。
