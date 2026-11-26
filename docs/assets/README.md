# 素材原始文件

> 用途：美术与 UI 设计参考用的刻线矢量源，供卡面与界面「单线平涂版画」方向取材。素材不进入客户端运行时，也不在纹理登记册中占槽位。

## 目录

| 目录 | 内容 | 许可 |
| --- | --- | --- |
| [golden-record/](golden-record/) | 旅行者金唱片封面刻线的矢量重绘，7 个图组 + 整盘 | CC BY-SA 4.0 |
| [pioneer-plaque/](pioneer-plaque/) | 先锋号金属板（Pioneer plaque）五个子元素的清理线稿与原始文件 | 公有领域 |

两套素材都是白线透明底的 SVG，可直接在设计软件或 Godot 中着色、缩放；`preview/` 下是按深底（`#0a0d12`）合成的浏览用 PNG，只看效果，不作为素材使用。

## 金唱片 `golden-record/`

| 文件 | 内容 | viewBox |
| --- | --- | --- |
| `00_full_cover.svg` | 整盘全部刻线（含盘缘与中心孔） | `115.857 269.842 487.367 484.512` |
| `01_playback_ring.svg` | 唱片播放示意（圆盘、中心孔、唱针臂） | `181.016 348.714 123.150 124.575` |
| `02_waveform.svg` | 方波、音频波形与二进制刻度 | `349.412 350.561 182.526 59.363` |
| `03_bit_zigzag.svg` | 带圆点的扫描触发折线 | `405.440 417.736 127.451 52.277` |
| `04_raster_decode.svg` | 栅格矩形与首帧图像 | `404.333 467.546 70.089 119.458` |
| `05_pulsar_map.svg` | 脉冲星地图 | `178.996 530.162 295.382 200.157` |
| `06_atom_symbols.svg` | 氢原子跃迁的两个相连圆环 | `413.947 673.862 64.900 27.192` |
| `07_playback_side.svg` | 侧视播放示意 | `184.125 498.491 119.470 49.304` |
| `source_voyager_plaque_transparent.svg` | 矢量来源原件 | `106 261 505 500` |
| `preview/<同名>_preview.png` | 各文件的深底预览 | — |

- 线条统一为 `stroke:#ffffff`、`fill:none`，改 `style` 里的这一项即可换色；`stroke-width:0.7` 是视图单位，缩放时等比。
- 每个文件带 `<title>` 与 `<desc>`，`<desc>` 已写入来源与许可。

### 许可义务（CC BY-SA 4.0）

矢量源为 Wikimedia Commons 的 `Voyager_plaque_transparent.svg`（Theklan，CC BY-SA 4.0，参考 NASA 金唱片封面原图；NASA 原图为公有领域）。随游戏分发这套素材时：

1. **署名**：在游戏致谢页或素材说明列出文件页、作者 Theklan、许可链接，并写明修改（裁切、转白线、去盘缘）。
2. **相同方式共享**：这套衍生素材本身按 CC BY-SA 4.0 提供，不能改为保留所有权利。
3. **不加额外限制**：不得用 DRM 或条款阻止他人提取、再分发这些素材。
4. 游戏整体通常不因此被传染（作为独立素材聚合）；避免与素材深度融合到无法分离。
5. 想完全免除义务：基于 NASA 公有领域原图自行重绘一套。

## 先锋板 `pioneer-plaque/`

| 文件 | 内容 | viewBox |
| --- | --- | --- |
| `clean/00_full_plaque.svg` | 整块先锋板（五个子元素齐全） | `69.78 22.71 2968.05 2321.48` |
| `clean/01_hydrogen_atom.svg` | 氢原子超精细跃迁（两个圆环与刻度） | `48.61 17.22 619.85 198.11` |
| `clean/02_pulsar_map.svg` | 脉冲星地图（太阳居中，14 条射线带二进制刻度） | `49.46 54.58 1800.54 1929.15` |
| `clean/03_humans.svg` | 男女裸体人像 | `3.89 55.26 737.35 915.38` |
| `clean/04_solar_system.svg` | 太阳系与飞船轨迹 | `62.58 -0.95 2229.98 416.99` |
| `clean/05_spacecraft.svg` | 先锋号飞船轮廓 | `86.88 38.91 1369.60 1822.23` |
| `original/*.svg` | Wikimedia 原始文件（7 个，含米色底与灰色上下文层） | 各自 width/height |
| `preview/<同名>_preview.png` | `clean/` 各文件的深底预览 | — |

- 清理版只留黑色本体层：源文件中 `stroke-opacity`/`fill-opacity` 约 0.25 的灰色层是相邻图元与细节，米色 `#fdeee1`、`#e0b794` 是底板，均已去掉；黑线转白线，viewBox 按内容边界收紧。
- `Pioneer_plaque_sun.svg` 画的是脉冲星地图：太阳在中心，14 条射线表示脉冲星方向；文件名里的「sun」指中心位置。
- `05_spacecraft` 只有机身轮廓。源文件的黑色层本身只有轮廓，碟形天线细节与轨迹刻度在灰色上下文层；需要完整飞船请用 `00_full_plaque`。
- `02_pulsar_map` 在 x=1850 处截断视口，把贯穿板面的银河系中心方向长线收成出框射线。
- 各源文件使用各自的局部坐标系，**不同文件的 viewBox 原点互不对应，不能直接叠加对齐**；需要组合时用 `00_full_plaque.svg` 作底，或按 viewBox 手工换算偏移。

### 许可

- NASA 先锋板由美国政府雇员创作，属公有领域（PD-USGov）。矢量重绘来自 Wikimedia Commons（作者 Mysid 与 Jossifresco，均标注公有领域），随游戏分发无署名与同协议义务。
- 建议在致谢页写一行「Pioneer plaque imagery: NASA / public domain」，属礼节而非要求。

## 使用约定

- 白线透明底素材配深色背景；`color-bg` 为 `#070a0f`。换色只改 `stroke`，不要改线宽以外的几何。
- 两套素材目前只是设计参考，未接入 `client/assets/`；真正进入界面时按[纹理登记册](../current/technical/client/textures.md)登记槽位，并遵守[卡面美术方向](../decisions/0002-card-art-direction.md)。
- 缩放后线宽按视图单位等比变化；需要固定像素线宽时在目标端换算 `stroke-width`。

## 生成方式

- 金唱片：解析 `source_voyager_plaque_transparent.svg`，按图元包围盒把刻线分到 7 个图组，盘缘圆环与中心孔只保留在整盘文件，描边转白，viewBox 取内容加 1.5 单位留白。
- 先锋板：过滤累计 `opacity` 低于 0.5 的子树、米色底板与半透明上下文层，黑线转白，viewBox 取内容加 20 单位留白，`02_pulsar_map` 额外限制视口上限。
- 预览：SVG 渲染为 1000px 宽，再用 `#0a0d12` 合成深底。
- 生成脚本未入库，留存在本地 `output/scratch/`（缓存目录，可随时删除）；需要重出时按上述规则重写，或直接改现有 SVG 的颜色与 viewBox。
