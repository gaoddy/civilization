# ASTC 2 分区原型与质量归因（阶段 0）

日期：2026-11-23
样本：`client/assets/cards/faces` 前 60 张（448×584，不透明），度量同 BASELINE.md。

## 1. 规格确认（来自 astcenc 源码）

- **分区模式是程序化生成的**，没有 1024 项静态表：`hash52(seed)` + `select_partition`（`astcenc_partition_tables.cpp`）。2 分区时 `seed = index + 1024`；4×4 块 16 texel < 32 → `small_block=true`，x/y 先左移 1 位；`a`、`b` 取 `& 0x3F` 后比较，`a < b` 归入分区 1。
- **2 分区块布局**（`astcenc_symbolic_physical.cpp`）：
  `bits[10:0]` 块模式；`bits[12:11]` = 分区数−1；`bits[21:13]` 分区索引（10 位）；`bits[28:23]` encoded_type 低 6 位；encoded_type 高 2 位位于 `128 − weight_bits − 2`；端点 ISE 从 bit 29 开始；权重从 bit 127 向下。
- **端点预算**：12 个整数（CEM 8 × 2）时 `color_bits = 99 − weight_bits − 2`，对应 `quant_mode_table` 第 6 行：4-bit 权重 → QUANT_16，3-bit → QUANT_32，2-bit → QUANT_96。双分区把端点精度砍半，这正是原型里质量下降的原因。
- 单分区（对照）：`color_bits = 111 − weight_bits`，4-bit 权重 → QUANT_192，与 gputex 的 0x242 一致。

## 2. 实测（60 张，CPU 等价模拟器）

模拟器已用「单分区 192/4b」校准：模拟 40.66 dB vs GPU 实测 41.35 dB，偏低 0.7 dB，作为保守估计。

| 配置 | mean | min |
| --- | --- | --- |
| 理想线（端点 256 级 + 权重 65 级，不合法，纯上界） | 41.50 | 36.12 |
| 单分区 256/4b（非法组合，测端点量化影响） | 40.69 | 35.47 |
| 单分区 192/4b（GPU 现状格式） | 40.66 | 35.46 |
| 双分区 C：QUANT_32 + 3-bit 权重 | 41.14 | 37.70 |
| 双分区 A：QUANT_96 + 2-bit 权重 | 39.68 | 34.84 |
| 双分区 B：QUANT_16 + 4-bit 权重 | 37.44 | 35.86 |
| 抽取权重网格 4×3 + 4-bit（单分区，6 张） | 34.17 | 30.95 |

结论：单分区模型的理论上限约 41.5 dB；双分区在受限布局下没有超过它（最好 41.14）；抽取网格明显更差。

## 3. 逐块归因（2 张图，33 264 块）

用 astcenc `fastest` 编码、解码后逐块对比：

| astcenc 实际选的分区数 | 块数 | astcenc | 我们单分区 |
| --- | --- | --- | --- |
| 1 分区 | 27 825（85.1%） | 46.61 dB | **47.30 dB** |
| 2 分区 | 4 867（14.9%） | **39.50 dB** | 37.03 dB |

即：容易的块我们的线拟合反而更好；差距全部来自约 15% 的难点块，astcenc 用分区 + 块配置搜索（权重网格/精度/分区/CEM）在那里赢 2.5 dB。medium 的分区使用率升到 22.9%，并出现 3 分区。

## 4. astcenc preset 曲线（60 张，单线程）

| preset | mean | 每张 |
| --- | --- | --- |
| fastest | 43.48 dB | 53 ms |
| fast | 44.00 dB | 87 ms |
| medium | **44.54 dB（= Godot）** | 305 ms |

`fast` 比 Godot 低 0.54 dB，超出 −0.3 dB 门限；`standard` 只能用 `medium`。

## 5. GPU 预算

现 shader BC7 纯 GPU 计算 0.9 ms/图。2 分区至少要对 32 个候选模式做逐分区拟合（≈32×）再加端点/权重搜索 → ≥29 ms/图，超过 15 ms 闸门；若要复刻 astcenc 的配置搜索（网格×精度×分区，≥10×）→ ≥300 ms/图，且重量级拟合后质量估计仍只有约 42 dB。

## 6. 判定

按设计文档 9.3 的三分支：

- 最好质量约 41.5 dB（含校准约 42.2），**低于 42.8 dB 门槛**；
- 达到该质量所需的搜索量 **超过 25 ms/图**。

→ **回退方案 A**：`standard`/`exact` 用 CPU `astcenc`（`medium`，与 Godot 逐项一致），GPU 只保留 `fast` 档（BC7 42.11 / ASTC 41.35）用于预览等非正式场景。
