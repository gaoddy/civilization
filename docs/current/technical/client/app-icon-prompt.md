# 应用图标出图记录

> 成品：`client/icon.png`（1024×1024、RGB、无 alpha）
> 画面基线与验收：见[图片素材索引](image-asset-index.md) §2.5

## 1. 出图方式

- 平台与模型：EvoLink，`gpt-image-2.5`（模型 ID `gpt-image-2.5-flare`）
- 方式：参考图编辑（image-to-image），在上一版成品上按需求逐步调整；中间稿不随源提交
- 最终任务：`task-unified-1795609836-8lm8wc7y`，输出 1024×1024 PNG
- 收尾：成品按 §2.5 覆盖 `client/icon.png`，不做压缩，不裁剪圆角

## 2. 最终提示词

```text
Edit the reference image. Keep the whole composition identical: the perfect circular outer ring, the tilted closed inner ellipse, the detailed golden record center with star map, hydrogen symbol, record-player diagram, binary dashes and grooves, the dark red flag, the dark chess pawn, the black background and the lighting.
Change ONLY the seven era glyphs on the outer band:
(1) remove the circular badge outlines around each glyph completely, so there are no circles or medallions on the band;
(2) the glyphs now sit directly on the gold band, each drawn as a clean flat line icon with a thin dark contrasting stroke that outlines and separates the shape from the gold surface, even stroke weight;
(3) keep exactly seven glyphs, same meanings and same clockwise order: flint axe, campfire, Greek temple facade, castle with battlements, single gear, atom with orbit, rocket;
(4) keep the double-tick separators and the thin border lines along both edges of the band.
No text, no letters, no numbers.
```

## 3. 重出注意

- 时代象形必须恰好七枚、各出现一次并按顺时针排列；生成模型容易多画或重复，出图后逐枚核对。
- 外环须为正圆且闭合；内环为倾斜闭合椭圆并完全收在外环以内。
- 出图平台对受版权或品牌保护的对象名称会触发内容策略拦截，提示词用几何描述，不点名具体作品。
