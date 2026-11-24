# 阶段 0 基线复现

日期：2026-11-23
机器：Apple Silicon（Metal / wgpu 30）
样本：`client/assets/cards/faces` 前 60 张，448×584，不透明
度量：解码为 RGBA8 后对源 PNG 计算 PSNR（含 alpha 通道）

| 方案 | BC7 | ASTC 4x4 |
| --- | --- | --- |
| Godot 4.7.2 现有产物 | 43.38 dB（min 39.41） | 44.54 dB（min 39.97） |
| gputex GPU（wgpu/Metal） | 42.11 dB（min 37.11） | 41.35 dB（min 36.29） |
| CPU：ISPC alpha_basic / astcenc PRE_MEDIUM | 45.23 dB（min 41.00） | 44.54 dB（min 39.97） |

与设计文档第 6 节一致（ASTC GPU 41.33 → 41.35，0.02dB 抖动）。CPU ASTC 与 Godot 完全一致，测量方法可信。

阶段 0 门槛：ASTC GPU 2 分区后 mean ≥ 42.8 dB，单张 < 15ms。
