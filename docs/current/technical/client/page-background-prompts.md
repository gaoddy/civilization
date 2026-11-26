# 页面背景出图记录

> 状态：已实现
> 范围：9 个页面的外层氛围图与内容区结构框架图的提示词、参数、后处理与已知偏差。素材规格与使用位置见[客户端图片素材索引](image-asset-index.md) §2.1.1，槽位与取用入口见[纹理登记册](textures.md) §1.1、§1.2。

## 1. 出图参数

| 项 | 取值 |
| --- | --- |
| 服务 | EvoLink CLI，模型 `doubao-seedream-5.5` |
| 外层氛围图 | `size=1:1`、`quality=2K`，出图后缩放到 512×512 |
| 结构框架图 | `size=16:9`、`quality=2K`，出图后缩放到 1280×720；大厅与对局的手机档位缩放到 960×540 |
| 出图日期 | 2026-11-27 |
| 单张成本 | 约 $0.028 |

AI 出图不可确定性复现，成品 PNG 即真源。重出时按下列提示词原文提交，再按 §4 筛选与对齐。

## 2. 外层氛围背景

模板（`{hint}` 按页面替换）：

```text
Seamless ambient texture, square, outermost background layer of a dark strategy game. Deep indigo navy vellum (#070d18 to #0e1c30) with very faint watercolor staining and fine aged-paper grain. Only tiny scattered dim star specks and extremely subtle constellation hairlines at very low contrast, evenly distributed across the ENTIRE surface. Theme hint, very faint: {hint}. Absolutely no compass rose, no border, no frame, no emblem, no large object, no horizon, no directional structure, no corner decorations, no bright spots, no focal point, no vignette. Uniform brightness and detail everywhere so any crop from any position looks the same. No text, no letters, no symbols.
```

| 页面 | `{hint}` |
| --- | --- |
| 登录 | `a single morning star low above the mist` |
| 大厅 | `the North Star` |
| 房间等待区 | `the Orion constellation` |
| 对局 | `the Sirius star with a small hunting dog constellation` |
| 结算 | `the Libra scales constellation` |
| 卡牌图鉴 | `the Lyra constellation` |
| 排行榜 | `the Orion belt` |
| 个人主页 | `the Cassiopeia constellation` |
| 账号 | `the Southern Cross` |

## 3. 结构框架

模板（`{layout}` 按页面替换）：

```text
Flat page background for a dark strategy game menu, exact 16:9 composition. Deep indigo navy nautical chart paper (#0a1524 to #122a42), faint engraved latitude and longitude grid across the whole surface, thin muted cyan and antique gold hairlines, subtle watercolor staining, aged paper texture. LAYOUT, strictly observed: {layout} A few tiny constellation dots and small engraved compass motifs tucked into empty corners. Very low contrast everywhere, muted, quiet. NO text, NO letters, NO numbers, NO characters, NO creatures, NO UI buttons, NO icons, NO bright highlights, NO glowing center, NO vignette. The artwork must read as a quiet structural frame behind dense UI.
```

| 页面 | `{layout}` |
| --- | --- |
| 登录 | `one vertical framed panel occupying the central 34 percent of the width and 50 percent of the height, perfectly centred; one thin ornamental horizontal rule across the top 6 percent of the height; the rest stays empty star chart.` |
| 大厅 | `one large empty framed area occupying the left 66 percent of the width and the full height, with a thin double gold rule around it; one narrow vertical column on the right 30 percent of the width, divided by thin gold rules into four stacked rectangles of unequal height; one thin horizontal band across the bottom 7 percent of the height, separated by a single gold rule.` |
| 房间等待区 | `a full-frame double gold border inset 1.5 percent from the edges; inside it one horizontal band across the top 8 percent of the height; a grid of eight equal rectangles, four columns by two rows, occupying the middle 60 percent of the height; one horizontal band across the bottom 11 percent of the height.` |
| 结算 | `one vertical framed panel occupying the central 60 percent of the width and 86 percent of the height, perfectly centred; three thin horizontal rules divide its interior into a short header, a large middle area and a short footer.` |
| 卡牌图鉴 | `one horizontal band across the top 6 percent of the height, separated by a single gold rule; the entire remaining area below is one large open framed region.` |
| 排行榜 | `one horizontal band across the top 6 percent of the height; below it a thin status strip of 3 percent of the height; then one large open framed region down to the bottom.` |
| 个人主页 | `one horizontal band across the top 6 percent of the height; below it a thin strip of 3 percent; then one large open framed region down to the bottom.` |
| 账号 | `one horizontal band across the top 6 percent of the height; below it a tall centred form column occupying the central 62 percent of the width, with faint horizontal guide rules every 12 percent of its height; empty margins on both sides.` |
| 对局（桌面） | 见 §3.1，提示词为独立全文，不套用模板抬头与尾句 |
| 对局（手机） | 见 §3.1，960×540 同构重排，独立出图 |

大厅手机档位（960×540）单独出图，`{layout}` 为：

```text
no outer margin, the frame runs to the edges; one large empty framed area occupying the left 70 percent of the width from the very top to 90 percent of the height; one vertical column on the right 30 percent of the width, divided by thin gold rules into four stacked rectangles; one thin horizontal band across the bottom 8 percent of the height spanning the full width.
```

### 3.1 对局页结构框架（桌面 / 手机）

对局页分区最多，提示词单独成文：抬头由 `a dark strategy game menu` 改为 `a dark strategy game board`，保留 `LAYOUT, strictly observed` 与其后的禁止项，尾句 `The artwork must read as a quiet structural frame behind dense UI.` 省略。两张同批提交（合并报价 2 张，合计 $0.032 / 2.176 积分）。

桌面（`bg_game_desktop.png`，1280×720）：

```text
Flat page background for a dark strategy game board, exact 16:9 composition. Deep indigo navy nautical chart paper (#0a1524 to #122a42), faint engraved latitude and longitude grid across the whole surface, thin muted cyan and antique gold hairlines, subtle watercolor staining, aged paper texture. LAYOUT, strictly observed: a narrow vertical strip on the far left from 1.5 to 8 percent of the width, running from 2 to 97 percent of the height, framed by a thin double gold rule and carrying a column of small circular medallions; a wide central column from 9 to 99 percent of the width, split by thin gold rules into five stacked bands: a tall upper band from 2 to 34 percent of the height, a narrow strip from 35 to 41 percent, a framed arena band from 42 to 62 percent, a short band from 64 to 75 percent, and a deep bottom band from 76 to 99 percent; a slim utility bar across the top from 1.5 to 5 percent of the height with a small octagonal emblem at its centre. A few tiny constellation dots and small compass motifs in empty corners. Very low contrast, muted, quiet. NO text, NO letters, NO numbers, NO characters, NO creatures, NO UI buttons, NO icons, NO bright highlights, NO glowing center, NO vignette.
```

手机（`bg_game_phone.png`，960×540）：

```text
Flat page background for a dark strategy game board, exact 16:9 composition. Deep indigo navy nautical chart paper (#0a1524 to #122a42), faint engraved latitude and longitude grid across the whole surface, thin muted cyan and antique gold hairlines, subtle watercolor staining, aged paper texture. LAYOUT, strictly observed: a slim utility bar across the top from 1 to 7 percent of the height; a narrow vertical strip on the far left from 0.5 to 10 percent of the width, running from 8 to 99 percent of the height, framed by a thin double gold rule with a column of small circular medallions; a wide central column from 10.5 to 99 percent of the width, split by thin gold rules into five stacked bands: a tall upper band from 9 to 41 percent of the height, a narrow strip from 42 to 47 percent, a framed arena band from 48 to 67 percent, a short band from 68 to 82 percent, and a bottom hand band from 83 to 99 percent. A few tiny constellation dots and small compass motifs in empty corners. Very low contrast, muted, quiet. NO text, NO letters, NO numbers, NO characters, NO creatures, NO UI buttons, NO icons, NO bright highlights, NO glowing center, NO vignette.
```

## 4. 后处理与筛选

1. 下载原图后用 `sips -z <高> <宽>` 缩放到目标尺寸，不裁切。
2. 外层图逐张检查：无主体、无边框、无罗盘、四角亮度一致；出现边缘主体时重出。
3. 内层图与 `make ui-wireframe PAGE=<页面>` 的实测矩形对照，偏差超过约 2% 时重出或在提示词中调整比例；用 `make ui-preview PAGE=<页面>` 看真实窗口。
4. 出图后执行 `make asset-compress` 与 `make client-import`，`.import`/`.md5` 由压缩工具生成，不手改。

## 5. 已知偏差

- 排行榜内层图多出一条约 15% 高度处的装饰横线，落在列表区上部；不影响操作，但重出时应收紧「top 6 percent + 3 percent」的表述。
- 卡牌图鉴、账号的内层图在底角有小型装饰框，属可接受的点缀。
- 大厅手机档位与对局页按设备档位单独取图；其余页面在 960×540 下由 1280×720 等比缩放，分区比例偏差约 1% 以内，未单独出图。
- 对局桌面图（1280×720）：顶栏带实际画到 2.1–8.4%，工具条只有 1.7–5.3%，多出的一条金线落在商店区上部；河流带下沿在 40.3%（应为 41.4%）；棋盘带 42.0–61.3% 与 42.2–62.5% 基本对齐；英雄带内多出两条装饰线（63.6–68.5% 徽记横带、71.6% 单线），但 63.6% 与 76.4% 两条边界线与 HeroRow、HandStrip 对齐；左栏内边线在 8.7%（应为 8.1%）。棋盘带内另有约 10.5–71% 宽的装饰框，落在棋盘左三分之二。
- 对局手机图（960×540）：顶栏 1.5–6.3% 对齐良好；商店区没有 9.4% 顶线与 40.9% 底线，只与顶栏分开，商店靠自身面板成区；河流带整体下移约 1pp，棋盘带下沿高约 1.8pp，英雄带起线在 69.7%（应为 67.6%，为全图最大偏差），手牌带上沿高约 1.2pp；左缘竖栏只有 1.2–4.4% 宽，窄于 0.6–9.8% 的座位栏，徽记列会被不透明座位栏盖住，中央列从约 11% 起。
