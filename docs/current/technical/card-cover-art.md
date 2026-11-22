# 卡片封面美术生成

> 状态：已实现
> 范围：本文记录已落盘卡面素材的生成风格、prompt 模板、主题符号映射与后处理约束；不描述 UI 如何取图与回退（见 [Godot 客户端](client.md) §10），也不描述批量处理的脚本实现（见 `scripts/process_assets.py`）。

## 1. 风格基线

卡片封面采用「深墨描金扁平插画 + 科学史纹样」风格，对应英文 prompt 描述 `stylized flat vector illustration`。该风格由首批 20 张试点素材确立，23 位英雄补齐时沿用，当前 43 项 `card_face` 素材风格一致。

| 要素 | 取值 | prompt 对应片段 |
| --- | --- | --- |
| 构图 | 正面半身、居中 | `half-body front-facing centered composition` |
| 底色 | 近黑深蓝 | `Deep navy-black background` |
| 主体 | 平涂色块、边缘清晰、轻微渐变 | `Clean flat color blocks with subtle gradient shading, crisp edges` |
| 描边 | 黄铜金几何线稿 | `delicate brass-gold geometric linework as decorative motifs` |
| 背景纹样 | 与人物成就相关的低对比度科学史符号 | `faint ...` |
| 光效 | 柔和轮廓光 | `cinematic soft rim light` |
| 质感 | 精致克制的游戏角色插画 | `elegant refined painterly vector style` |

## 2. Prompt 模板

每张立绘的 prompt 由固定头、可变槽与固定尾组成，23 位英雄的生成全部使用同一模板，仅替换人物与主题符号。

```text
Game character portrait, stylized flat vector illustration, half-body front-facing centered composition. Subject: {人物外貌}, {时代服装}. Deep navy-black background with faint {主题符号}, delicate brass-gold geometric linework as decorative motifs. Clean flat color blocks with subtle gradient shading, crisp edges, elegant refined painterly vector style, cinematic soft rim light. No text, no letters, no watermark, no border.
```

可变槽取值规则：

1. `{人物外貌}`、`{时代服装}`：以该人物卡牌文档 `## 出处` 段的时代、身份与代表成就为据，外貌作合理想象，服装符合其时代背景。
2. `{主题符号}`：取下表对应条目，用英文写入 `faint` 之后；可含 2–3 个符号，用逗号连接。
3. 固定尾 `No text, no letters, no watermark, no border.` 不省略。

生成参数固定为 `size: 1024x1536`、`quality: high`、`background: opaque`。

## 3. 主题符号映射

下表是 23 位已生成英雄在 prompt 中使用的主题符号原文，新增同类人物卡时按下表口径取词。

| 卡牌 id | 主题符号（prompt 原文） |
| --- | --- |
| `marie-curie` | glowing radium-green atomic orbital rings, small glass vials emitting an ethereal green glow |
| `adam-smith` | golden scales of justice, dividing hands motifs |
| `beethoven` | golden musical staves, piano keys, a conductor's baton silhouette |
| `darwin` | golden finch silhouettes, a branching evolutionary tree diagram, a tortoise outline |
| `davinci` | golden anatomical sketches, a flying machine contraption, a Vitruvian circle diagram |
| `faraday` | glowing golden electromagnetic field lines, a coil and magnet apparatus, lightning arcs |
| `fleming` | greenish-gold petri dishes showing a mould ring, bacteria shapes, a penicillin molecule diagram |
| `galileo` | golden telescope outline, Jupiter and its moons orbital diagram, star charts |
| `gauss` | golden number theory symbols, modular arithmetic circles, a normal distribution bell curve outline |
| `lamarck` | golden giraffe silhouette, a transforming lineage diagram of species, leaf motifs |
| `lavoisier` | golden balance scales, glass retorts and beakers, a flame diagram |
| `linnaeus` | golden pressed flower specimens, a branching classification tree diagram, a binomial naming label frame |
| `luban` | golden carpenter's tools, a mechanical wooden contraption, a saw and compass |
| `luxun` | golden open book, a writing brush, silhouettes of figures from a crowd |
| `mendel` | golden pea plant pods, a Punnett square grid diagram, dominant and recessive flower motifs |
| `morgan` | golden fruit flies, a chromosome X-shape diagram, a red-eyed fly motif |
| `quyuan` | golden orchid and lotus flowers, flowing ribbon-like cloud patterns, a scroll motif |
| `rutherford` | glowing golden atomic nuclear model, electrons orbiting a nucleus, alpha particle scattering tracks |
| `shenkuo` | golden movable type blocks, a magnetic compass needle, a bamboo scroll, mountains and a petroleum lamp motif |
| `simaqian` | golden bamboo slip scrolls, a writing brush, stacked history book volumes |
| `tesla` | glowing electric-blue and golden Tesla coil arcs, lightning bolts, a circular alternating current symbol |
| `wright-brothers` | golden early biplane silhouette, a propeller, wing aerodynamic curves and wind lines |
| `zhangzhongjing` | golden medicinal herb sprigs, a classic medical scroll, a yin-yang balance symbol and a bronze medicine mortar |

## 4. 后处理约束

1. 生成器在立绘右下角强制添加「AI生成 / WORKBUDDY」水印；prompt 中的 `no watermark` 不能阻止，`background: opaque` 与 `transparent` 对此同样无效。
2. `scripts/process_assets.py` 的 `WM_BOX`（`886, 1452, 1016, 1535`）与采样行 `SAMPLE_Y = 1446` 是按首批素材设定的固定值。新批次水印的实际位置可能更靠右下并延伸到图片底边，执行 `make assets` 前须核对输出图右下角，确认无残留后才会派生卡面。
3. 卡面封面由立绘中心裁切到 112:146 比例后缩放到 448×584，头像由面部区域裁切到 256×256；两者均从去水印后的立绘派生，不单独生成。
4. 产物路径与清单登记见 [Godot 客户端](client.md) §10；改图后运行 `make client-import`。

## 相关文档

- [Godot 客户端](client.md) §10 美术素材层：素材路径、加载、展示位与回退
- [设计总纲](../product/overview/design-overview.md) §9：卡牌美术风格只约定科学史气质、不做奇幻
- [卡面容器](../product/ui/components/card-panel.md)：卡面在 UI 中的封面与信息区结构
