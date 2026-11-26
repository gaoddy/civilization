# 卡面美术采用单线平涂版画与七时代方言

## 决策

卡面（随从、效果牌、英雄立绘与头像）统一采用「单线平涂版画」语法：一条结构线加最多两级平涂、单一左上柔光，禁用交叉排线；七个时代各用一套方言，方言只改背景载体纹样、载体色域与装饰线色，不改构图与渲染方式；装饰线色取现有 `color-era-*`，主体用博物图谱固有色；存量 432 张不批量重出，新增与重画卡按本指引执行。存量素材尚未按本方向重画，现有生成基线仍见[卡片封面美术生成](../current/technical/client/card-cover-art.md)。

原因是现有 432 张卡面在扁平矢量与写实厚涂之间漂移：生成模板同时要求 `stylized flat vector illustration` 与 `elegant refined painterly vector style`，两个词指向相反的渲染方式。而卡面在游戏内最小只有 78×102，可见主体带最紧 78×42，排线与多层渐变在该尺寸下会糊成灰块。时代已由框体与顶栏角标识别，画面方言只做冗余加强；单主题深色要求卡面底色统一为 `#070a0f`。

排除的替代方案：暗黑写实厚涂（小尺寸不可读、成本高、432 张批产不一致）；二次元 Q 版（偏离科学史气质，与[设计总纲](../current/product/overview/design-overview.md) §9「不做奇幻」冲突）；纯信息图与图标化（丢掉标本式叙事，且与详情面板重复）；铜版画排线执行（78px 下排线糊成灰块，prompt 无法稳定控制线密度）；按时代给整张卡换底色（破坏玩法色封面与深色 UI 的统一）。

## 执行指引

### 1. 语法与四个硬参数

气质来自科学史铜版画与博物图谱：正面、留白、细线、单色主调、把对象当标本画。执行上不用铜版画的排线，改用「一条结构线 + 两级平涂」表达体积，线只描述轮廓与结构。

| 参数 | 取值 | 判定方式 | 失败信号 |
| --- | --- | --- | --- |
| 线稿浓度 | 结构线覆盖画面 15–25%；全图一种线宽，448px 下 2px；线不描主体内部明暗 | 缩到 78px，线仍连续可数 | 交叉排线、素描擦笔、线宽变化 |
| 装饰线位置 | 只允许三处：背景纹样、装饰边框、成就符号；不勾主体轮廓 | 遮住主体后装饰线仍成立 | 主体被金线描边、线上长高光 |
| 平涂比例 | 主体 70–85% 面积为平涂，最多两级明暗（固有色 + 一级暗部） | 数主体色块层数 | 三级以上渐变、皮肤高光、体积光 |
| 光影强度 | 单一方向柔光，从左上来；无环境光遮蔽堆叠、无发光溢出、无镜头光晕 | 缩到 78px 看主体明度关系是否仍成立 | 双光源、边缘光过曝、bloom |

四条在七个时代里都不变，方言只改材质、纹样与色域。

### 2. 统一 prompt 模板

固定头 + 三个可变槽 + 固定尾，其余逐字固定：

```text
Game card artwork, screen-print poster illustration with a limited flat palette, {构图}. Subject: {主体描述}. {时代方言}. Palette limited to {色域}, accent linework in {装饰线色}. Deep navy-black background #070a0f with faint {主题符号}. One line weight, flat color blocking with a single shadow step, one soft light from upper left, generous negative space around the subject. No text, no letters, no watermark, no border, no photorealism, no 3D render, no cross-hatching, no gradient mesh, no bloom, no fantasy elements.
```

槽位规则：

1. `{构图}`：随从与效果牌固定 `half-body or object, front-facing, centered, subject in the upper 40 percent of the frame`；英雄固定 `half-body front-facing centered portrait`。
2. `{主体描述}`：随从与英雄写「外貌 + 时代服装」；效果牌写物件与场景，不出现具体人物。主体用博物图谱的固有色，不套时代色。
3. `{时代方言}`：取第 3 节对应行的 prompt 片段，逐字写入。
4. `{色域}`：取第 3 节载体色域列，最多 3 个平涂色 + 底色。
5. `{装饰线色}`：取该时代 `color-era-*` 的 hex。史前与原始两色明度低（`#b4552a`、`#8f8b46`），美术里提亮 15–20% 再用，令牌不改。
6. `{主题符号}`：沿用[卡片封面美术生成](../current/technical/client/card-cover-art.md) §3 的映射表；新增卡按同一口径取 2–3 个符号。
7. 固定尾不省略。`elegant refined painterly vector style` 与 `stylized flat vector illustration` 从模板中删除，禁止回到任何 prompt。

生成参数沿用现有流程：`size 448x576`、`quality high`、`background opaque`；后处理仍按 `scripts/process_assets.py` 的水印框输出 448×584。

### 3. 七套时代方言

方言只作用于背景载体与装饰线，主体始终用博物图谱的固有色。

| 时代 | 装饰线色 | 纹样语汇 | 载体色域 | `{时代方言}` prompt 片段 |
| --- | --- | --- | --- | --- |
| 史前文明 | `#b4552a`（提亮后） | 岩壁刻痕、炭黑手印、骨器刻线、洞穴壁面 | 炭黑、赭石、骨白 | `cave-wall texture with scratched ochre marks, hand stencil silhouettes and bone-tool engravings, on a rough stone fragment` |
| 原始社会 | `#8f8b46`（提亮后） | 青铜器饕餮纹、编织几何、陶器绳纹、玉琮刻线 | 苔绿、青铜、陶土 | `bronze-age ritual motifs, woven basketry geometry and cord-marked pottery patterns, on a cast bronze fragment` |
| 古典文明 | `#c2a878` | 宣纸墨线、卷轴边框、竹简刻度、星图 | 宣纸白、墨、赭 | `ink on rice paper with a scroll border, bamboo-slip grid and constellation chart lines, on an aged paper fragment` |
| 封建社会 | `#c8a24a` | 雕版印刷、朱砂印、舆图方格、书页边栏 | 朱砂、墨、绢黄 | `woodblock print texture with a cinnabar seal, cartographic grid and book-page margin rules, on a silk fragment` |
| 工业社会 | `#8d97a6` | 蓝图网格、机械制图、齿轮剖面、专利图纸 | 钢蓝、铁灰、纸白 | `blueprint grid with technical drawing lines, a gear cross-section and patent-sheet hatching-free diagrams, on a blue-grey blueprint sheet` |
| 现代社会 | `#b9c6d2` | 杂志网点、实验室器皿、光谱条、显微刻度 | 铬白、鸽灰、仪器青 | `halftone print dots, laboratory glassware outlines and spectrum bars, on an instrument-panel fragment` |
| 未来社会 | `#5fd3e8` | 线框光、数据流线、全息层、轨道环 | 近黑、全息青、冷白 | `wireframe glow outlines, data-stream lines and holographic overlay rings, as holographic overlay layers` |

用法：

- 装饰线色即该时代的 `color-era-*`，画面与框体同色系，不新增令牌。
- 纹样以局部载体出现：一块石板、一段卷轴、一页蓝图、一层全息浮片，浮在统一的深墨底上，不铺满整幅。载体连同明暗起伏不超过画面 30%，四周露出深底，否则整张卡变浅色，与页面背景、框体、信息区脱节。
- 纹样不压主体，缩到 78px 后退成一层可辨的质感，而不是前景图案。
- 相邻时代要有明度差：史前最暗、未来最亮。商店 6 个槽位按星级抽卡、不按时代过滤，一屏最多同时出现 6 个不同时代的卡，方言的明度差必须在同一屏里成立。
- 英雄卡没有 `era` 字段，方言按人物所处时代取，以卡牌文档 `## 出处` 段为准；该英雄的立绘、卡面与头像共用同一套方言。

### 4. 与 UI 的对齐

| UI 层 | 现状 | 要求 |
| --- | --- | --- |
| 框体时代合金 | 7 色，取 `color-era-*` | 不改；美术装饰线取同一 hex |
| 顶栏时代角标 | 14px 时代图标 + 合金色，商店、手牌、图鉴与棋盘卡全部显示 | 不改；这是时代的第一识别层 |
| 封面底色 | 玩法色 `darkened(0.62)`，无图时的回退 | 不改；美术不依赖底色，也不在画里使用玩法色 |
| 星级与三连 | 星级用顶栏星点，三连用足金框 | 不改；两者都不进入画面 |
| 背景 | 页面 `color-bg` `#070a0f` | 卡面用同一底色，卡与页面不留色差 |

不新增令牌，不改主题，不改页面。实现阶段只替换 `client/assets/cards/faces/` 下的素材与 prompt 数据，不碰 `client/ui/`。

### 5. 可读性验收

1. **尺寸门**：以 448×584 为原图，双线性缩到 112×146 与 78×102；用不透明色块遮住信息区（桌面 90px 或 56px、手机 60px 或 56px），只看剩余条带，最紧的一档是手机带介绍的 78×42。
2. **题材门**：条带内 0.5 秒能说出「人 / 动物 / 物件 / 概念」中的哪一类。
3. **时代门**（组间可分，不要求单张可辨）：七个时代各抽 3 张，21 张打乱铺开并遮住角标，0.5 秒内能把同时代的卡归到一组；个别边缘卡误判可接受，整组混成一片即不合格。
4. **明度门**：78×102 下主体相邻平涂色块的明度差不低于 18%。
5. **线稿门**：78px 下结构线不断成噪点；断了就减线量，不加粗线宽。
6. **一致性门**：同一时代抽 10 张并排（2 英雄 + 5 随从 + 3 效果），线宽、平涂级数、光向必须一致；出现第二种渲染语法即整批重做。
7. **禁止项门**：写实皮肤、3D 渲染、交叉排线、渐变网格、发光溢出、文字与边框，出现任意一项即不合格。

批次节奏：每个时代先出 8 张样张（2 英雄 + 4 随从 + 2 效果），过完七道门再放量到该时代全部卡。

### 6. 实施边界

- 本指引适用于新增卡与重画卡；存量 432 张不批量重生成，单卡替换按需进行，替换时满足第 5 节。
- 效果牌 prompt 数据在 `scripts/effect_cover_prompts.json`；英雄与随从按[卡片封面美术生成](../current/technical/client/card-cover-art.md) §2 的流程生成。
- 素材落盘与验收命令：`make assets-covers`（效果牌封面）或 `make assets`（立绘与派生卡面），随后 `make client-import`；屏幕上看实际效果用 `make ui-preview PAGE=game`。
- 卡面按新方向重画并落盘后，回写[卡片封面美术生成](../current/technical/client/card-cover-art.md) §1 风格基线与 §2 prompt 模板。

## 相关文档

- [卡片封面美术生成](../current/technical/client/card-cover-art.md)：现有素材的生成基线、主题符号映射与后处理约束
- [时代与场景](../current/product/rules/eras-and-scenarios.md)：七个时代的定义与解锁关系
- [设计令牌](../current/product/ui/tokens.md)：`color-era-*` 与背景底色的取值
- [框体材质](../current/product/ui/components/frame-material.md)：时代合金框的绘制
- [对局页](../current/product/ui/pages/game.md)：卡面顶栏、信息区与棋盘卡的展示规则
