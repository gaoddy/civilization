# UI 几何资产交付清单

> 范围：卡框 `cards/frames`、稀有度徽章 `badges/rarity`、UI 底材 `ui/{bg,button,icon,panel,pill,border}`
> 生成器：`scripts/gen_ui_assets.py`（可复现，色值取自 `client/ui/theme.gd`）
> 主题：深色单一主题，无浅色变体
> 视觉语言：深墨描金 + 科学仪器纹样（金属渐变描边、罗盘取景角、中轴刻度、多面切割宝石）
> 接入状态：**未接入**（代码已还原到接入前；资产与生成器保留备用）
> 生成日期：2026-11-21

## 1. 交付物

| 目录 | 文件 | 数量 | 尺寸 | 说明 |
| --- | --- | --- | --- | --- |
| `cards/frames/` | `frame_{common,rare,epic,legendary}.png` | 4 | 448×584 | 卡框叠加层，中心透明，与 `card_*_face.png` 同尺寸对齐；多层金属渐变描边 + 四角罗盘取景角 + 中轴刻度，四档配色 |
| `badges/rarity/` | `{common,rare,epic,legendary}.png` | 4 | 64×64 | 四面切割宝石徽章，金属外框 + 中心高光，居中锚点 |
| `badges/rarity/` | `atlas.png` + `atlas.json` | 1+1 | 256×256 POT | 徽章图集，4 帧坐标与 pivot(0.5,0.5) |
| `ui/bg/` | `bg_main.png` | 1 | 256×256 | 全局深色底纹，不透明，可平铺 |
| `ui/bg/` | `bg_panel.png` | 1 | 64×64 | 面板底衬，九宫格友好 |
| `ui/button/` | `button_{primary,secondary,danger,ghost}_{normal,hover,pressed,focus,disabled}.png` | 20 | 96×40 | 四变体 × 五状态，渐变 + 高光 + 内阴影；`default/micro/slot` 归一到 secondary 视觉 |
| `ui/icon/` | `icon_slot_{slot,active,disabled}.png` | 3 | 64×64 | 图标底槽，同心圆 + 高光弧，激活态金环 |
| `ui/panel/` | `panel_{normal,raised,sunken,accent,success,danger}.png` | 6 | 64×64 | 渐变面板 + 角铆钉，九宫格友好 |
| `ui/pill/` | `pill_{neutral,accent,success,danger,info,plain}.png` | 6 | 96×28 | 胶囊标签底，六档语义色，九宫格友好 |
| `ui/border/` | `tint_{panel,button}.png` | 2 | 64×64 / 96×40 | 可染色描边（内近黑/边近白），配合 `modulate_color` 保留动态着色 |

合计 48 张 PNG，全部 RGBA 直通透明（Straight Alpha），均由 Godot `--headless --import` 生成 `.import`，uid 唯一。

## 2. 命名与色值对齐

- 稀有度分档与 `client/autoload/card_db.gd` 的 `RARITY` 常量一致：level 1–2 = common，3–4 = rare，5–6 = epic，7 = legendary。
- 卡框尺寸 448×584 = 1x 卡面 112×146 的 4 倍，与 `scripts/process_assets.py` 的 `CARD_SIZE` 一致。
- 所有色值取自 `theme.gd` 的 `PRIMITIVE_COLOR`；九宫格边距从主题令牌换算。

## 3. 接入状态与待接入点

当前 UI 代码已还原到接入前状态：面板/按钮/胶囊仍由 `StyleBoxFlat` 程序化绘制，卡框/徽章/背景资产**尚未被任何代码引用**。以下为待接入点清单（供后续接入时参考，尚未实施）：

| 资产 | 建议接入位置 | 方式 |
| --- | --- | --- |
| 面板/按钮/胶囊底材 | `client/ui/theme.gd` | `panel_style`/`button_style`/`pill_style` 返回九宫格 `StyleBoxTexture`；`transparent`/`bare`/`cover-info` 保留 `StyleBoxFlat`（结构性无底材，避免破坏布局契约） |
| 动态着色 | `theme.gd` 新增 `tinted_panel`/`tinted_button` | 6 处依赖 `border_color` 动态改色（选中金边、玩法色边框、激活蓝高亮）无法直接换纹理；用本批 `ui/border/tint_*.png`（内近黑/边近白）+ `modulate_color` 染色保留该能力 |
| 卡框 | `client/scenes/card_face_base.gd` | 按 `CardDB.rarity_tier()` 叠 `frame_<rarity>.png` 到封面之上、信息区之下 |
| 稀有度徽章 | `client/scenes/gallery_detail.gd` | 图鉴详情预览右下角叠宝石徽章 |
| 页面背景 | `client/ui/ui.gd` 新增 `UI.background()` | 5 个页面（game/lobby/room/result/card_gallery）铺 `bg_main` 平铺纹理，缺图回退主题色 |

接入时的已知 Godot API 注意点：`StyleBoxTexture` 九宫格切分属性是 `texture_margin_*`（无 `set_patch_margin_all`）；`as Texture2D` 参与布尔表达式需加括号；改共享层会牵动 40+ 消费者，须同步处理 6 处动态着色与 2 处测试断言。

## 4. 引擎导入配置

| 项 | 值 |
| --- | --- |
| 纹理类型 | `CompressedTexture2D`（Godot 默认导入，`.import` 已生成） |
| Alpha Is Transparency | `true`（直通透明） |
| Premultiply Alpha | `false` |
| Filter Mode | `Bilinear`（几何/矢量风格，非像素画） |
| Mipmaps | 关闭 |

九宫格边距（代码中从令牌换算）：panel/pill 切分 14px，button 切分 12px。图集帧坐标见 `client/assets/badges/rarity/atlas.json`。

## 5. 验证结果

- 资产层：48 张 PNG 全部 RGBA 可读；Godot `--headless --import` 生成 48 个 `.import`、uid 唯一、无脚本错误。
- 视觉层：用合成预览图检查过卡框描边、宝石切割、面板层次、按钮状态，无毛边与渲染异常。
- 代码层：接入改动已撤销，代码回到接入前状态，需以 `make client-smoke` / `make client-check` 重新确认基线。

未验证：headless 环境无渲染视口，无法截图做像素级视觉验收。

## 6. 复现

```bash
python3 scripts/gen_ui_assets.py
/Applications/Godot.app/Contents/MacOS/Godot --headless --path client --import
```
