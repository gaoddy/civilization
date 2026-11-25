# 应用图标出图记录

> 成品：`client/icon.png`（1024×1024、RGB、无 alpha）
> 画面基线与验收：见[图片素材索引](image-asset-index.md) §2.5
> 中间稿不随源提交；本节保留每次出图的提示词与任务号，供重出或改版追溯。

## 1. 出图方式

- 平台：EvoLink
- 模型：`doubao-seedream-5.0-pro`（方向探索）、`gemini-3-pro-image-preview`（Nano Banana Pro，文本重绘与参考图编辑）、`gpt-image-2.5-flare`（参考图编辑，定稿链路）
- 流程：先用三个方向各出一图定方向，再逐步修正环体、象形数量与中心纹样；定稿链路都是以上一版成品为参考图做编辑
- 参数：Seedream `1024×1024` / `quality=2K` / `png` / 不透明 / 无水印；Nano Banana Pro `size=1:1` / `quality=2K`；GPT Image 2.5 `1024×1024` / `quality=high` / `png` / 不透明
- 收尾：成品按 §2.5 直接覆盖 `client/icon.png`，不做压缩、不裁圆角

## 2. 版本记录

| 版本 | 模型 | 任务 ID | 结果 |
| --- | --- | --- | --- |
| 方向 A 纪元之塔 | Seedream 5.0 Pro | `task-unified-1795604483-qr05aa37` | 七层材质成立；配色偏棕白灰、象形密度低，未采用 |
| 方向 B 星盘战棋 | Seedream 5.0 Pro | `task-unified-1795604483-xt9tf85j` | 质感最好；黄铜偏棕、黄道符号过多，作为后续基础 |
| 方向 C 破晓之旗 | Seedream 5.0 Pro | `task-unified-1795604483-144zlkrc` | 背景被画成白色，违反深色底，未采用 |
| v2 星盘去符号 | Nano Banana Pro | `task-unified-1795605379-6ntkv825` | 去黄道符号与星座线、金核高亮；旗杆悬空 |
| v3 旗杆入轨 | Nano Banana Pro | `task-unified-1795607781-mpokwdno` | 旗杆插入轨道；旗面变暗红 |
| v4 外环七象形 | Nano Banana Pro | `task-unified-1795607966-pqoz1m2m` | 实画 9 枚、齿轮重复 3 次 |
| v5 七象形强约束 | Nano Banana Pro | `task-unified-1795608101-6h51uxci` | 恰好 7 枚，但内环断成两段弧 |
| v6 内环闭合 | Nano Banana Pro | `task-unified-1795608207-00b868fm` | 内环闭合，但城堡重复 |
| v7 金唱片核心 | Nano Banana Pro | `task-unified-1795608287-n6a8dme8` | 唱片元素齐全；原子重复，共 8 枚 |
| v8a 基于 v3 编辑 | Nano Banana Pro | `task-unified-1795608384-19xsypo2` | 内容策略拦截，未扣费 |
| v8b 基于 v3 编辑（改词） | Nano Banana Pro | `task-unified-1795608477-ns4zm81i` | 约 11 枚象形、大量重复，未采用 |
| v9 GPT 编辑 | GPT Image 2.5 | `task-unified-1795608649-kdjvlwwu` | 恰好 7 枚、内环闭合、唱片完整；象形贴纸感 |
| v10 加宽与唱片补齐 | GPT Image 2.5 | `task-unified-1795608890-1paggw91` | 象形收进环带、内环 3D 旋转、唱片元素补齐；内环断开 |
| v11 外环正圆 | GPT Image 2.5 | `task-unified-1795609018-3xesl5o4` | 环体正确，但中心退回发光球 |
| v12 基于 v10 修环 | GPT Image 2.5 | `task-unified-1795609105-a38sk0ee` | 保住唱片并修正外环；内环压在外环上 |
| v13 内环收纳 | GPT Image 2.5 | `task-unified-1795609233-k57khqhk` | 内环完全收进外环 |
| v14 象形徽章化 | GPT Image 2.5 | `task-unified-1795609449-sz02kzif` | 内凹圆徽章 + 双色刻线，设计感提升 |
| v15 正圆扁平徽章 | GPT Image 2.5 | `task-unified-1795609655-ldnz3ztl` | 徽章改正圆、去雕刻；保留圆圈描边 |
| v16 去圆圈加描边 | GPT Image 2.5 | `task-unified-1795609836-8lm8wc7y` | 去掉徽章圆圈，象形直接落在环带并加细描边，**定稿** |

## 3. 提示词原文

### 3.1 方向探索（Seedream 5.0 Pro）

**方向 A｜纪元之塔**

```text
Mobile game app icon, full-bleed square, no text, no letters, no numbers, no mockup, no device frame. A single centered vertical obelisk tower, 72% of canvas height and 26% width, base at 88% height. The tower is built from seven stacked horizontal strata, bottom to top: rough flint stone, charred earth with faint ember glints, white marble with fluted column lines, crenellated brick, riveted cast iron, steel and glass lattice, luminous crystal. Each stratum uses one distinct material color and one crisp highlight edge, progressively more refined toward the top. On the very top a small golden sun disc with short rays. Deep ink-black radial background from #070A0F center to #121925 edge. Gold appears only at the top sun and crystal layer. Bold readable silhouette, crisp vector edges, subtle metallic shading, no glow bloom, no photorealism.
```

**方向 B｜星盘战棋**

```text
Mobile game app icon, full-bleed square, no text, no letters, no numbers, no mockup, no device frame. A brass orrery astrolabe filling the canvas: two concentric orbital rings with the outer diameter at 84% of the canvas, both rings broken by a visible gap and tilted on a diagonal. At the exact center a solid glowing gold core, like an atom nucleus or a sun. On the lower-left orbit a small dark chess pawn; on the upper-right orbit a tiny swallowtail banner. Black enamel background #070A0F, engraved brass linework in #F0C66B and #B98A2F, the gold core is the only bright highlight. Balanced composition with diagonal tension, crisp vector engraving style, subtle metallic sheen, no glow bloom, no photorealism.
```

**方向 C｜破晓之旗**

```text
Mobile game app icon, full-bleed square, no text, no letters, no numbers, no mockup, no device frame. One large wind-filled flag sweeping diagonally across the canvas from lower-left 15% to upper-right 85%, the cloth built from three flat geometric color blocks: deep gold, bright gold, ink black. The negative space inside the flag forms a half-risen sun disc, 34% of canvas diameter, offset to the right of center. A short thick gold pole at the lower left and a thin horizon line at the bottom. Flat three-tone vector, extremely thick simple shapes, crisp clean edges, no gradients, no fine detail, no glow, no photorealism. Maximum silhouette clarity at tiny sizes.
```

### 3.2 文本重绘（Nano Banana Pro）

**v2｜星盘去符号**

```text
Mobile game app icon, full-bleed square, no text, no letters, no zodiac symbols, no constellation lines, no mockup, no device frame. A polished gold orrery astrolabe filling the canvas: two concentric orbital rings with the outer diameter at 88% of the canvas, each ring broken by one clean gap, tilted along a diagonal. Ring surfaces are smooth bright gold #F0C66B with darker gold #B98A2F edges and only simple engraved tick marks, no symbols or figures. At the exact center a solid luminous gold core, like an atom nucleus or a sun, the single brightest element. On the lower-left ring a clearly visible dark chess pawn silhouette, about 12% of canvas height; on the upper-right ring a small but readable swallowtail banner. Background deep ink black #070A0F with a faint radial lift toward the center. Clean vector shading, crisp edges, strong contrast, minimal detail, no glow bloom, no photorealism.
```

**v3｜旗杆入轨**

```text
Mobile game app icon, full-bleed square, no text, no letters, no zodiac symbols, no constellation lines, no mockup, no device frame. A polished gold orrery astrolabe filling the canvas: two concentric orbital rings with the outer diameter at 88% of the canvas, each ring broken by one clean gap, tilted along a diagonal. Ring surfaces are smooth bright gold #F0C66B with darker gold #B98A2F edges and only simple engraved tick marks. At the exact center a solid luminous gold core, like an atom nucleus or a sun, the single brightest element. On the lower-left ring a clearly visible dark chess pawn silhouette, about 12% of canvas height. At the upper right, a swallowtail banner mounted on a slim gold pole whose base is visibly rooted in and connected to the upper-right ring; the banner cloth flies outward, is clearly readable, and nothing floats detached from the structure. Background deep ink black #070A0F with a faint radial lift toward the center. Clean vector shading, crisp edges, strong contrast, minimal detail, no glow bloom, no photorealism.
```

**v4｜外环七象形**

```text
Mobile game app icon, full-bleed square, no text, no letters, no numbers, no zodiac symbols, no constellation lines, no mockup, no device frame. A polished gold orrery astrolabe filling the canvas: two concentric orbital rings with the outer diameter at 88% of the canvas, each ring broken by one clean gap, tilted along a diagonal. Along the outer ring, seven small engraved era symbols in clockwise order, each centered in its own segment between tick marks: a flint axe, a campfire, a Greek temple facade, a castle with battlements, a gear, an atom with orbit, a rocket. All seven are simple gold line glyphs, uniform size, evenly spaced, clearly separated, same gold as the ring, no labels. At the exact center a solid luminous gold core, like an atom nucleus or a sun, the single brightest element. On the lower-left ring a dark chess pawn silhouette about 12% of canvas height. At the upper right a swallowtail banner on a slim gold pole rooted in the ring, the banner cloth flying outward in bright gold #F0C66B with darker gold #B98A2F shade. Background deep ink black #070A0F with a faint radial lift toward the center. Clean vector shading, crisp edges, strong contrast, minimal detail, no glow bloom, no photorealism.
```

**v5｜七象形强约束**

```text
Mobile game app icon, full-bleed square, no text, no letters, no numbers, no zodiac symbols, no constellation lines, no mockup, no device frame. A polished gold orrery astrolabe filling the canvas: two concentric orbital rings with the outer diameter at 88% of the canvas, each ring broken by one clean gap, tilted along a diagonal. On the outer ring there are EXACTLY SEVEN small engraved era glyphs and no more, each glyph appearing exactly once, evenly spaced clockwise from the top: 1 flint axe, 2 campfire, 3 Greek temple facade, 4 castle with battlements, 5 a single gear, 6 atom with orbit, 7 rocket. Do not repeat any symbol, do not draw a second or third gear, no duplicated glyphs, no additional icons. All seven are simple dark-gold line glyphs of the same size, centered in their own segment between tick marks. At the exact center a solid luminous gold core, like an atom nucleus or a sun, the single brightest element. On the lower-left ring a dark chess pawn silhouette about 12% of canvas height. At the upper right a swallowtail banner on a slim gold pole rooted in the ring, the banner cloth flying outward in bright gold #F0C66B with darker gold #B98A2F shade. Background deep ink black #070A0F with a faint radial lift toward the center. Clean vector shading, crisp edges, strong contrast, minimal detail, no glow bloom, no photorealism.
```

**v6｜内环闭合**

```text
Mobile game app icon, full-bleed square, no text, no letters, no numbers, no zodiac symbols, no mockup, no device frame. A polished gold orrery astrolabe filling the canvas: TWO concentric rings, outer diameter 88% of canvas, both rings tilted slightly along a diagonal. The outer ring is a wide gold band carrying EXACTLY SEVEN small engraved era glyphs, no more, each appearing exactly once, evenly spaced clockwise from the top: flint axe, campfire, Greek temple facade, castle with battlements, one single gear, atom with orbit, rocket. The inner ring is a COMPLETE UNBROKEN CLOSED CIRCLE, fully drawn all the way around, no gap, no break, no missing arc, a smooth thin gold band clearly separated from the outer band. At the exact center inside the inner ring, a solid luminous gold core, the single brightest element. On the inner ring at the lower left, a dark chess pawn silhouette about 12% of canvas height. At the upper right, a swallowtail banner on a slim gold pole rooted in the outer ring, banner cloth flying outward in bright gold #F0C66B with darker gold #B98A2F shade. Background deep ink black #070A0F with a faint radial lift toward the center. Clean vector shading, crisp edges, strong contrast, minimal detail, no glow bloom, no photorealism.
```

**v7｜金唱片核心**

```text
Mobile game app icon, full-bleed square, no text, no letters, no numbers, no zodiac symbols, no mockup, no device frame. A polished gold orrery astrolabe filling the canvas: two concentric rings, outer diameter 88% of canvas, slightly tilted along a diagonal. The outer ring is a wide gold band carrying EXACTLY SEVEN engraved era glyphs, each appearing exactly once, evenly spaced clockwise from the top: flint axe, campfire, Greek temple facade, castle with battlements, one single gear, atom with orbit, rocket. No duplicate symbols, no second castle, no second gear, no extra icons. The inner ring is a COMPLETE UNBROKEN CLOSED CIRCLE drawn all the way around, no gap, no missing arc. At the exact center, inside the inner ring, the glowing core is a Voyager Golden Record disc: a warm luminous gold record with fine concentric grooves, engraved on its face with the Voyager pulsar map (a small central point and fourteen thin radiating lines of different lengths) and a tiny hydrogen atom symbol made of two circles joined by a short bar. The record disc is the brightest element and glows softly. On the inner ring at the lower left a dark chess pawn silhouette about 12% of canvas height. At the upper right a gold swallowtail banner on a slim gold pole rooted in the outer ring. Background deep ink black #070A0F. Clean vector shading, crisp edges, strong contrast, minimal detail, no glow bloom beyond the disc, no photorealism.
```

### 3.3 参考图编辑

**v8a｜基于 v3 编辑（被安全策略拦截，未扣费）**

参考图：v3 成品。提示词：

```text
Use the reference image as the exact base. Keep its ring geometry, gold materials, deep black background, the gold swallowtail flag on its pole at the upper right, and the dark chess pawn at the lower left unchanged. Make only these three changes: (1) close the inner ring into a complete unbroken closed circle with no gap; (2) engrave exactly seven small dark-gold era glyphs on the outer ring, one of each, evenly spaced clockwise from the top: flint axe, campfire, Greek temple facade, castle with battlements, a single gear, atom with orbit, rocket; no duplicate symbols and no extra glyphs; (3) turn the plain glowing sphere at the center into a Voyager Golden Record disc: warm gold record with fine concentric grooves, engraved with the Voyager pulsar map of thin radiating lines of different lengths and the hydrogen atom symbol of two circles joined by a bar, still the brightest element with a soft glow. No text, no letters, no numbers, no mockup, no device frame.
```

**v8b｜基于 v3 编辑（改词重试）**

参考图：v3 成品。提示词：

```text
Use the reference image as the exact base. Keep its ring geometry, gold materials, deep black background, the gold flag on its pole at the upper right, and the dark chess pawn at the lower left unchanged. Make only these three changes: (1) close the inner ring into a complete unbroken closed circle with no gap; (2) engrave exactly seven small dark-gold era glyphs on the outer ring, one of each, evenly spaced clockwise from the top: flint axe, campfire, Greek temple facade, castle with battlements, a single gear, atom with orbit, rocket; no duplicate symbols and no extra glyphs; (3) turn the plain glowing sphere at the center into an engraved golden record disc: warm gold disc with fine concentric grooves, engraved on its face with a simple star map of thin radiating lines of different lengths and a tiny two-circle atom symbol, still the brightest element with a soft glow. No text, no letters, no numbers, no mockup, no device frame.
```

**v9｜GPT Image 编辑**

参考图：v3 成品。提示词：

```text
Edit the reference image. Keep the existing ring layout, gold rings, deep black background, gold flag on its pole at the upper right, and the dark chess pawn at the lower left. Apply exactly these changes: (1) make the inner ring a complete unbroken closed circle, fully drawn all the way around with no gap; (2) place exactly seven era glyphs on the outer ring, one of each and no repeats, evenly spaced clockwise starting at the top: flint axe, campfire, Greek temple facade, castle with battlements, a single gear, atom with orbit, rocket; each glyph appears exactly once, total glyph count is seven; (3) replace the plain glowing sphere at the center with an engraved golden record disc: warm gold disc with fine concentric grooves, engraved with a simple star map of thin radiating lines of different lengths and a small two-circle atom symbol, still the brightest element with a soft glow. No text, no letters, no numbers.
```

**v10｜加宽外环与唱片补齐**

参考图：v3 成品。提示词：

```text
Edit the reference image. Keep the gold-on-black circular composition, the dark red flag on its pole at the upper right, and the dark chess pawn at the lower left. Change exactly these things: (1) Widen the outer ring band by about 30 percent so the seven glyphs sit fully inside the band with clear margins on both sides; no glyph may touch or cross either band edge. (2) Keep exactly seven glyphs, one of each and no repeats, evenly spaced clockwise from the top: flint axe, campfire, Greek temple facade, castle with battlements, a single gear, atom with orbit, rocket. Draw all seven as fine dark-gold engraved line icons of equal size, without white outline and without drop shadow. (3) The inner ring stays a complete closed circle but is rotated in three dimensions around a diagonal axis, like a tilted orbital plane, clearly reading as a ring in rotation rather than a flat concentric circle. (4) Replace the center with a richly detailed golden record disc in the classic interstellar record-cover style: fine concentric grooves across the whole disc; in the upper-left a star map of one small central dot with fourteen thin radiating lines of different lengths ending in short ticks; in the upper-right a hydrogen transition symbol of two circles joined by a short horizontal bar with a small vertical tick; in the lower-left a small record-player diagram of a circle with a pivoted tone arm and stylus; around the rim small groups of short binary-like dashes; the disc stays the brightest element with a soft glow. No text, no letters, no words, no numbers.
```

**v11｜外环正圆**

参考图：v3 成品。提示词：

```text
Edit the reference image. Keep the gold-on-black composition, the dark red flag on its pole at the upper right, the dark chess pawn at the lower left, and the detailed golden record disc at the center with its star map, hydrogen symbol, record-player diagram, binary dashes and concentric grooves. Change exactly these things: (1) The OUTER ring must be a perfect circle: perfectly round, concentric with the canvas center, uniform band width all the way around, complete with no notch, no flattening, no tilt, and nothing crossing over or cutting into it. (2) Keep exactly seven glyphs fully inside the outer band with clear margins, evenly spaced clockwise from the top: flint axe, campfire, Greek temple facade, castle with battlements, a single gear, atom with orbit, rocket. Fine dark-gold engraved line icons, no white outline, no drop shadow. (3) The INNER ring stays tilted in three dimensions like a rotating orbital plane, but must be a complete closed ellipse with no gap and no missing arc. No text, no letters, no words, no numbers.
```

**v12｜基于 v10 修环**

参考图：v10 成品。提示词：

```text
Edit the reference image. Preserve every existing element and its style exactly: the gold materials, the dark background, the dark red flag on its pole at the upper right, the dark chess pawn at the lower left, the seven engravings inside the outer band, and especially the detailed golden record disc at the center with its star map, hydrogen symbol, record-player diagram, binary dashes and concentric grooves. Change only the ring geometry: (1) make the OUTER ring a perfect circle, perfectly round, concentric with the canvas, uniform band width all the way around, complete, with nothing crossing over it or cutting into it; (2) keep the INNER ring tilted in three dimensions like a rotating orbital plane, but make it a complete closed ellipse with no gap and no missing arc. Do not redraw or simplify the center disc. No text, no letters, no numbers.
```

**v13｜内环收纳**

参考图：v12 成品。提示词：

```text
Edit the reference image. Preserve everything exactly as it is: the perfect circular outer ring, the seven engraved glyphs inside it, the dark red flag on its pole, the dark chess pawn, the detailed golden record center with star map, hydrogen symbol, record-player diagram, binary dashes and grooves, the colors and the lighting. Change only one thing: the tilted inner ellipse ring must be completely contained within the open area inside the outer ring. Its outermost edge must not touch, cross, sit on top of, or overlap the outer band anywhere. Scale the inner ellipse down slightly if needed, keep it a complete closed tilted ellipse with the same thickness and the same 3D rotation, and keep it concentric with the center. No other changes. No text, no letters, no numbers.
```

**v14｜象形徽章化**

参考图：v13 成品。提示词：

```text
Edit the reference image. Keep the whole composition identical: the perfect circular outer ring, the tilted closed inner ellipse inside it, the detailed golden record center with star map, hydrogen symbol, record-player diagram, binary dashes and grooves, the dark red flag, the dark chess pawn, the black background and the lighting. Redesign ONLY the treatment of the seven era glyphs on the outer band, keeping exactly seven glyphs with the same meanings in the same clockwise order (flint axe, campfire, Greek temple facade, castle with battlements, single gear, atom with orbit, rocket): each glyph is now engraved in relief inside its own shallow circular medallion recessed into the gold band, with a thin beveled ring around it and a subtle inner shadow; the glyph strokes are two-tone, a dark engraved core with a thin bright gold highlight edge; between neighboring medallions place a slim ornamental double-tick separator; add a fine engraved border line following both edges of the outer band. Keep the medal design restrained, symmetrical and premium, not cartoonish. No text, no letters, no numbers.
```

**v15｜正圆扁平徽章**

参考图：v14 成品。提示词：

```text
Edit the reference image. Keep the whole composition identical: the perfect circular outer ring, the tilted closed inner ellipse, the detailed golden record center with star map, hydrogen symbol, record-player diagram, binary dashes and grooves, the dark red flag, the dark chess pawn, the black background and the lighting. Change ONLY the seven glyph badges on the outer band: (1) each badge must be a perfect circle of identical diameter, not an oval and not a capsule; (2) remove all engraving and relief styling: no bevel, no chamfer, no inner shadow, no embossed highlight, no recessed medal look; the badge surface is flat and clean; (3) each badge has only a single thin circular outline of even weight; (4) each glyph is a simple flat line icon in one uniform dark-gold color with even stroke weight, no two-tone, no highlight edge, no shadow; keep exactly seven badges with the same meanings and the same clockwise order: flint axe, campfire, Greek temple facade, castle with battlements, single gear, atom with orbit, rocket. No text, no letters, no numbers.
```

**v16｜去圆圈加描边（定稿）**

参考图：v15 成品。提示词：

```text
Edit the reference image. Keep the whole composition identical: the perfect circular outer ring, the tilted closed inner ellipse, the detailed golden record center with star map, hydrogen symbol, record-player diagram, binary dashes and grooves, the dark red flag, the dark chess pawn, the black background and the lighting. Change ONLY the seven era glyphs on the outer band: (1) remove the circular badge outlines around each glyph completely, so there are no circles or medallions on the band; (2) the glyphs now sit directly on the gold band, each drawn as a clean flat line icon with a thin dark contrasting stroke that outlines and separates the shape from the gold surface, even stroke weight; (3) keep exactly seven glyphs, same meanings and same clockwise order: flint axe, campfire, Greek temple facade, castle with battlements, single gear, atom with orbit, rocket; (4) keep the double-tick separators and the thin border lines along both edges of the band. No text, no letters, no numbers.
```

## 4. 重出注意

- 时代象形必须恰好七枚、各出现一次并按顺时针排列；生成模型容易多画或重复，出图后逐枚核对。
- 外环须为正圆且闭合；内环为倾斜闭合椭圆并完全收在外环以内。
- 出图平台对受版权或品牌保护的对象名称会触发内容策略拦截；提示词用几何描述，不点名具体作品（v8a 即因此被拦且未扣费）。
- 中间稿不提交；需要重出时以上一版成品为参考图，保持同一套「外环象形 + 内环轨道 + 唱片核心」结构。
