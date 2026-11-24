# 第三方与来源说明

## `bc7-standard.wgsl`

本仓库阶段 0 原型 `tools/asset-compress/docs/phase0/bc7_mode1.wgsl` 的移植与整理，
版权归本项目所有，按 MIT 发布。

其中的 BC7（BPTC）位布局、分区表（PART2）与锚点表（ANCH1）来自 Microsoft
DirectX 规范的 BPTC 章节（DFS 20.1、表 111/116/118）。实现思路参考：

- bc7enc（MIT）：分区搜索与端点拟合的顺序；
- bcn-decode（MIT，仅作测试期解码器使用）：锚点规则与模式字段布局的对照实现。

## 依赖

| crate | 用途 | 许可 |
| --- | --- | --- |
| `astcenc-rs` / astcenc | CPU ASTC 4×4 编码与测试期解码 | Unlicense |
| `intel_tex_2`（ISPC Texture Compressor） | CPU BC7（`exact` 档） | MIT/Apache-2.0 |
| `wgpu` / `naga` | GPU BC7 执行层（Metal） | MIT/Apache-2.0 |
| `image` | PNG 解码 | MIT/Apache-2.0 |
| `bcn-decode` | 测试期 BC7 解码与 PSNR 统计（dev-dependency） | MIT |
