# UI 资产重做 · 设计（已确认）

> 日期：2026-11-21
> 范围：卡框、星级、时代象形、攻防底牌、页面背景
> 状态：已获用户确认（2026-11-21）

## 1. 意图与原因

`scripts/gen_ui_assets.py` 生成的 48 张几何 PNG 质量不足，且未被任何代码引用：

- 卡框按 448×584 绘制、按 112×146 显示，主描边缩成 1.25px，角饰与刻度全部丢失。
- `bg_main` 是网格加同心圆，平铺重复明显，像工程图纸。
- 面板、按钮、胶囊、图标底槽与现有 `StyleBoxFlat` 能力重复：位图不能随令牌改色，拉伸会糊。
- 生成器依赖 Pillow，本机所有 Python 均未安装，脚本当前无法复现。

## 2. 目标与非目标

**目标**

- 卡框双细线、无角饰，代码绘制。
- 顶部中央 1–7 星表示星级。
- 右上角时代象形（无外框、小尺寸、低调）。
- 攻防数字各带底牌：攻击黄圆、血量血滴，数字白字黑描边。
- 页面背景改为「纯质感深底」位图。
- 删除与 `StyleBoxFlat` 重复的底材位图与稀有度徽章，生成器零依赖。

**非目标**

- 不改页面结构、业务状态与交互流程。
- 不做浅色主题。
- 不引入新的第三方依赖。
- 不改服务端数据与协议（稀有度原本就只是客户端派生，现已整体移除）。

## 3. 已确认的选择

| 决策 | 选择 | 备选与放弃原因 |
| --- | --- | --- |
| 视觉方向 | 铜版星图（去角饰后） | 角饰在 112px 下过密，用户要求移除 |
| 稀有度 | 移除 | 改为直接展示星级 `level` |
| 卡框实现 | 代码 `_draw()` | 位图在两种尺寸下都会退化 |
| 星级位置 | 卡框顶部中央一排四角星 | 不占信息区，随卡框等比缩放 |
| 时代图标 | 无外框象形，卡面 14px / 详情 24px，金色 62% | 印章版过重，用户要求低调 |
| 攻防底牌 | 攻击黄圆、血量血滴，白字黑描边 | — |
| 背景 | 纯质感深底位图 | 网格星图重复感强 |
| 底材位图 | 全部删除 | 与 `StyleBoxFlat` 能力重复且更差 |

## 4. 实施内容

### 4.1 资产删除

- `client/assets/cards/frames/`（4 张 + `.import`）
- `client/assets/badges/`（4 张 + `atlas.png` / `atlas.json`）
- `client/assets/ui/button/`、`ui/icon/`、`ui/panel/`、`ui/pill/`、`ui/border/`、`ui/bg/bg_panel.png`
- 根目录 `DELIVERY_MANIFEST.md`（一次性交付清单，内容并入 UI 文档）
- 失效的 `.import` 由 `make client-import` 清理

### 4.2 生成器重写

`scripts/gen_ui_assets.py` 只生成 `client/assets/ui/bg/bg_main.png`：

- 512×512 RGB，整屏拉伸，不平铺。
- 基色 `color-bg`，中心极淡径向提亮（约 `#2b3a52` 88% → `#18212f` 44% → 透明），纸张颗粒噪声（≤ 6% 不透明度）。
- **纯标准库**（`zlib` + `struct`），与 `gen_app_icon.py` 一致，不依赖 Pillow。
- 固定种子，重跑字节级一致。

脚本名与文档同步改为「背景生成器」；不再生成卡框、徽章、底材。

### 4.3 程序化卡框与星级

新增 `client/ui/card_frame.gd`（`class_name CardFrame extends Control`）：

- `static func create(level: int) -> CardFrame`，全矩形铺满卡面，`MOUSE_FILTER_IGNORE`。
- `_draw()` 按控件尺寸等比放缩（基准 112×146）：
  - 外线：`gold-600` 60%，圆角 8
  - 中缝：`color-bg` 实线，把外线与内线隔开
  - 内线：`ink-500` 55%，内缩 4.5，圆角 6
  - 顶部中央：`level` 颗四角星，金色，随尺寸缩放
- 接入点：`CardFaceBase.add_frame(face_node, card_id, hero := false)`，由各卡面组装器在 `add_info` 之后调用；不能放在 `create_face` 里，否则底部信息区会盖住下边框线。`level` 取 `CardDB.card(card_id).get("level", 1)`；英雄卡不画星级。

### 4.4 攻防底牌

新增 `client/ui/stat_plate.gd`（`class_name StatPlate extends Control`）：

- `kind`：`attack` / `health`。
- `_draw()`：攻击为金色圆形渐变（`gold-400` → `gold-600`，深色描边）；血量为红色血滴（`color-danger` 渐变，深色描边）。
- 内嵌 `Label`，白字 + 黑色描边（`font_outline_color` 黑、`outline_size` 2）。
- 尺寸随字号：`font-body` 时 18px，`font-caption` 时 15px。
- 接入点：`CardFaceBase.add_info` 把 `CardAttack` / `CardHealth` 的文本改为放进 `StatPlate`；保留同名 Label 供 `set_stats` 查找。
- 位置：攻防分别贴信息区左右边缘。

### 4.5 时代象形

在 `client/ui/icons.gd` 增加 `draw_era(canvas, era_id, rect, color, width := 1.4)` 图标（`prehistoric` / `primitive` / `classical` / `feudal` / `industrial` / `modern` / `future`），源坐标 16×16 网格，按矩形等比映射绘制，1.4 描边、主形 24% 填充：

| 时代 | 象形 |
| --- | --- |
| 史前 | 石斧（木柄 + 燧石斧头 + 绑扎线） |
| 原始 | 篝火（外焰 + 内焰 + 交叉木柴） |
| 古典 | 神庙（山花 + 额枋 + 三柱 + 基座） |
| 封建 | 城堡（城垛 + 拱门） |
| 工业 | 齿轮（六齿 + 轮轴） |
| 现代 | 原子（核 + 双轨道） |
| 未来 | 火箭（箭身 + 舷窗 + 尾翼 + 焰） |

- 卡面右上角 14px，金色 62% 不透明度。
- `add_corner_badges` 的 `era_text` 参数改为 `era_id`，画象形而不是文字胶囊；5 个调用点同步。
- 图鉴详情卡名旁 24px 象形。

### 4.6 背景接入

- `client/ui/ui.gd` 新增 `static func background() -> Control`：`TextureRect` + `bg_main.png`，整屏拉伸，缺图回退 `color-bg` 纯色。
- 6 个页面（login/lobby/room/result/game/card_gallery）替换 `ColorRect` 背景；`card_gallery` 保留节点名 `GalleryBackground`。

### 4.7 文档回写

- `docs/current/product/ui/README.md` §1：位图资产只剩页面背景，卡框/星级/时代象形/攻防底牌由代码绘制。
- `docs/current/product/ui/tokens.md`：新增令牌若需要（星级尺寸、时代象形尺寸、攻防底牌尺寸），只加与本次相关的项。
- 删除 `DELIVERY_MANIFEST.md`。

## 5. 验证

- `make client-check`
- `make client-smoke`
- 临时预览场景（不提交）：并排渲染 112×146 与 84×92 的 2/4/6/7 星卡、时代象形、图鉴详情、页面背景；windowed Godot 截图到 `output/`，按实际显示尺寸目视确认。
- 生成器复现：连续跑两次，`shasum` 一致。

## 6. 风险

- `_draw()` 新增子节点可能影响 UI 烟雾测试断言：只新增子节点、不改变布局尺寸，失败时按测试语义调整。
- 攻防底牌把 Label 包进新控件可能改变信息区高度：底牌尺寸严格等于原 Label 行高，`add_info` 的高度测量逻辑同步微调。
- 背景从纯色换成纹理后影响文字对比度：径向提亮控制在既定范围，预览场景检查。
- `add_corner_badges` 签名变化影响 5 个调用点：一并修改，不留兼容分支（开发阶段不为旧接口保留兼容）。
